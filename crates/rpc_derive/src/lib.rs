extern crate proc_macro;
use non_persistent::non_persistent;
use persistent::persistent;
use proc_macro::TokenStream as TS;
use proc_macro2::TokenStream;
use syn::ItemImpl;

mod non_persistent;
mod persistent;

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
            Attr::Persistent => persistent(attr, org, nodes),
            Attr::NonPersistent => non_persistent(attr, org, nodes),
        }
    } else {
        panic!("using rpc on {item:?} is not supported",)
    }
}
