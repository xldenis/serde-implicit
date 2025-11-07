use proc_macro2::{Literal, TokenStream};
use quote::{format_ident, quote};
use syn::Ident;

use crate::ast::{self};

#[allow(dead_code)]
pub fn tuple_variant_enum(ty_name: &Ident, variants: &[ast::TupleVariant]) -> TokenStream {
    use quote::{format_ident, quote};

    let variant_enum_variants = variants.iter().enumerate().map(|(i, variant)| {
        let variant_ident = format_ident!("__variant{}", i);

        if let Some(field) = variant.fields.unnamed.iter().next() {
            let field_type = &field.ty;
            quote! { #variant_ident(#field_type) }
        } else {
            quote! { #variant_ident }
        }
    });

    let variant_enum_variants = quote! {
        #(#variant_enum_variants,)*
    };

    let deserialize_variant_arms = variants.iter().enumerate().map(|(i, variant)| {
        let variant_ident = format_ident!("__variant{}", i);

        if let Some(field) = variant.fields.unnamed.iter().next() {
            let field_type = &field.ty;

            quote! {
                if let serde::__private::Ok(__ok) =
                    <#field_type as serde::Deserialize>::deserialize(__deserializer) {
                    return serde::__private::Ok(__Variant::#variant_ident(__ok));
                }
            }
        } else {
            quote! {}
        }
    });

    let expected_str = Literal::string(&format!("a `{ty_name}`"));
    quote! {
        #[allow(non_camel_case_types)]
        #[doc(hidden)]
        enum __Variant {
            #variant_enum_variants
        }


        fn deserialize_variant<E : serde::de::Error>(
            __deserializer: serde::__private::de::ContentRefDeserializer<'_, '_, E>) -> serde::__private::Result<__Variant, E> {
            #(#deserialize_variant_arms)*

            let _any = <serde::__private::de::Content as serde::Deserialize>::deserialize(__deserializer)?;
            // If none of the variants matched
            serde::__private::Err(serde::de::Error::invalid_value(serde_implicit::__private::unexpected(&_any), &#expected_str))

        }
    }
}

pub fn expand_tuple_enum(
    ty_name: &Ident,
    variants: &[ast::TupleVariant],
) -> syn::Result<proc_macro2::TokenStream> {
    let mut variant_trials = vec![];

    for v in variants.iter() {
        let variant_ident = &v.ident;
        let fields = &v.fields;
        let field_count = fields.unnamed.len();
        let tag_index = v.tag_index;

        // Get the tag field type
        let tag_field = fields.unnamed.iter().nth(tag_index).ok_or_else(|| {
            syn::Error::new_spanned(
                &v.ident,
                format!("tag_index {} out of bounds for variant with {} fields", tag_index, field_count),
            )
        })?;
        let tag_type = &tag_field.ty;

        // Generate field names for all fields except the tag
        let field_constructions: Vec<_> = (0..field_count)
            .map(|i| {
                if i == tag_index {
                    (i, quote! { __tag }, true)
                } else {
                    let adjusted_i = if i > tag_index { i - 1 } else { i };
                    let field_name = format_ident!("__field{}", adjusted_i);
                    (i, quote! { #field_name }, false)
                }
            })
            .collect();

        // Generate deserialization for non-tag fields with error propagation (commit on tag match)
        let field_deserializations: Vec<_> = fields.unnamed.iter().enumerate()
            .filter(|(i, _)| *i != tag_index)
            .map(|(i, field)| {
                let field_type = &field.ty;
                let seq_index = proc_macro2::Literal::usize_unsuffixed(i);
                let adjusted_i = if i > tag_index { i - 1 } else { i };
                let field_name = format_ident!("__field{}", adjusted_i);

                quote! {
                    let #field_name = <#field_type as serde::Deserialize>::deserialize(
                        serde::__private::de::ContentRefDeserializer::<__D::Error>::new(&__seq[#seq_index])
                    )?;
                }
            })
            .collect();

        // Collect field construction values
        let field_construction_values: Vec<_> = field_constructions.iter().map(|(_, val, _)| val).collect();
        let tag_index_lit = proc_macro2::Literal::usize_unsuffixed(tag_index);
        let field_count_lit = proc_macro2::Literal::usize_unsuffixed(field_count);

        // Generate trial block for this variant
        let trial = if field_count == 1 && tag_index == 0 {
            // Special case: single-field variant with tag at position 0
            quote! {
                // Try variant #variant_ident (single field)
                if let serde::__private::de::Content::Seq(ref __seq) = __content {
                    if __seq.len() == 1 {
                        if let serde::__private::Ok(__tag) = <#tag_type as serde::Deserialize>::deserialize(
                            serde::__private::de::ContentRefDeserializer::<__D::Error>::new(&__seq[0])
                        ) {
                            // Tag matched - committed to this variant
                            return serde::__private::Ok(#ty_name::#variant_ident(__tag));
                        }
                    }
                } else {
                    // Try to deserialize the entire content as the tag
                    if let serde::__private::Ok(__tag) = <#tag_type as serde::Deserialize>::deserialize(
                        serde::__private::de::ContentDeserializer::<__D::Error>::new(__content.clone())
                    ) {
                        // Tag matched - committed to this variant
                        return serde::__private::Ok(#ty_name::#variant_ident(__tag));
                    }
                }
            }
        } else {
            // Multi-field variant
            quote! {
                // Try variant #variant_ident with tag at position #tag_index
                if let serde::__private::de::Content::Seq(ref __seq) = __content {
                    if __seq.len() == #field_count_lit {
                        // Try to deserialize tag at index #tag_index
                        if let serde::__private::Ok(__tag) = <#tag_type as serde::Deserialize>::deserialize(
                            serde::__private::de::ContentRefDeserializer::<__D::Error>::new(&__seq[#tag_index_lit])
                        ) {
                            // Tag matched - committed to this variant
                            // Deserialize remaining fields (errors propagate with ?)
                            #(#field_deserializations)*

                            return serde::__private::Ok(#ty_name::#variant_ident(#(#field_construction_values),*));
                        }
                    }
                }
            }
        };

        variant_trials.push(trial);
    }

    let expected_str = proc_macro2::Literal::string(&format!("a valid variant of {}", ty_name));

    Ok(quote! {
        let __content = <serde::__private::de::Content as serde::Deserialize>::deserialize(
            __deserializer,
        )?;

        // Try each variant in order
        #(#variant_trials)*

        // No variant matched
        serde::__private::Err(serde::de::Error::custom(format!(
            "data did not match any variant of enum {}",
            #expected_str
        )))
    })
}
