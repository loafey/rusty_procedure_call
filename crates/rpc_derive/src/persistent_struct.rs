use proc_macro::TokenStream as TS;
use syn::ItemStruct;

use crate::create_struct_field;

pub fn persistent_struct(mut nodes: ItemStruct) -> TS {
    nodes.fields = match nodes.fields {
        syn::Fields::Named(mut f) => {
            f.named.push(create_struct_field(
                "__client_channels",
                "std::collections::HashMap<u64, tokio::sync::mpsc::Sender<Vec<u8>>>",
            ));
            f.named.push(create_struct_field(
                "__receiver",
                "tokio::sync::mpsc::Receiver<__MessageHandler>",
            ));
            f.named.push(create_struct_field(
                "__sender",
                "tokio::sync::mpsc::Sender<__MessageHandler>",
            ));
            f.named.push(create_struct_field("id", "usize"));
            syn::Fields::Named(f)
        }
        _ => panic!("persistent does not work on structs without named fields"),
    };
    quote::quote! {#nodes}.into()
}
