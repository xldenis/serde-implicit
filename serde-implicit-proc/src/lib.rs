use proc_macro::TokenStream as TS1;
use syn::{DeriveInput, parse_macro_input};

mod ast;
mod expand;

// todo: shadow serde completely?
#[proc_macro_derive(Deserialize, attributes(serde_implicit))]
pub fn derive_serialize(input: TS1) -> TS1 {
    let input = parse_macro_input!(input as DeriveInput);

    let ts = expand::expand_derive_serialize(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into();

    ts
}
