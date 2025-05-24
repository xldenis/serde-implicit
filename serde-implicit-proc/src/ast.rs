use std::collections::HashSet;

use syn::{DeriveInput, Error, Field, FieldsNamed, Generics, Ident};

pub struct Variant {
    pub ident: Ident,
    pub tag: Ident,
    pub fields: FieldsNamed,
}

pub type Fields = FieldsNamed;

pub struct Enum {
    pub ident: Ident,
    pub generics: Generics,

    pub variants: Vec<Variant>,

    pub fallthrough: Option<Fallthrough>,
}

/// A fallthrough variant for `serde-implicit`
pub struct Fallthrough {
    pub ident: Ident,
    pub field: Field,
}

pub const TAG: &'static str = "tag";

pub fn parse_data(input: DeriveInput) -> syn::Result<Enum> {
    let mut enum_ = match input.data {
        syn::Data::Enum(data_enum) => data_enum,
        _ => {
            return Err(Error::new_spanned(
                input,
                "`serde_implicit` can only `Deserialize` struct enum variants",
            ));
        }
    };

    let mut variants = vec![];

    let last_var = enum_.variants.pop();

    for v in enum_.variants {
        let variant = parse_variant(v)?;
        variants.push(variant);
    }

    let mut fallthrough = None;

    if let Some(var) = last_var {
        let var_or_fall = parse_variant_or_fallthrough(&var.into_value(), true)?;

        match var_or_fall {
            VarOrFall::Var(var) => variants.push(var),
            VarOrFall::Fall(fall) => fallthrough = Some(fall),
        }
    }
    let mut unique_tags = HashSet::new();

    for v in &variants {
        if !unique_tags.insert(v.tag.clone()) {
            return Err(Error::new_spanned(v.tag.clone(), "duplicate tags found"));
        }
    }

    Ok(Enum {
        ident: input.ident,
        generics: input.generics,
        fallthrough,
        variants,
    })
}

enum VarOrFall {
    Var(Variant),
    Fall(Fallthrough),
}

fn parse_variant_or_fallthrough(v: &syn::Variant, can_fallthrough: bool) -> syn::Result<VarOrFall> {
    let named = match &v.fields {
        syn::Fields::Named(named) => named,
        syn::Fields::Unit | syn::Fields::Unnamed(_) => {
            return Err(Error::new_spanned(
                v,
                "`serde_implicit` can only `Deserialize` struct enum variants",
            ));
        }
    };

    // Find all fields with #[serde_implicit(tag)] attribute
    let mut tagged_fields = vec![];

    for field in &named.named {
        let mut has_tag = false;
        field
            .attrs
            .iter()
            .filter(|a| a.path().is_ident("serde_implicit"))
            .try_for_each(|attr| {
                attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident(TAG) {
                        has_tag = true;
                        Ok(())
                    } else {
                        Err(Error::new_spanned(attr, "omg"))
                    }
                })
            })?;

        if has_tag {
            tagged_fields.push(field);
        }
    }

    let tag;
    match tagged_fields.len() {
        0 => {
            if !can_fallthrough {
                return Err(Error::new_spanned(
                    named,
                    "missing `#[serde_implicit(tag)]`",
                ));
            };

            if named.named.len() != 1 {
                return Err(Error::new_spanned(
                    v,
                    "fallthrough must have exactly one field",
                ));
            }

            return Ok(VarOrFall::Fall(Fallthrough {
                ident: v.ident.clone(),
                field: named.named.last().cloned().unwrap(),
            }));
        }

        1 => {
            tag = tagged_fields[0].ident.clone().unwrap();
        }
        _ => {
            return Err(Error::new_spanned(
                named,
                "duplicate `#[serde_implicit(tag)]` annotations found, only one field can be tagged",
            ));
        }
    };

    Ok(VarOrFall::Var(Variant {
        ident: v.ident.clone(),
        tag,
        fields: named.clone(),
    }))
}

fn parse_variant(v: syn::Variant) -> syn::Result<Variant> {
    match parse_variant_or_fallthrough(&v, false)? {
        VarOrFall::Var(v) => Ok(v),
        _ => unreachable!(),
    }
}
