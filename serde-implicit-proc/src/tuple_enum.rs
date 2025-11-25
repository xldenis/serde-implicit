use quote::{format_ident, quote};
use syn::Ident;

use crate::ast::{self};

pub fn expand_tuple_enum(
    ty_name: &Ident,
    variants: &[ast::TupleVariant],
) -> syn::Result<proc_macro2::TokenStream> {
    // Separate variants into regular and flatten groups
    let (regular_variants, flatten_variants): (Vec<_>, Vec<_>) =
        variants.iter().partition(|v| !v.has_flatten);

    let mut variant_trials = vec![];

    // Generate trials for regular (non-flatten) variants
    for v in regular_variants.iter() {
        let variant_ident = &v.ident;
        let fields = &v.fields;
        let field_count = fields.unnamed.len();
        let tag_index = v.tag_index;

        let tag_field = fields
            .unnamed
            .iter()
            .nth(tag_index)
            .expect("tag index must be smaller than variant's field count");
        let tag_type = &tag_field.ty;

        let trial = if field_count == 1 {
            quote! {
                if let serde_implicit::__private::Content::Seq(ref __seq) = __content {
                    if __seq.len() == 1 {
                        if let ::std::result::Result::Ok(__tag) = <#tag_type as serde::Deserialize>::deserialize(
                            serde_implicit::__private::ContentRefDeserializer::<__D::Error>::new(&__seq[0])
                        ) {
                            return ::std::result::Result::Ok(#ty_name::#variant_ident(__tag));
                        }
                    }
                } else {
                    // Try to deserialize the entire content as the tag
                    if let ::std::result::Result::Ok(__tag) = <#tag_type as serde::Deserialize>::deserialize(
                        serde_implicit::__private::ContentDeserializer::<__D::Error>::new(__content.clone())
                    ) {
                        return ::std::result::Result::Ok(#ty_name::#variant_ident(__tag));
                    }
                }
            }
        } else {
            let variant_deserializer =
                implement_variant_deserializer(variant_ident, fields, ty_name);
            let tag_index_lit = proc_macro2::Literal::usize_unsuffixed(tag_index);
            let field_count_lit = proc_macro2::Literal::usize_unsuffixed(field_count);

            quote! {
                if let serde_implicit::__private::Content::Seq(ref __seq) = __content {
                    // Check length and tag, if both pass, commit to this variant
                    if __seq.len() == #field_count_lit && <#tag_type as serde::Deserialize>::deserialize(
                        serde_implicit::__private::ContentRefDeserializer::<__D::Error>::new(&__seq[#tag_index_lit])
                    ).is_ok() {
                        let __deserializer = serde_implicit::__private::ContentRefDeserializer::<__D::Error>::new(&__content);
                        return #variant_deserializer;
                    }
                }
            }
        };

        variant_trials.push(trial);
    }

    // Generate trials for flatten variants (tried only if no regular variant matched)
    let mut flatten_trials = vec![];
    for v in flatten_variants.iter() {
        let variant_ident = &v.ident;
        let fields = &v.fields;

        // Flatten variants have exactly one field
        let field = fields.unnamed.first().ok_or_else(|| {
            syn::Error::new_spanned(&v.ident, "flatten variant must have exactly one field")
        })?;
        let field_type = &field.ty;

        let trial = quote! {
            if let ::std::result::Result::Ok(__field0) = <#field_type as serde::Deserialize>::deserialize(
                serde_implicit::__private::ContentDeserializer::<__D::Error>::new(__content.clone())
            ) {
                return ::std::result::Result::Ok(#ty_name::#variant_ident(__field0));
            }
        };

        flatten_trials.push(trial);
    }

    let expected_str = proc_macro2::Literal::string(&format!("a valid variant of {}", ty_name));

    Ok(quote! {
        let __content = <serde_implicit::__private::Content as serde::Deserialize>::deserialize(
            __deserializer,
        )?;

        // Try each regular variant in order
        #(#variant_trials)*

        // If no regular variant matched, try flatten variants
        #(#flatten_trials)*

        // No variant matched
        ::std::result::Result::Err(serde::de::Error::custom(format!(
            "data did not match any variant of enum {}",
            #expected_str
        )))
    })
}

fn implement_variant_deserializer(
    variant_ident: &Ident,
    fields: &syn::FieldsUnnamed,
    enum_name: &syn::Ident,
) -> proc_macro2::TokenStream {
    let variant_name = format!("{}::{}", enum_name, variant_ident);
    let expecting_message = format!("tuple variant {}", variant_name);
    let field_count = fields.unnamed.len();

    // Generate field deserialization: __seq.next_element::<Type>()?.ok_or_else(...)?
    let field_deserializations: Vec<_> = fields
        .unnamed
        .iter()
        .enumerate()
        .map(|(i, field)| {
            let field_type = &field.ty;
            let field_var = format_ident!("__field{}", i);
            let field_index = proc_macro2::Literal::usize_unsuffixed(i);
            let error_context = format!("{}: {{}}", variant_name);

            quote! {
                let #field_var = match serde::de::SeqAccess::next_element::<#field_type>(&mut __seq)
                    .map_err(|__e| serde::de::Error::custom(format!(#error_context, __e)))?
                {
                    ::std::option::Option::Some(__value) => __value,
                    ::std::option::Option::None => {
                        return ::std::result::Result::Err(serde::de::Error::invalid_length(
                            #field_index,
                            &#expecting_message,
                        ));
                    }
                };
            }
        })
        .collect();

    let field_vars: Vec<_> = (0..field_count)
        .map(|i| format_ident!("__field{}", i))
        .collect();

    let tuple_init = quote! {
        #enum_name::#variant_ident(#(#field_vars),*)
    };

    quote! {
        {
            #[doc(hidden)]
            struct __Visitor;

            #[automatically_derived]
            impl<'de> serde::de::Visitor<'de> for __Visitor {
                type Value = #enum_name;

                fn expecting(
                    &self,
                    __formatter: &mut ::std::fmt::Formatter,
                ) -> ::std::fmt::Result {
                    ::std::fmt::Formatter::write_str(
                        __formatter,
                        #expecting_message,
                    )
                }

                #[inline]
                fn visit_seq<__A>(self, mut __seq: __A) -> ::std::result::Result<Self::Value, __A::Error>
                where
                    __A: serde::de::SeqAccess<'de>,
                {
                    #(#field_deserializations)*

                    ::std::result::Result::Ok(#tuple_init)
                }
            }

            serde::Deserializer::deserialize_seq(
                __deserializer,
                __Visitor,
            )
        }
    }
}
