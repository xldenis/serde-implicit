use proc_macro2::{Literal, TokenStream};
use quote::{format_ident, quote};
use syn::{Error, Fields, Ident, punctuated::Punctuated};

pub fn tuple_variant_enum(
    ty_name: &Ident,
    variants: &Punctuated<syn::Variant, syn::token::Comma>,
) -> TokenStream {
    use quote::{format_ident, quote};

    let variant_enum_variants = variants.iter().enumerate().map(|(i, variant)| {
        let variant_ident = format_ident!("__variant{}", i);

        if let Some(field) = variant.fields.iter().next() {
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

        if let Some(field) = variant.fields.iter().next() {
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

pub fn expand_tuple_enum(input: syn::DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let enum_ = match input.data {
        syn::Data::Enum(e) => e,
        _ => return Err(Error::new_spanned(input, "unsupported")),
    };

    let enum_variant = tuple_variant_enum(&input.ident, &enum_.variants);

    // if we need the 'de lifetime do same trick as serde
    let (_, ty_generics, where_clause) = input.generics.split_for_impl();
    let this_type = &input.ident;
    // let this_type_str = Literal::string(&this_type.to_string());

    let mut variant_arms = vec![];

    for (i, v) in enum_.variants.iter().enumerate() {
        let variant_ident = format_ident!("__variant{}", i);
        let original_variant_ident = &v.ident;

        let Fields::Unnamed(fields) = &v.fields else {
            return Err(Error::new_spanned(v, "unsupported"));
        };

        let field_count = fields.unnamed.len();

        // Simple case for variants with just one field
        if field_count == 1 {
            variant_arms.push(quote! {
                __Variant::#variant_ident(__value) => {

                    if __content.is_some() {
                        return serde::__private::Err(
                            serde::de::Error::invalid_length(
                                0,
                                &concat!("tuple variant ", stringify!(#this_type), "::", stringify!(#original_variant_ident), " with ", stringify!(#field_count), " elements"),
                            ),
                        );
                    };

                    Ok(#this_type::#original_variant_ident(__value))
                }
            });
            continue;
        }

        // For variants with multiple fields, we need to generate a visitor
        let visitor_name = format_ident!("__{}Visitor", original_variant_ident);

        // Generate code for deserializing each field
        let field_deserialize = fields.unnamed.iter().skip(1).enumerate().map(|(j, field)| {
            let field_name = format_ident!("__field{}", j + 1); // Start from 1 since the first field is already handled
            let field_type = &field.ty;
            let idx = proc_macro2::Literal::usize_unsuffixed(j);

            quote! {
                let #field_name = match serde::de::SeqAccess::next_element::<#field_type>(&mut __seq)? {
                    serde::__private::Some(__value) => __value,
                    serde::__private::None => {
                        return serde::__private::Err(
                            serde::de::Error::invalid_length(
                                #idx,
                                &concat!("tuple variant ", stringify!(#this_type), "::", stringify!(#original_variant_ident), " with ", stringify!(#field_count), " elements"),
                            ),
                        );
                    }
                };
            }
        });

        // Generate the fields for constructing the variant
        let field_names = (1..field_count).map(|j| format_ident!("__field{}", j));

        let variant_str = format!("tuple variant {}::{}", this_type, original_variant_ident);
        // let variant_elements_str = format!("tuple variant {}::{} with {} elements",
        //     this_type, original_variant_ident, field_count);

        // For variants with multiple fields, we use the first field from __Variant
        // and deserialize the remaining fields with a visitor
        let field_count_remaining = field_count - 1;
        let tag_ty = &fields.unnamed.first().as_ref().unwrap().ty;
        variant_arms.push(quote! {
            __Variant::#variant_ident(__first_field) => {
                #[doc(hidden)]
                struct #visitor_name {
                    marker: serde::__private::PhantomData<#this_type>,
                    first_field: #tag_ty,
                }

                impl<'de> serde::de::Visitor<'de> for #visitor_name {
                    type Value = #this_type;

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
                        // Skip deserializing the first field since we already have it
                        #(#field_deserialize)*

                        serde::__private::Ok(#this_type::#original_variant_ident(self.first_field #(, #field_names)*))
                    }
                }

                let __deserializer = serde::__private::de::ContentDeserializer::<__D::Error>::new(__content.unwrap());

                serde::Deserializer::deserialize_tuple(
                    __deserializer,
                    #field_count_remaining,
                    #visitor_name {
                        marker: serde::__private::PhantomData::<#this_type>,
                        first_field: __first_field,
                    },
                )
            }
        });
    }

    Ok(quote! {
        #[automatically_derived]
        impl<'de> serde::Deserialize<'de> for #this_type #ty_generics #where_clause {
            fn deserialize<__D>(__deserializer: __D) -> Result<Self, __D::Error>
            where __D: serde::Deserializer<'de>
            {
                #enum_variant


                let __content = <serde::__private::de::Content as serde::Deserialize>::deserialize(
                    __deserializer,
                )?;

                let (tag, __content) = serde_implicit::__private::pop_front(__content)?;

                let __deserializer = serde::__private::de::ContentRefDeserializer::<__D::Error,>::new(&tag);

                let __tag = deserialize_variant(__deserializer)?;

                match __tag {
                    #(#variant_arms)*
                }
            }
        }
    })
}
