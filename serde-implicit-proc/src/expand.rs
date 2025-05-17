use proc_macro2::{Literal, TokenStream};
use quote::{format_ident, quote};

use syn::{DeriveInput, Error, Member, punctuated::Punctuated, token::Comma};
pub const TAG: &'static str = "tag";

pub fn expand_derive_serialize(
    input: &mut syn::DeriveInput,
) -> syn::Result<proc_macro2::TokenStream> {
    let mut tags = vec![];
    let data_enum = match &input.data {
        syn::Data::Enum(data_enum) => {
            for v in &data_enum.variants {
                match &v.fields {
                    syn::Fields::Named(named) => {
                        let tag_field = named
                            .named
                            .iter()
                            .find(|x| x.attrs.iter().find(|a| a.path().is_ident(TAG)).is_some());
                        tags.push(tag_field);
                    }
                    syn::Fields::Unit | syn::Fields::Unnamed(_) => {
                        return Err(Error::new_spanned(
                            v,
                            "`serde_implicit` can only `Deserialize` struct enum variants",
                        ));
                    }
                }
            }
            data_enum
        }
        _ => {
            return Err(Error::new_spanned(
                input,
                "`serde_implicit` can only `Deserialize` struct enum variants",
            ));
        }
    };

    // let deser: serde::__private::de::TaggedContentVisitor<()> = todo!();
    // todo validate that tags are all unique

    // step 3 initial code gen: build fragments for variants

    let mut variant_arms = vec![];
    for (ix, (tag, var)) in tags.iter().zip(&data_enum.variants).enumerate() {
        let block = deserialize_variant(var);

        let variant = implement_variant_deserializer(var, &format_ident!("Omg"), 0);
        let cons = format_ident!("__variant{ix}");
        variant_arms.push(quote! {
            __Variant::#cons => {#block #variant }
        });
    }

    // if we need the 'de lifetime do same trick as serde
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let this_type = &input.ident;

    // // step 4 final code generation
    // let visitor = quote! {

    //     #[doc(hidden)]
    //     struct __Visitor <'de, #impl_generics> #where_clause {
    //         marker: std::marker::PhantomData<#this_type #ty_generics>,
    //         lifetime: std::marker::PhantomData<&'de ()>,
    //     }

    //     impl<'de, #impl_generics> serde::de::Visitor<'de, #ty_generics> for __Visitor<'de, #ty_generics> #where_clause
    //     {
    //         type Value = #this_type #ty_generics;

    //         fn visit_map<__A>(self, mut __map: __A) -> serde::Result<Self::Value, __A::Error>
    //         where
    //             __A: serde::de::MapAccess<'de>
    //         {
    //             let (tag, content) = TaggedContentVisitor::visit_map(__map)?;

    //         }
    //     }

    // };

    let enum_variant = generate_variant_enum(&data_enum.variants, &tags);

    Ok(quote! {
        #[automatically_derived]
        impl<'de> serde::Deserialize<'de> for #this_type #ty_generics #where_clause {
            fn deserialize<__D>(__deserializer: __D) -> Result<Self, __D::Error>
            where __D: serde::Deserializer<'de>
            {
                #enum_variant


                let (__tag, __content) = serde::Deserializer::deserialize_any(
                    __deserializer,
                    // put correct path here
                    serde_implicit::TaggedContentVisitor::<__Variant>::new("omg"))?;
                let __deserializer = serde::__private::de::ContentDeserializer::<__D::Error>::new(__content);

                match __tag {
                    #(#variant_arms)*
                }

                // deserialize variants
            }

        }
    })
}

/// Generates a `__Variant` enum that can be used to deserialize enum variants based on tag field values.
///
/// # Parameters
///
/// * `variants` - The variants of the enum being deserialized
/// * `tags` - A vector of optional references to fields that have been tagged with the `#[tag]` attribute
///
/// # Returns
///
/// A TokenStream containing the generated `__Variant` enum and its implementations
pub fn generate_variant_enum(
    variants: &Punctuated<syn::Variant, Comma>,
    tags: &[Option<&syn::Field>],
) -> TokenStream {
    use proc_macro2::{Literal, TokenStream};
    use quote::{format_ident, quote};
    use std::str::FromStr;
    use syn::LitStr;

    // Generate variant enum variants
    let variant_enum_variants = variants.iter().enumerate().map(|(i, _)| {
        let variant = format_ident!("__variant{}", i);
        quote! { #variant }
    });

    // Add an ignore variant for unknown tag values
    let variant_enum_variants = quote! {
        #(#variant_enum_variants,)*
    };

    // Generate match arms for visit_str based on tag values
    let visit_str_arms = variants
        .iter()
        .enumerate()
        .zip(tags)
        .filter_map(|((i, _), tag_field)| {
            // Only generate match arms for variants that have tag fields
            tag_field.map(|field| {
                // Find the tag value from field attributes
                let tag_value = field
                    .attrs
                    .iter()
                    .find(|a| a.path().is_ident(TAG))
                    .and_then(|attr| {
                        // Extract the tag value from the attribute
                        attr.parse_args::<LitStr>().ok().map(|lit| lit.value())
                    })
                    .unwrap_or_else(|| {
                        // Fallback to the field name if no explicit tag value
                        field
                            .ident
                            .as_ref()
                            .map(|id| id.to_string())
                            .unwrap_or_default()
                    });

                let variant = format_ident!("__variant{}", i);
                quote! {
                    #tag_value => serde::__private::Ok(__Variant::#variant),
                }
            })
        });

    // Generate match arms for visit_bytes using the same tag values
    let visit_bytes_arms =
        variants
            .iter()
            .enumerate()
            .zip(tags)
            .filter_map(|((i, _), tag_field)| {
                tag_field.map(|field| {
                    let tag_value = field
                        .attrs
                        .iter()
                        .find(|a| a.path().is_ident(TAG))
                        .and_then(|attr| attr.parse_args::<LitStr>().ok().map(|lit| lit.value()))
                        .unwrap_or_else(|| {
                            field
                                .ident
                                .as_ref()
                                .map(|id| id.to_string())
                                .unwrap_or_default()
                        });

                    let byte_string = format!("b\"{}\"", tag_value);
                    let byte_tokens = TokenStream::from_str(&byte_string).unwrap_or_else(|_| {
                        quote! { #tag_value.as_bytes() }
                    });

                    let variant = format_ident!("__variant{}", i);
                    quote! {
                        #byte_tokens => serde::__private::Ok(__Variant::#variant),
                    }
                })
            });

    // Generate the combined token stream
    quote! {
        #[allow(non_camel_case_types)]
        #[doc(hidden)]
        enum __Variant {
            #variant_enum_variants
        }

        #[doc(hidden)]
        struct __VariantVisitor;

        #[automatically_derived]
        impl<'de> serde::de::Visitor<'de> for __VariantVisitor {
            type Value = __Variant;

            fn expecting(
                &self,
                __formatter: &mut serde::__private::Formatter,
            ) -> serde::__private::fmt::Result {
                serde::__private::Formatter::write_str(
                    __formatter,
                    "variant tag identifier",
                )
            }

            fn visit_str<__E>(
                self,
                __value: &str,
            ) -> serde::__private::Result<Self::Value, __E>
            where
                __E: serde::de::Error,
            {
                match __value {
                    #(#visit_str_arms)*
                    _ => serde::__private::de::missing_field("omg"),
                    // _ => serde::__private::Ok(__Variant::__ignore),
                }
            }

            fn visit_bytes<__E>(
                self,
                __value: &[u8],
            ) -> serde::__private::Result<Self::Value, __E>
            where
                __E: serde::de::Error,
            {
                match __value {
                    #(#visit_bytes_arms)*
                    _ => serde::__private::de::missing_field("omg"),
                    // _ => serde::__private::Ok(__Variant::__ignore),
                }
            }
        }

        #[automatically_derived]
        impl<'de> serde::Deserialize<'de> for __Variant {
            #[inline]
            fn deserialize<__D>(
                __deserializer: __D,
            ) -> serde::__private::Result<Self, __D::Error>
            where
                __D: serde::Deserializer<'de>,
            {
                serde::Deserializer::deserialize_identifier(
                    __deserializer,
                    __VariantVisitor,
                )
            }
        }
    }
}

fn deserialize_variant(var: &syn::Variant) -> TokenStream {
    use syn::{Fields, FieldsNamed};
    let Fields::Named(FieldsNamed { named, .. }) = &var.fields else {
        unreachable!()
    };

    // Generate field enum variants
    let field_variants = (0..named.len()).map(|i| {
        let variant = format_ident!("__field{}", i);
        quote! { #variant }
    });

    // Collect field variants and add the ignore variant
    let field_variants = quote! {
        #(#field_variants,)*
        __ignore,
    };

    // Generate match arms for visit_str and visit_bytes
    let mut visit_str_arms = Vec::new();
    let mut visit_bytes_arms = Vec::new();

    for (i, field) in named.iter().enumerate() {
        let field_ident = field.ident.as_ref().unwrap(); // Safe because we already checked these are named fields
        let field_name = field_ident.to_string();
        let variant = format_ident!("__field{}", i);

        visit_str_arms.push(quote! {
            #field_name => serde::__private::Ok(__Field::#variant),
        });

        // For visit_bytes, we'll create a raw string for the byte string literal
        let byte_string = format!("b\"{}\"", field_name);
        let byte_tokens = Literal::byte_string(&byte_string.as_bytes());

        visit_bytes_arms.push(quote! {
            #byte_tokens => serde::__private::Ok(__Field::#variant),
        });
    }

    // Combine it all into the final token stream
    quote! {
        #[allow(non_camel_case_types)]
        #[doc(hidden)]
        enum __Field {
            #field_variants
        }

        #[doc(hidden)]
        struct __FieldVisitor;

        #[automatically_derived]
        impl<'de> serde::de::Visitor<'de> for __FieldVisitor {
            type Value = __Field;

            fn expecting(
                &self,
                __formatter: &mut serde::__private::Formatter,
            ) -> serde::__private::fmt::Result {
                serde::__private::Formatter::write_str(
                    __formatter,
                    "field identifier",
                )
            }

            fn visit_str<__E>(
                self,
                __value: &str,
            ) -> serde::__private::Result<Self::Value, __E>
            where
                __E: serde::de::Error,
            {
                match __value {
                    #(#visit_str_arms)*
                    _ => serde::__private::Ok(__Field::__ignore),
                }
            }

            fn visit_bytes<__E>(
                self,
                __value: &[u8],
            ) -> serde::__private::Result<Self::Value, __E>
            where
                __E: serde::de::Error,
            {
                match __value {
                    #(#visit_bytes_arms)*
                    _ => serde::__private::Ok(__Field::__ignore),
                }
            }
        }

        #[automatically_derived]
        impl<'de> serde::Deserialize<'de> for __Field {
            #[inline]
            fn deserialize<__D>(
                __deserializer: __D,
            ) -> serde::__private::Result<Self, __D::Error>
            where
                __D: serde::Deserializer<'de>,
            {
                serde::Deserializer::deserialize_identifier(
                    __deserializer,
                    __FieldVisitor,
                )
            }
        }
    }
}

/// Generates code for deserializing an enum variant using the __Field enum.
///
/// # Parameters
///
/// * `var` - The variant to generate deserialization code for
/// * `enum_name` - The name of the enum type
/// * `variant_index` - The index of this variant in the enum
///
/// # Returns
///
/// A TokenStream containing the generated visitor and deserialization code for the variant
fn implement_variant_deserializer(
    var: &syn::Variant,
    enum_name: &syn::Ident,
    variant_index: usize,
) -> TokenStream {
    use proc_macro2::TokenStream;
    use quote::{format_ident, quote};

    let variant_ident = &var.ident;
    let variant_name = format!("{}::{}", enum_name, variant_ident);
    let expecting_message = format!("struct variant {}", variant_name);

    // Get the named fields from the variant
    let fields = match &var.fields {
        syn::Fields::Named(named) => &named.named,
        _ => return quote! {}, // Already checked earlier that all variants are struct variants
    };

    // Generate field declarations and assignments for the visitor
    let mut field_declarations = Vec::new();
    let mut field_processing = Vec::new();
    let mut final_fields = Vec::new();

    for (i, field) in fields.iter().enumerate() {
        let field_ident = field.ident.as_ref().unwrap(); // Safe for named fields
        let field_name = field_ident.to_string();
        let field_type = &field.ty;
        let field_var = format_ident!("__field{}", i);
        let field_enum_variant = format_ident!("__field{}", i);

        // Field declaration
        field_declarations.push(quote! {
            let mut #field_var: serde::__private::Option<#field_type> = serde::__private::None;
        });

        // Field matching in visit_map
        field_processing.push(quote! {
            __Field::#field_enum_variant => {
                if serde::__private::Option::is_some(&#field_var) {
                    return serde::__private::Err(
                        <__A::Error as serde::de::Error>::duplicate_field(#field_name),
                    );
                }
                #field_var = serde::__private::Some(
                    serde::de::MapAccess::next_value::<#field_type>(&mut __map)?,
                );
            }
        });

        // Final field extraction with missing field handling
        final_fields.push(quote! {
            let #field_var = match #field_var {
                serde::__private::Some(#field_var) => #field_var,
                serde::__private::None => {
                    serde::__private::de::missing_field(#field_name)?
                }
            };
        });
    }

    // Build the struct initialization expression
    let field_idents = fields.iter().map(|f| f.ident.as_ref().unwrap());
    let field_vars = (0..fields.len()).map(|i| format_ident!("__field{}", i));

    let struct_init = quote! {
        #enum_name::#variant_ident {
            #(#field_idents: #field_vars),*
        }
    };

    // Generate the complete visitor
    quote! {
        #[doc(hidden)]
        struct __Visitor<'de> {
            marker: serde::__private::PhantomData<#enum_name>,
            lifetime: serde::__private::PhantomData<&'de ()>,
        }

        #[automatically_derived]
        impl<'de> serde::de::Visitor<'de> for __Visitor<'de> {
            type Value = #enum_name;

            fn expecting(
                &self,
                __formatter: &mut serde::__private::Formatter,
            ) -> serde::__private::fmt::Result {
                serde::__private::Formatter::write_str(
                    __formatter,
                    #expecting_message,
                )
            }


            #[inline]
            fn visit_map<__A>(
                self,
                mut __map: __A,
            ) -> serde::__private::Result<Self::Value, __A::Error>
            where
                __A: serde::de::MapAccess<'de>,
            {
                #(#field_declarations)*

                while let serde::__private::Some(__key) = serde::de::MapAccess::next_key::<
                    __Field,
                >(&mut __map)? {
                    match __key {
                        #(#field_processing)*
                        _ => {
                            let _ = serde::de::MapAccess::next_value::<
                                serde::de::IgnoredAny,
                            >(&mut __map)?;
                        }
                    }
                }

                #(#final_fields)*

                serde::__private::Ok(#struct_init)
            }
        }

        // How we'll use this visitor with the variant enum
        serde::Deserializer::deserialize_map(
            __deserializer,
            __Visitor {
                marker: serde::__private::PhantomData::<#enum_name>,
                lifetime: serde::__private::PhantomData,
            }
        )

    }
}

mod hacks {}
