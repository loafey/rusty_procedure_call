use non_persistent::non_persistent;
use persistent::persistent;
use proc_macro::TokenStream as TS;
use proc_macro2::{Ident, TokenStream};
use syn::{
    punctuated::Punctuated, token::Comma, GenericParam, ImplItem, ItemImpl, PatType, ReturnType,
    Type, Visibility,
};

mod non_persistent;
mod persistent;

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

#[derive(Debug, Clone, Copy, Default)]
enum Attr {
    Persistent,
    #[default]
    NonPersistent,
}

#[proc_macro_attribute]
pub fn rpc(attr: TS, item: TS) -> TS {
    let attr = format!("{attr}");
    let attr = if attr == "Persistent" {
        Attr::Persistent
    } else if attr.is_empty() {
        Attr::NonPersistent
    } else {
        panic!("the attribute needs to be either `Persistent` or nothing at all")
    };
    let org = TokenStream::from(item.clone());
    if let Ok(nodes) = syn::parse::<ItemImpl>(item.clone()) {
        match attr {
            Attr::Persistent => persistent(org, nodes),
            Attr::NonPersistent => non_persistent(org, nodes),
        }
    } else {
        panic!("using rpc on {item:?} is not supported",)
    }
}
