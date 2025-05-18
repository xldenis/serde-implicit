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
    pub tag_index: usize,
    pub has_flatten: bool,
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
pub const FLATTEN: &'static str = "flatten";

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

fn has_tag_attribute(field: &Field) -> syn::Result<bool> {
    let mut has_tag = false;
    for attr in &field.attrs {
        if attr.path().is_ident("serde_implicit") {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident(TAG) {
                    has_tag = true;
                    Ok(())
                } else if meta.path.is_ident(FLATTEN) {
                    // Allow flatten in the same pass, will be validated later
                    Ok(())
                } else {
                    Err(Error::new_spanned(
                        attr,
                        "unknown attribute, expected `tag` or `flatten`",
                    ))
                }
            })?;
        }
    }
    Ok(has_tag)
}

fn has_flatten_attribute(field: &Field) -> syn::Result<bool> {
    let mut has_flatten = false;
    for attr in &field.attrs {
        if attr.path().is_ident("serde_implicit") {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident(FLATTEN) {
                    has_flatten = true;
                    Ok(())
                } else if meta.path.is_ident(TAG) {
                    // Allow tag in the same pass, will be validated later
                    Ok(())
                } else {
                    Err(Error::new_spanned(
                        attr,
                        "unknown attribute, expected `tag` or `flatten`",
                    ))
                }
            })?;
        }
    }
    Ok(has_flatten)
}

fn parse_enum_variants(enum_variants: Punctuated<syn::Variant, Comma>) -> syn::Result<Style> {
    let mut variants = vec![];
    let mut seen_flatten = false;

    for v in enum_variants {
        let variant_ident = v.ident.clone();
        let variant = match v.fields {
            syn::Fields::Named(_) => {
                return Err(Error::new_spanned(
                    v,
                    "`serde_implicit` cannot combine struct and tuple variants",
                ));
            }
            syn::Fields::Unnamed(fields_unnamed) => {
                parse_enum_variant(variant_ident, fields_unnamed, &mut seen_flatten)?
            }
            syn::Fields::Unit => {
                return Err(Error::new_spanned(
                    v,
                    "`serde_implicit` does not handle unit variants",
                ));
            }
        };
        variants.push(variant);
    }

    Ok(Style::Tuple(variants))
}

fn parse_enum_variant(
    variant_ident: Ident,
    fields_unnamed: FieldsUnnamed,
    seen_flatten: &mut bool,
) -> syn::Result<TupleVariant> {
    // Find which field has the tag or flatten attribute
    let mut tag_index = None;
    let mut flatten_index = None;

    for (i, field) in fields_unnamed.unnamed.iter().enumerate() {
        let has_tag = has_tag_attribute(field)?;
        let has_flatten = has_flatten_attribute(field)?;

        // Validate tag and flatten are mutually exclusive
        if has_tag && has_flatten {
            return Err(Error::new_spanned(
                field,
                "field cannot have both `#[serde_implicit(tag)]` and `#[serde_implicit(flatten)]`",
            ));
        }

        if has_tag {
            if tag_index.is_some() {
                return Err(Error::new_spanned(
                    field,
                    "duplicate `#[serde_implicit(tag)]` annotations found, only one field can be tagged",
                ));
            }
            tag_index = Some(i);
        }

        if has_flatten {
            if flatten_index.is_some() {
                return Err(Error::new_spanned(
                    field,
                    "duplicate `#[serde_implicit(flatten)]` annotations found, only one field can be flattened",
                ));
            }
            flatten_index = Some(i);
        }
    }

    let has_flatten = flatten_index.is_some();

    // Validate flatten variants only have exactly 1 field
    if has_flatten && fields_unnamed.unnamed.len() != 1 {
        return Err(Error::new_spanned(
            &variant_ident,
            "flatten variant must have exactly one field",
        ));
    }

    // Validate no non-flatten variants come after flatten variants
    if !has_flatten && *seen_flatten {
        return Err(Error::new_spanned(
            &variant_ident,
            "flatten variants must appear after all non-flatten variants in the enum definition",
        ));
    }

    if has_flatten {
        *seen_flatten = true;
    }

    Ok(TupleVariant {
        ident: variant_ident,
        fields: fields_unnamed,
        tag_index: tag_index.unwrap_or(0), // Default to position 0
        has_flatten,
    })
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
