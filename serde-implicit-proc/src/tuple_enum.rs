use itertools::Itertools;
use proc_macro2::{Literal, TokenStream};
use quote::{format_ident, quote};
use syn::Ident;

use crate::ast::{self};

// precondition `variants` are sorted by `tag_position`.
pub fn tuple_variant_enum(ty_name: &Ident, variants: &[ast::TupleVariant]) -> TokenStream {
    use quote::{format_ident, quote};

    let variant_enum_variants = variants.iter().enumerate().map(|(i, variant)| {
        let variant_ident = format_ident!("__variant{}", i);
        if variant.fields.unnamed.len() > 0 {
            let field = &variant.fields.unnamed[variant.tag_position];
            let field_type = &field.ty;
            quote! { #variant_ident(#field_type) }
        } else {
            quote! { #variant_ident }
        }
    });

    let variant_enum_variants = quote! {
        #(#variant_enum_variants,)*
    };

    let variants_by_group = variants
        .iter()
        .enumerate()
        .chunk_by(|(_, var)| var.tag_position);

    let deserialize_variant_arms = variants_by_group.into_iter().map(|(tag_ix, variant)| {
        let tests = variant.into_iter().map(|(i, variant)| {
            let variant_ident = format_ident!("__variant{}", i);

            let field = &variant.fields.unnamed[tag_ix];
            let field_type = &field.ty;

            quote! {
                if let serde::__private::Ok(__ok) =
                    <#field_type as serde::Deserialize>::deserialize(__deserializer) {
                    return serde::__private::Ok(__Variant::#variant_ident(__ok));
                }
            }
        });

        quote! {
            let __grp = serde_implicit::__private::nth_elem(__elements, #tag_ix)?;
            let __deserializer = serde::__private::de::ContentRefDeserializer::<E,>::new(__grp);
            #(#tests)*
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
            __elements: &serde::__private::de::Content) -> serde::__private::Result<__Variant, E> {
            #(#deserialize_variant_arms)*

            let _any = __elements;
            // If none of the variants matched
            serde::__private::Err(serde::de::Error::invalid_value(serde_implicit::__private::unexpected(&_any), &#expected_str))

        }
    }
}

pub fn expand_tuple_enum(
    ty_name: &Ident,
    variants: &[ast::TupleVariant],
) -> syn::Result<proc_macro2::TokenStream> {
    let mut variant_arms = vec![];

    for (i, v) in variants.iter().enumerate() {
        let variant_ident = format_ident!("__variant{}", i);
        let original_variant_ident = &v.ident;

        let fields = &v.fields;

        let field_count = fields.unnamed.len();

        // // Simple case for variants with just one field
        // if field_count == 1 {
        //     variant_arms.push(quote! {
        //         __Variant::#variant_ident(__value) => {

        //             if let Some(serde::__private::de::Content::Seq(s)) =  __content {
        //                 if s.len() > 0 {
        //                     return serde::__private::Err(
        //                         serde::de::Error::invalid_length(
        //                             0,
        //                             &concat!("tuple variant ", stringify!(#ty_name), "::", stringify!(#original_variant_ident), " with ", stringify!(#field_count), " elements"),
        //                         ),
        //                     );
        //                 }
        //             } else {
        //                 todo!()
        //             };

        //             Ok(#ty_name::#original_variant_ident(__value))
        //         }
        //     });
        //     continue;
        // }

        let visitor_name = format_ident!("__{}Visitor", original_variant_ident);

        let field_deserialize = fields.unnamed.iter().enumerate().map(|(j, field)| {
            let field_name = format_ident!("__field{}", j);
            let field_type = &field.ty;
            let idx = proc_macro2::Literal::usize_unsuffixed(j);

            quote! {
                let #field_name = match serde::de::SeqAccess::next_element::<#field_type>(&mut __seq)? {
                    serde::__private::Some(__value) => __value,
                    serde::__private::None => {
                        return serde::__private::Err(
                            serde::de::Error::invalid_length(
                                #idx,
                                &concat!("tuple variant ", stringify!(#ty_name), "::", stringify!(#original_variant_ident), " with ", stringify!(#field_count), " elements"),
                            ),
                        );
                    }
                };
            }
        });

        let field_names = (0..field_count).map(|j| format_ident!("__field{}", j));

        let variant_str = format!("tuple variant {}::{}", ty_name, original_variant_ident);
        // let variant_elements_str = format!("tuple variant {}::{} with {} elements",
        //     ty_name, original_variant_ident, field_count);

        // For variants with multiple fields, we use the first field from __Variant
        // and deserialize the remaining fields with a visitor
        // let field_count_remaining = field_count - 1;
        // let tag_ty = &fields.unnamed.first().as_ref().unwrap().ty;
        variant_arms.push(quote! {
            __Variant::#variant_ident(__first_field) => {
                #[doc(hidden)]
                struct #visitor_name {
                    marker: serde::__private::PhantomData<#ty_name>,
                    // first_field: #tag_ty,
                }

                impl<'de> serde::de::Visitor<'de> for #visitor_name {
                    type Value = #ty_name;

                    fn expecting(
                        &self,
                        __formatter: &mut serde::__private::Formatter,
                    ) -> serde::__private::fmt::Result {
                        serde::__private::Formatter::write_str(__formatter, #variant_str)
                    }

                    #[inline]
                    fn visit_seq<__A>(
                        self,
                        mut __seq: __A,
                    ) -> serde::__private::Result<Self::Value, __A::Error>
                    where
                        __A: serde::de::SeqAccess<'de>,
                    {
                        #(#field_deserialize)*

                        serde::__private::Ok(#ty_name::#original_variant_ident(#(#field_names),*))
                    }
                }

                let __deserializer = serde::__private::de::ContentDeserializer::<__D::Error>::new(__content);

                serde::Deserializer::deserialize_tuple(
                    __deserializer,
                    #field_count,
                    #visitor_name {
                        marker: serde::__private::PhantomData::<#ty_name>,
                        // first_field: __first_field,
                    },
                )
            }
        });
    }

    Ok(quote! {
        let __content = <serde::__private::de::Content as serde::Deserialize>::deserialize(
            __deserializer,
        )?;

        // let (tag, __content) = serde_implicit::__private::pop_front(__content)?;

        // let __deserializer = serde::__private::de::ContentRefDeserializer::<__D::Error,>::new(&tag);

        let __tag = deserialize_variant(&__content)?;

        match __tag {
            #(#variant_arms)*
        }
    })
}
