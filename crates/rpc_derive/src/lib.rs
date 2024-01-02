use non_persistent::non_persistent;
use persistent::persistent;
use persistent_struct::persistent_struct;
use proc_macro::TokenStream as TS;
use proc_macro2::{Ident, TokenStream};
use syn::{
    parse_macro_input, punctuated::Punctuated, token::Comma, Field, GenericParam, ImplItem,
    ItemImpl, ItemStruct, PatType, Path, ReturnType, Type, Visibility,
};

mod non_persistent;
mod persistent;
mod persistent_struct;

fn get_generics(_ty: &PatType) -> Punctuated<GenericParam, Comma> {
    Punctuated::new()
}

type Function = (Ident, Punctuated<PatType, Comma>, Option<Type>);
struct FunctionTypes {
    _generics: Punctuated<GenericParam, Comma>,
    functions: Vec<Function>,
}
fn get_function_types(items: Vec<ImplItem>) -> FunctionTypes {
    let mut _generics = Punctuated::new();
    let mut functions = Vec::new();
    for item in items {
        if let ImplItem::Fn(func) = item {
            if !matches!(func.vis, Visibility::Public(..)) {
                continue;
            }
            let func_name = func.sig.ident;
            let mut args: Punctuated<_, Comma> = Punctuated::new();
            let mut is_self_ref = false;
            for arg in func.sig.inputs.into_iter() {
                match arg {
                    syn::FnArg::Receiver(r) => {
                        is_self_ref = r.reference.is_some();
                    }
                    syn::FnArg::Typed(t) => {
                        _generics.extend(get_generics(&t));
                        args.push(t);
                    }
                }
            }
            let ret_type = match func.sig.output {
                ReturnType::Default => None,
                ReturnType::Type(_, t) => Some(*t),
            };
            if is_self_ref {
                functions.push((func_name, args, ret_type));
            }
        }
    }
    FunctionTypes {
        _generics,
        functions,
    }
}

fn create_ident(s: &str) -> Ident {
    syn::Ident::new(s, proc_macro2::Span::call_site())
}

fn create_struct_field(name: &str, r#type: &str) -> Field {
    Field {
        attrs: Vec::new(),
        vis: Visibility::Inherited,
        mutability: syn::FieldMutability::None,
        ident: Some(create_ident(name)),
        colon_token: None,
        ty: Type::Verbatim(r#type.parse().unwrap()),
    }
}

#[derive(Clone, Default)]
enum Attr {
    Persistent,
    #[default]
    NonPersistent,
}

#[proc_macro_attribute]
pub fn rpc(attr: TS, item: TS) -> TS {
    let attr = parse_macro_input!(attr with Punctuated::<Path, Comma>::parse_terminated)
        .into_iter()
        .collect::<Vec<_>>();
    let attr = if matches!(&attr[..], [p] if format!("{}",p.get_ident().unwrap()) == "Persistent") {
        Attr::Persistent
    } else if attr.is_empty() {
        Attr::NonPersistent
    } else {
        panic!("the attribute needs to be either `Persistent`, `PersistentClient, <type>` or nothing at all")
    };
    let org = TokenStream::from(item.clone());
    if let Ok(nodes) = syn::parse::<ItemImpl>(item.clone()) {
        match attr {
            Attr::Persistent => persistent(org, nodes),
            Attr::NonPersistent => non_persistent(org, nodes),
        }
    } else if let Ok(nodes) = syn::parse::<ItemStruct>(item.clone()) {
        if matches!(attr, Attr::Persistent) {
            persistent_struct(nodes)
        } else {
            panic!("using rpc on non impl/structs is not supported",)
        }
    } else {
        panic!("using rpc on non impl/structs is not supported",)
    }
}
