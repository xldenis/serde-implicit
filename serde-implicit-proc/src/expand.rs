use proc_macro2::{Literal, TokenStream};
use quote::{format_ident, quote};

use crate::ast;

pub fn expand_derive_serialize(input: syn::DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let data_enum = ast::parse_data(input)?;

    let mut variant_arms = vec![];
    for (ix, var) in data_enum.variants.iter().enumerate() {
        let block = deserialize_variant(var);

        let variant = implement_variant_deserializer(var, &data_enum.ident);
        let cons = format_ident!("__variant{ix}");
        variant_arms.push(quote! {
            __Variant::#cons => {#block #variant }
        });
    }

    // if we need the 'de lifetime do same trick as serde
    let (_, ty_generics, where_clause) = data_enum.generics.split_for_impl();

    let this_type = &data_enum.ident;

    let enum_variant = generate_variant_enum(&data_enum.variants);

    let this_type_str = Literal::string(&this_type.to_string());
    Ok(quote! {
        #[automatically_derived]
        impl<'de> serde::Deserialize<'de> for #this_type #ty_generics #where_clause {
            fn deserialize<__D>(__deserializer: __D) -> Result<Self, __D::Error>
            where __D: serde::Deserializer<'de>
            {
                #enum_variant

                let (__tag, __content) = serde::Deserializer::deserialize_any(
                    __deserializer,
                    serde_implicit::__private::TaggedContentVisitor::<__Variant>::new(#this_type_str))?;
                let __deserializer = serde::__private::de::ContentDeserializer::<__D::Error>::new(__content);

                match __tag {
                    #(#variant_arms)*
                }
            }

        }
    })
}

pub fn generate_variant_enum(variants: &[ast::Variant]) -> TokenStream {
    use proc_macro2::TokenStream;
    use quote::{format_ident, quote};
    use std::str::FromStr;

    let variant_enum_variants = variants.iter().enumerate().map(|(i, _)| {
        let variant = format_ident!("__variant{}", i);
        quote! { #variant }
    });

    // Add an ignore variant for unknown tag values
    let variant_enum_variants = quote! {
        #(#variant_enum_variants,)*
    };

    let visit_str_arms = variants.iter().enumerate().map(|(i, var)| {
        let tag_value = Literal::string(&var.tag.to_string());

        let variant = format_ident!("__variant{}", i);
        quote! {
            #tag_value => serde::__private::Ok(__Variant::#variant),
        }
    });

    let visit_bytes_arms = variants.iter().enumerate().map(|(i, var)| {
        let tag_value = &var.tag;

        let byte_string = format!("b\"{}\"", tag_value);
        let byte_tokens = TokenStream::from_str(&byte_string).unwrap_or_else(|_| {
            quote! { #tag_value.as_bytes() }
        });

        let variant = format_ident!("__variant{}", i);
        quote! {
            #byte_tokens => serde::__private::Ok(__Variant::#variant),
        }
    });

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

fn deserialize_variant(var: &ast::Variant) -> TokenStream {
    let field_variants = (0..var.fields.named.len()).map(|i| {
        let variant = format_ident!("__field{}", i);
        quote! { #variant }
    });

    // todo: remove `__ignore` if `deny_unknown_fields` is set.
    let field_variants = quote! {
        #(#field_variants,)*
        __ignore,
    };

    let mut visit_str_arms = Vec::new();
    let mut visit_bytes_arms = Vec::new();

    for (i, field) in var.fields.named.iter().enumerate() {
        let field_ident = field.ident.as_ref().unwrap();
        let field_name = field_ident.to_string();
        let variant = format_ident!("__field{}", i);

        visit_str_arms.push(quote! {
            #field_name => serde::__private::Ok(__Field::#variant),
        });

        let byte_string = format!("b\"{}\"", field_name);
        let byte_tokens = Literal::byte_string(&byte_string.as_bytes());

        visit_bytes_arms.push(quote! {
            #byte_tokens => serde::__private::Ok(__Field::#variant),
        });
    }

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

fn implement_variant_deserializer(var: &ast::Variant, enum_name: &syn::Ident) -> TokenStream {
    use quote::{format_ident, quote};

    let fields = &var.fields.named;
    let variant_ident = &var.ident;
    let variant_name = format!("{}::{}", enum_name, variant_ident);
    let expecting_message = format!("struct variant {}", variant_name);

    let mut field_declarations = Vec::new();
    let mut field_processing = Vec::new();
    let mut final_fields = Vec::new();

    for (i, field) in var.fields.named.iter().enumerate() {
        let field_ident = field.ident.as_ref().unwrap();
        let field_name = field_ident.to_string();
        let field_type = &field.ty;
        let field_var = format_ident!("__field{}", i);
        let field_enum_variant = format_ident!("__field{}", i);

        field_declarations.push(quote! {
            let mut #field_var: serde::__private::Option<#field_type> = serde::__private::None;
        });

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

        final_fields.push(quote! {
            let #field_var = match #field_var {
                serde::__private::Some(#field_var) => #field_var,
                serde::__private::None => {
                    serde::__private::de::missing_field(#field_name)?
                }
            };
        });
    }

    let field_idents = fields.iter().map(|f| f.ident.as_ref().unwrap());
    let field_vars = (0..fields.len()).map(|i| format_ident!("__field{}", i));

    let struct_init = quote! {
        #enum_name::#variant_ident {
            #(#field_idents: #field_vars),*
        }
    };

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

        serde::Deserializer::deserialize_map(
            __deserializer,
            __Visitor {
                marker: serde::__private::PhantomData::<#enum_name>,
                lifetime: serde::__private::PhantomData,
            }
        )

    }
}
