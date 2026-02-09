use annoying::{ImplGenerics, TypeGenerics};
use proc_macro2::{Literal, TokenStream};
use quote::{format_ident, quote};
use syn::{Ident, WhereClause};

use crate::{
    ast::{self, Fallthrough, Style},
    tuple_enum::expand_tuple_enum,
};

pub fn expand_derive_serialize(input: syn::DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let data_enum = ast::parse_data(input)?;

    // if we need the 'de lifetime do same trick as serde
    let (_, _, where_clause) = data_enum.generics.split_for_impl();

    let this_type = &data_enum.ident;

    let enum_variant = enum_variant(&data_enum);

    let impl_generics = ImplGenerics(&data_enum.generics);
    let ty_generics = TypeGenerics(&data_enum.generics);

    let body = match data_enum.vars {
        Style::Struct {
            variants,
            fallthrough,
        } => expand_struct_enum(
            &data_enum.ident,
            (impl_generics, ty_generics, where_clause),
            &variants,
            fallthrough.as_ref(),
        )?,
        Style::Tuple(variants) => expand_tuple_enum(&data_enum.ident, &variants)?,
    };

    Ok(quote! {
        #[automatically_derived]
        impl <'de, #impl_generics > serde::Deserialize<'de> for #this_type < #ty_generics > #where_clause {
            fn deserialize<__D>(__deserializer: __D) -> Result<Self, __D::Error>
            where __D: serde::Deserializer<'de>
            {
               #enum_variant

               #body
            }

        }
    })
}

pub fn enum_variant(enum_: &ast::Enum) -> proc_macro2::TokenStream {
    match &enum_.vars {
        Style::Tuple(_) => {
            // Tuple enums don't generate a separate variant enum type
            // (though they perhaps could).
            // Instead the code is structured as a series of 'trials'
            // like `serde(untagged)` which commits as soon as a matching variant is found.
            quote! {}
        }
        Style::Struct {
            variants,
            fallthrough,
        } => generate_variant_enum(&variants, fallthrough.as_ref()),
    }
}

pub fn expand_struct_enum(
    ty_name: &Ident,
    generics: (ImplGenerics, TypeGenerics, Option<&WhereClause>),
    variants: &[ast::Variant],
    fallthrough: Option<&Fallthrough>,
) -> syn::Result<proc_macro2::TokenStream> {
    let (impl_generics, ty_generics, where_clause) = generics;
    let this_type_str = Literal::string(&ty_name.to_string());

    let mut variant_arms = vec![];
    for (ix, var) in variants.iter().enumerate() {
        let block = deserialize_fields(&var.fields);

        let variant = implement_variant_deserializer(
            &var.ident,
            &var.fields,
            &ty_name,
            &impl_generics,
            &ty_generics,
            &where_clause,
        );
        let cons = format_ident!("__variant{ix}");
        variant_arms.push(quote! {
            __Variant::#cons => {#block #variant }
        });
    }

    if let Some(fall) = &fallthrough {
        let variant = implement_fallthrough_deserializer(&fall, &ty_name);

        variant_arms.push(quote! {
            __Variant::Fallthrough => { #variant }
        });
    }

    let fallthrough = if fallthrough.is_some() {
        quote! { Some(__Variant::Fallthrough) }
    } else {
        quote! { None }
    };

    Ok(quote! {
         let (__tag, __content) = serde::Deserializer::deserialize_any(
            __deserializer,
            serde_implicit::__private::TaggedContentVisitor::<__Variant>::new(#this_type_str, #fallthrough))?;
        let __deserializer = serde_implicit::__private::ContentDeserializer::<__D::Error>::new(__content);

        match __tag {
            #(#variant_arms)*
        }
    })
}

pub fn generate_variant_enum(
    variants: &[ast::Variant],
    fallthrough: Option<&Fallthrough>,
) -> TokenStream {
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
            #tag_value => ::std::result::Result::Ok(__Variant::#variant),
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
            #byte_tokens => ::std::result::Result::Ok(__Variant::#variant),
        }
    });

    let fallthrough_variant = fallthrough.map(|_| {
        quote! { Fallthrough }
    });

    quote! {
        #[allow(non_camel_case_types)]
        #[doc(hidden)]
        enum __Variant {
            #variant_enum_variants
            #fallthrough_variant
        }

        #[doc(hidden)]
        struct __VariantVisitor;

        #[automatically_derived]
        impl<'de> serde::de::Visitor<'de> for __VariantVisitor {
            type Value = __Variant;

            fn expecting(
                &self,
                __formatter: &mut ::std::fmt::Formatter,
            ) -> ::std::fmt::Result {
                ::std::fmt::Formatter::write_str(
                    __formatter,
                    "variant tag identifier",
                )
            }

            fn visit_str<__E>(
                self,
                __value: &str,
            ) -> ::std::result::Result<Self::Value, __E>
            where
                __E: serde::de::Error,
            {
                match __value {
                    #(#visit_str_arms)*
                    _ => ::std::result::Result::Err(__E::missing_field("omg")),
                    // _ => ::std::result::Result::Ok(__Variant::__ignore),
                }
            }

            fn visit_bytes<__E>(
                self,
                __value: &[u8],
            ) -> ::std::result::Result<Self::Value, __E>
            where
                __E: serde::de::Error,
            {
                match __value {
                    #(#visit_bytes_arms)*
                    _ => ::std::result::Result::Err(__E::missing_field("omg")),
                    // _ => ::std::result::Result::Ok(__Variant::__ignore),
                }
            }
        }

        #[automatically_derived]
        impl<'de> serde::Deserialize<'de> for __Variant {
            #[inline]
            fn deserialize<__D>(
                __deserializer: __D,
            ) -> ::std::result::Result<Self, __D::Error>
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

fn deserialize_fields(fields: &ast::Fields) -> TokenStream {
    let field_variants = (0..fields.named.len()).map(|i| {
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

    for (i, field) in fields.named.iter().enumerate() {
        let field_ident = field.ident.as_ref().unwrap();
        let field_name = field_ident.to_string();
        let variant = format_ident!("__field{}", i);

        visit_str_arms.push(quote! {
            #field_name => ::std::result::Result::Ok(__Field::#variant),
        });

        let byte_string = format!("b\"{}\"", field_name);
        let byte_tokens = Literal::byte_string(&byte_string.as_bytes());

        visit_bytes_arms.push(quote! {
            #byte_tokens => ::std::result::Result::Ok(__Field::#variant),
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
                __formatter: &mut ::std::fmt::Formatter,
            ) -> ::std::fmt::Result {
                ::std::fmt::Formatter::write_str(
                    __formatter,
                    "field identifier",
                )
            }

            fn visit_str<__E>(
                self,
                __value: &str,
            ) -> ::std::result::Result<Self::Value, __E>
            where
                __E: serde::de::Error,
            {
                match __value {
                    #(#visit_str_arms)*
                    _ => ::std::result::Result::Ok(__Field::__ignore),
                }
            }

            fn visit_bytes<__E>(
                self,
                __value: &[u8],
            ) -> ::std::result::Result<Self::Value, __E>
            where
                __E: serde::de::Error,
            {
                match __value {
                    #(#visit_bytes_arms)*
                    _ => ::std::result::Result::Ok(__Field::__ignore),
                }
            }
        }

        #[automatically_derived]
        impl<'de> serde::Deserialize<'de> for __Field {
            #[inline]
            fn deserialize<__D>(
                __deserializer: __D,
            ) -> ::std::result::Result<Self, __D::Error>
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

fn implement_fallthrough_deserializer(
    fallthrough: &Fallthrough,
    enum_name: &syn::Ident,
) -> TokenStream {
    let variant_name = &fallthrough.ident;
    let field_name = &fallthrough.field.ident;

    quote! {
        serde::Deserialize::deserialize(__deserializer).map(|res| { #enum_name :: #variant_name { #field_name: res } })
    }
}

fn implement_variant_deserializer(
    variant_ident: &Ident,
    fields: &ast::Fields,
    enum_name: &syn::Ident,
    impl_generics: &ImplGenerics,
    ty_generics: &TypeGenerics,
    where_clause: &Option<&WhereClause>,
) -> TokenStream {
    use quote::{format_ident, quote};

    let variant_name = format!("{}::{}", enum_name, variant_ident);
    let expecting_message = format!("struct variant {}", variant_name);

    let mut field_declarations = Vec::new();
    let mut field_processing = Vec::new();
    let mut final_fields = Vec::new();

    for (i, field) in fields.named.iter().enumerate() {
        let field_ident = field.ident.as_ref().unwrap();
        let field_name = field_ident.to_string();
        let field_type = &field.ty;
        let field_var = format_ident!("__field{}", i);
        let field_enum_variant = format_ident!("__field{}", i);

        field_declarations.push(quote! {
            let mut #field_var: ::std::option::Option<#field_type> = ::std::option::Option::None;
        });

        field_processing.push(quote! {
            __Field::#field_enum_variant => {
                if ::std::option::Option::is_some(&#field_var) {
                    return ::std::result::Result::Err(
                        <__A::Error as serde::de::Error>::duplicate_field(#field_name),
                    );
                }
                #field_var = ::std::option::Option::Some(
                    serde::de::MapAccess::next_value::<#field_type>(&mut __map)?,
                );
            }
        });

        final_fields.push(quote! {
            let #field_var = match #field_var {
                ::std::option::Option::Some(#field_var) => #field_var,
                ::std::option::Option::None => {
                    serde_implicit::__private::missing_field(#field_name)?
                }
            };
        });
    }

    let field_idents = fields.named.iter().map(|f| f.ident.as_ref().unwrap());
    let field_vars = (0..fields.named.len()).map(|i| format_ident!("__field{}", i));

    let struct_init = quote! {
        #enum_name::#variant_ident {
            #(#field_idents: #field_vars),*
        }
    };

    quote! {
        #[doc(hidden)]
        struct __Visitor<'de, #ty_generics> {
            marker: ::std::marker::PhantomData<#enum_name < #ty_generics >>,
            lifetime: ::std::marker::PhantomData<&'de ()>,
        }

        #[automatically_derived]
        impl<'de, #impl_generics> serde::de::Visitor<'de> for __Visitor<'de, #ty_generics> #where_clause {
            type Value =  #enum_name < #ty_generics >;

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
            fn visit_map<__A>(
                self,
                mut __map: __A,
            ) -> ::std::result::Result<Self::Value, __A::Error>
            where
                __A: serde::de::MapAccess<'de>,
            {
                #(#field_declarations)*

                while let ::std::option::Option::Some(__key) = serde::de::MapAccess::next_key::<
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

                ::std::result::Result::Ok(#struct_init)
            }
        }

        serde::Deserializer::deserialize_map(
            __deserializer,
            __Visitor {
                marker: ::std::marker::PhantomData::<#enum_name < #ty_generics > >,
                lifetime: ::std::marker::PhantomData,
            }
        )

    }
}

mod annoying {
    use proc_macro2::TokenStream;
    use quote::{ToTokens, quote};
    use syn::{GenericParam, Generics, Token};

    #[derive(Clone, Copy)]
    pub struct ImplGenerics<'a>(pub(crate) &'a Generics);

    pub(crate) struct TokensOrDefault<'a, T: 'a>(pub &'a Option<T>);

    impl<'a, T> ToTokens for TokensOrDefault<'a, T>
    where
        T: ToTokens + Default,
    {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            match self.0 {
                Some(t) => t.to_tokens(tokens),
                None => T::default().to_tokens(tokens),
            }
        }
    }

    impl<'a> ToTokens for ImplGenerics<'a> {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            if self.0.params.is_empty() {
                return;
            }

            // TokensOrDefault(&self.0.lt_token).to_tokens(tokens);

            // Print lifetimes before types and consts, regardless of their
            // order in self.params.
            let mut trailing_or_empty = true;
            for param in self.0.params.pairs() {
                if let GenericParam::Lifetime(_) = **param.value() {
                    param.to_tokens(tokens);
                    trailing_or_empty = param.punct().is_some();
                }
            }
            for param in self.0.params.pairs() {
                if let GenericParam::Lifetime(_) = **param.value() {
                    continue;
                }
                if !trailing_or_empty {
                    <Token![,]>::default().to_tokens(tokens);
                    trailing_or_empty = true;
                }
                match param.value() {
                    GenericParam::Lifetime(_) => unreachable!(),
                    GenericParam::Type(param) => {
                        // Leave off the type parameter defaults
                        param.ident.to_tokens(tokens);
                        // super hack
                        if !param.bounds.is_empty() {
                            TokensOrDefault(&param.colon_token).to_tokens(tokens);
                            param.bounds.to_tokens(tokens);
                            tokens.extend(quote! { + serde::Deserialize<'de> });
                        } else {
                            tokens.extend(quote! { :serde::Deserialize<'de> });
                        }
                    }
                    GenericParam::Const(param) => {
                        // Leave off the const parameter defaults
                        param.const_token.to_tokens(tokens);
                        param.ident.to_tokens(tokens);
                        param.colon_token.to_tokens(tokens);
                        param.ty.to_tokens(tokens);
                    }
                }
                param.punct().to_tokens(tokens);
            }

            // TokensOrDefault(&self.0.gt_token).to_tokens(tokens);
        }
    }

    #[derive(Clone, Copy)]
    pub struct TypeGenerics<'a>(pub(crate) &'a Generics);

    impl<'a> ToTokens for TypeGenerics<'a> {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            if self.0.params.is_empty() {
                return;
            }

            // TokensOrDefault(&self.0.lt_token).to_tokens(tokens);

            // Print lifetimes before types and consts, regardless of their
            // order in self.params.
            let mut trailing_or_empty = true;
            for param in self.0.params.pairs() {
                if let GenericParam::Lifetime(def) = *param.value() {
                    // Leave off the lifetime bounds and attributes
                    def.lifetime.to_tokens(tokens);
                    param.punct().to_tokens(tokens);
                    trailing_or_empty = param.punct().is_some();
                }
            }
            for param in self.0.params.pairs() {
                if let GenericParam::Lifetime(_) = **param.value() {
                    continue;
                }
                if !trailing_or_empty {
                    <Token![,]>::default().to_tokens(tokens);
                    trailing_or_empty = true;
                }
                match param.value() {
                    GenericParam::Lifetime(_) => unreachable!(),
                    GenericParam::Type(param) => {
                        param.ident.to_tokens(tokens);
                    }
                    GenericParam::Const(param) => {
                        // Leave off the const parameter defaults
                        param.ident.to_tokens(tokens);
                    }
                }
                param.punct().to_tokens(tokens);
            }

            // TokensOrDefault(&self.0.gt_token).to_tokens(tokens);
        }
    }
}
