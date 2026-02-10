use proc_macro::TokenStream as TS1;
use syn::{DeriveInput, parse_macro_input};

mod ast;
mod expand;
mod tuple_enum;

/// Derive macro for implicitly tagged enum deserialization.
///
/// Annotate one field per variant with `#[serde_implicit(tag)]` to mark it as
/// the discriminant. When that key appears in the input, the deserializer
/// commits to the corresponding variant and produces targeted error messages
/// instead of serde's generic "data did not match any variant" error.
///
/// **Tag fields should be non-optional.** During deserialization, keys whose
/// value is `null` are ignored when searching for the implicit tag. If a tag
/// field is `Option<T>` and the input contains `"field": null`, that variant
/// will not be selected.
// todo: shadow serde completely?
#[proc_macro_derive(Deserialize, attributes(serde_implicit))]
pub fn derive_serialize(input: TS1) -> TS1 {
    let input = parse_macro_input!(input as DeriveInput);

    let ts = expand::expand_derive_serialize(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into();

    ts
}
