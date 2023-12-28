extern crate proc_macro;
use proc_macro::TokenStream as TS;
use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{punctuated::Punctuated, GenericParam, ItemImpl, Lifetime, LifetimeParam};

#[proc_macro_attribute]
pub fn rpc(_attr: TS, item: TS) -> TS {
    let org = TokenStream::from(item.clone());
    let nodes = syn::parse::<ItemImpl>(item).unwrap();
    let mut items = quote!();
    let mut res = quote!();
    let this_type = nodes.self_ty;
    let mut generics = Vec::new();
    for item in nodes.items {
        if let syn::ImplItem::Fn(func) = item {
            let func_name = func.sig.ident;
            for arg in func.sig.inputs.into_iter() {
                match arg {
                    syn::FnArg::Receiver(r) => {
                        // if let Some((_, l)) = r.reference {
                        //     if let Some(l) = l {
                        //         generics.insert(0, GenericParam::Lifetime(LifetimeParam::new(l)));
                        //         items = quote!(#items #func_name(&#this_type),);
                        //     } else {
                        //         let life_time = GenericParam::Lifetime(LifetimeParam::new(
                        //             Lifetime::new("'__hopefully_unused", Span::call_site()),
                        //         ));
                        //         if !generics.iter().any(|l| {
                        //             matches!(
                        //                 l,
                        //                 GenericParam::Lifetime(LifetimeParam {
                        //                     lifetime: Lifetime { ident, .. },
                        //                     ..
                        //                 }) if ident == &Ident::new("__hopefully_unused", Span::call_site())
                        //             )
                        //         }) {
                        //             let id = Lifetime::new("'__hopefully_unused", Span::call_site());
                        //             generics.insert(0, life_time);
                        //             items = quote!(#items #func_name(&#id #this_type),);
                        //         }
                        //     }
                        // }
                    }
                    syn::FnArg::Typed(_) => todo!(),
                }
            }
        }
    }

    let name = syn::Ident::new(
        &format!("{}Arg", this_type.to_token_stream()),
        proc_macro2::Span::call_site(),
    );

    let items = if generics.is_empty() {
        quote!(enum #name {
            #items
        })
    } else {
        let mut punc = Punctuated::new();
        punc.extend(generics);
        let generics = syn::Generics {
            lt_token: None,
            params: punc,
            gt_token: None,
            where_clause: None,
        };
        quote!(enum #name #generics {
            #items
        })
    };

    let output = quote! {
        #org
        #items
        #res
    };

    output.into()
}
