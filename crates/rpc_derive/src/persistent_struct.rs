use proc_macro::TokenStream as TS;
use syn::ItemStruct;

use crate::{create_ident, create_struct_field};

pub fn persistent_struct(mut nodes: ItemStruct) -> TS {
    let arg_name = create_ident(&format!("__{}RpcArg", nodes.ident));

    nodes.fields = match nodes.fields {
        syn::Fields::Named(mut f) => {
            f.named.push(create_struct_field(
                "__client_channels",
                "std::collections::HashMap<u64, tokio::sync::mpsc::Sender<Vec<u8>>>",
            ));
            f.named.push(create_struct_field(
                "__receiver",
                &format!("tokio::sync::mpsc::Receiver<({arg_name}, u64)>"),
            ));
            f.named.push(create_struct_field(
                "__sender",
                &format!("tokio::sync::mpsc::Sender<({arg_name}, u64)>"),
            ));
            f.named.push(create_struct_field("id", "usize"));
            syn::Fields::Named(f)
        }
        _ => panic!("persistent does not work on structs without named fields"),
    };
    quote::quote! {#nodes}.into()
}
