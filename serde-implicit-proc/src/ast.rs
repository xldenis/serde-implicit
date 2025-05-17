use std::collections::HashSet;

use syn::{DeriveInput, Error, FieldsNamed, Generics, Ident};

pub struct Variant {
    pub ident: Ident,
    pub tag: Ident,
    pub fields: FieldsNamed,
}

struct Enum {
    pub ident: Ident,
    pub generics: Generics,

    pub variants: Vec<Variant>,
}

pub const TAG: &'static str = "tag";

pub fn parse_data(input: DeriveInput) -> syn::Result<Enum> {
    let enum_ = match input.data {
        syn::Data::Enum(data_enum) => data_enum,
        _ => {
            return Err(Error::new_spanned(
                input,
                "`serde_implicit` can only `Deserialize` struct enum variants",
            ));
        }
    };

    let mut variants = vec![];
    for v in enum_.variants {
        let variant = parse_variant(v)?;
        variants.push(variant);
    }

    // let mut unique_tags = HashSet::new();

    // for t in &tags {
    //     if !unique_tags.insert(t.ident.clone()) {
    //         return Err(Error::new_spanned(t, "duplicate tags found"));
    //     }
    // }

    Ok(Enum {
        ident: input.ident,
        generics: input.generics,
        variants,
    })
}

fn parse_variant(v: syn::Variant) -> syn::Result<Variant> {
    let mut tag;
    let named = match v.fields {
        syn::Fields::Named(named) => named,
        syn::Fields::Unit | syn::Fields::Unnamed(_) => {
            return Err(Error::new_spanned(
                v,
                "`serde_implicit` can only `Deserialize` struct enum variants",
            ));
        }
    };

    // Find all fields with #[tag] attribute
    let tagged_fields: Vec<_> = named
        .named
        .iter()
        .filter(|x| x.attrs.iter().any(|a| a.path().is_ident(TAG)))
        .collect();

    match tagged_fields.len() {
        0 => {
            return Err(Error::new_spanned(named, "missing `#[tag]`"));
        }
        1 => {
            tag = tagged_fields[0].ident.clone().unwrap();
        }
        _ => {
            return Err(Error::new_spanned(
                named,
                "duplicate `#[tag]` annotations found, only one field can be tagged",
            ));
        }
    };

    Ok(Variant {
        name: v.ident,
        tag,
        fields: named,
    })
}
