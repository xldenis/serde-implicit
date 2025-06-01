use std::collections::HashSet;

use syn::{
    DeriveInput, Error, Field, FieldsNamed, FieldsUnnamed, Generics, Ident, punctuated::Punctuated,
    token::Comma,
};

pub struct Variant {
    pub ident: Ident,
    pub tag: Ident,
    pub fields: FieldsNamed,
}

pub struct TupleVariant {
    pub ident: Ident,
    pub fields: FieldsUnnamed,
}

pub type Fields = FieldsNamed;

pub struct Enum {
    pub ident: Ident,
    pub generics: Generics,

    pub vars: Style,
}

pub enum Style {
    Tuple(Vec<TupleVariant>),
    Struct {
        variants: Vec<Variant>,
        fallthrough: Option<Fallthrough>,
    },
}

/// A fallthrough variant for `serde-implicit`
pub struct Fallthrough {
    pub ident: Ident,
    pub field: Field,
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

    let variants = match enum_.variants.first().map(|v| &v.fields) {
        Some(syn::Fields::Named(_)) => parse_struct_variants(enum_.variants)?,
        Some(syn::Fields::Unnamed(_)) => parse_enum_variants(enum_.variants)?,
        Some(syn::Fields::Unit) => todo!(),
        None => Style::Tuple(vec![]),
    };

    Ok(Enum {
        ident: input.ident,
        generics: input.generics,
        vars: variants,
    })
}

enum VarOrFall {
    Var(Variant),
    Fall(Fallthrough),
}

fn parse_struct_variants(mut enum_variants: Punctuated<syn::Variant, Comma>) -> syn::Result<Style> {
    let mut variants = vec![];

    let last_var = enum_variants.pop();

    for v in enum_variants {
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

    Ok(Style::Struct {
        fallthrough,
        variants,
    })
}

fn parse_enum_variants(enum_variants: Punctuated<syn::Variant, Comma>) -> syn::Result<Style> {
    let mut variants = vec![];

    for v in enum_variants {
        let variant = match v.fields {
            syn::Fields::Named(_) => return Err(Error::new_spanned(v, "blah")),
            syn::Fields::Unnamed(fields_unnamed) => TupleVariant {
                ident: v.ident,
                fields: fields_unnamed,
            },
            syn::Fields::Unit => return Err(Error::new_spanned(v, "blah")),
        };
        variants.push(variant);
    }

    Ok(Style::Tuple(variants))
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
