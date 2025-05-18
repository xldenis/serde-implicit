use proc_macro::TokenStream as TS1;
use syn::{DeriveInput, parse_macro_input};

mod ast;
mod expand;
mod tuple_enum;

// todo: shadow serde completely?
#[proc_macro_derive(Deserialize, attributes(serde_implicit))]
pub fn derive_serialize(input: TS1) -> TS1 {
    let input = parse_macro_input!(input as DeriveInput);

    let ts = tuple_enum::expand_tuple_enum(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into();

    ts
}
