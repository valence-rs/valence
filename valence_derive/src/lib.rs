use proc_macro::TokenStream as StdTokenStream;
use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, ToTokens};
use syn::spanned::Spanned;
use syn::{
    parse2, parse_quote, Data, DeriveInput, Error, Fields, GenericParam, Generics, Lifetime,
    LifetimeDef, LitInt, Result,
};

#[proc_macro_derive(Encode)]
pub fn derive_encode(item: StdTokenStream) -> StdTokenStream {
    match derive_encode_impl(item.into()) {
        Ok(tokens) => tokens.into(),
        Err(e) => e.into_compile_error().into(),
    }
}

#[proc_macro_derive(Decode)]
pub fn derive_decode(item: StdTokenStream) -> StdTokenStream {
    match derive_decode_impl(item.into()) {
        Ok(tokens) => tokens.into(),
        Err(e) => e.into_compile_error().into(),
    }
}

fn derive_encode_impl(item: TokenStream) -> Result<TokenStream> {
    let mut input = parse2::<DeriveInput>(item)?;

    let name = input.ident;

    match input.data {
        Data::Struct(s) => {
            add_trait_bounds(&mut input.generics, quote!(::valence::__private::Encode));

            let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

            let encode_fields = match &s.fields {
                Fields::Named(fields) => fields
                    .named
                    .iter()
                    .map(|f| {
                        let name = &f.ident;
                        quote! {
                            self.#name.encode(&mut _w)?; // TODO: add anyhow context.
                        }
                    })
                    .collect(),
                Fields::Unnamed(fields) => (0..fields.unnamed.len())
                    .map(|i| {
                        let lit = LitInt::new(&i.to_string(), Span::call_site());
                        quote! {
                            self.#lit.encode(&mut _w)?; // TODO: add anyhow context.
                        }
                    })
                    .collect(),
                Fields::Unit => TokenStream::new(),
            };

            let encoded_len_fields = match &s.fields {
                Fields::Named(fields) => fields
                    .named
                    .iter()
                    .map(|f| {
                        let name = &f.ident;
                        quote! {
                            + self.#name.encoded_len()
                        }
                    })
                    .collect(),
                Fields::Unnamed(fields) => (0..fields.unnamed.len())
                    .map(|i| {
                        let lit = LitInt::new(&i.to_string(), Span::call_site());
                        quote! {
                            + self.#lit.encoded_len()
                        }
                    })
                    .collect(),
                Fields::Unit => TokenStream::new(),
            };

            Ok(quote! {
                #[allow(unused_imports)]
                impl #impl_generics ::valence::__private::Encode for #name #ty_generics #where_clause {
                    fn encode(&self, mut _w: impl ::std::io::Write) -> ::valence::__private::Result<()> {
                        use ::valence::__private::{Encode, Context};

                        #encode_fields

                        Ok(())
                    }

                    fn encoded_len(&self) -> usize {
                        use ::valence::__private::{Encode, Context};

                        0 #encoded_len_fields
                    }
                }
            })
        }
        Data::Enum(e) => {
            add_trait_bounds(&mut input.generics, quote!(::valence::__private::Encode));

            let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

            let encode_arms = e
                .variants
                .iter()
                .enumerate()
                .map(|(variant_index, variant)| {
                    let name = &variant.ident;
                    let variant_index = variant_index as i32;

                    match &variant.fields {
                        Fields::Named(fields) => {
                            let fields = fields.named.iter().map(|f| &f.ident).collect::<Vec<_>>();

                            quote! {
                                Self::#name { #(#fields,)* } => {
                                    VarInt(#variant_index).encode(&mut _w)?; // TODO: anyhow context

                                    #(
                                        #fields.encode(&mut _w)?; // TODO: anyhow context.
                                    )*
                                }
                            }
                        }
                        Fields::Unnamed(fields) => {
                            let fields = (0..fields.unnamed.len())
                                .map(|i| Ident::new(&format!("_{i}"), Span::call_site()))
                                .collect::<Vec<_>>();

                            quote! {
                                Self::#name(#(#fields,)*) => {
                                    VarInt(#variant_index).encode(&mut _w)?;

                                    #(
                                        #fields.encode(&mut _w)?; // TODO: anyhow context.
                                    )*
                                }
                            }
                        }
                        Fields::Unit => quote! {
                            Self::#name => VarInt(#variant_index).encode(&mut _w)?,
                        },
                    }
                })
                .collect::<TokenStream>();

            let encoded_len_arms = e
                .variants
                .iter()
                .enumerate()
                .map(|(variant_index, variant)| {
                    let name = &variant.ident;
                    let variant_index = variant_index as i32;

                    match &variant.fields {
                        Fields::Named(fields) => {
                            let fields = fields.named.iter().map(|f| &f.ident).collect::<Vec<_>>();

                            quote! {
                                Self::#name { #(#fields,)* } => {
                                    VarInt(#variant_index).encoded_len()

                                    #(+ #fields.encoded_len())*
                                }
                            }
                        }
                        Fields::Unnamed(fields) => {
                            let fields = (0..fields.unnamed.len())
                                .map(|i| Ident::new(&format!("_{i}"), Span::call_site()))
                                .collect::<Vec<_>>();

                            quote! {
                                Self::#name(#(#fields,)*) => {
                                    VarInt(#variant_index).encoded_len()

                                    #(+ #fields.encoded_len())*
                                }
                            }
                        }
                        Fields::Unit => {
                            quote! {
                                Self::#name => VarInt(#variant_index).encoded_len(),
                            }
                        }
                    }
                })
                .collect::<TokenStream>();

            Ok(quote! {
                #[allow(unused_imports)]
                impl #impl_generics ::valence::Encode for #name #ty_generics #where_clause {
                    fn encode(&self, mut _w: impl ::std::io::Write) -> ::valence::__private::anyhow::Result<()> {
                        use ::valence::__private::{Encode, VarInt, Context};

                        match self {
                            #encode_arms
                            _ => unreachable!(),
                        }

                        Ok(())
                    }
                    #[allow(unused_imports)]
                    fn encoded_len(&self) -> usize {
                        use ::valence::{Encode, VarInt};
                        use ::valence::__private::anyhow::Context;

                        match self {
                            #encoded_len_arms
                            _ => unreachable!(),
                        }
                    }
                }
            })
        }
        Data::Union(u) => Err(Error::new(
            u.union_token.span(),
            "cannot derive `Encode` on unions",
        )),
    }
}

fn derive_decode_impl(item: TokenStream) -> Result<TokenStream> {
    let mut input = parse2::<DeriveInput>(item)?;

    let name = input.ident;

    if input.generics.lifetimes().count() > 1 {
        return Err(Error::new(
            input.generics.params.span(),
            "type deriving `Decode` must have no more than one lifetime",
        ));
    }

    // Use the lifetime specified in the type definition or just use `'a` if not
    // present.
    let lifetime = input
        .generics
        .lifetimes()
        .next()
        .map(|l| l.lifetime.clone())
        .unwrap_or(parse_quote!('a));

    match input.data {
        Data::Struct(s) => {
            let decode_fields = match s.fields {
                Fields::Named(fields) => {
                    let init = fields.named.iter().map(|f| {
                        let name = &f.ident;
                        quote! {
                            #name: Decode::decode(_r)?,
                        }
                    });

                    quote! {
                        Self {
                            #(#init)*
                        }
                    }
                }
                Fields::Unnamed(fields) => {
                    let init = (0..fields.unnamed.len())
                        .map(|_| {
                            quote! {
                                Decode::decode(_r)?,
                            }
                        })
                        .collect::<TokenStream>();

                    quote! {
                        Self(#init)
                    }
                }
                Fields::Unit => quote!(Self),
            };

            add_trait_bounds(&mut input.generics, quote!(::valence::Decode<#lifetime>));

            let (impl_generics, ty_generics, where_clause) =
                decode_split_for_impl(input.generics, lifetime.clone());

            Ok(quote! {
                #[allow(unused_imports)]
                impl #impl_generics ::valence::Decode<#lifetime> for #name #ty_generics
                #where_clause
                {
                    fn decode(_r: &mut &#lifetime [u8]) -> ::valence::__private::anyhow::Result<Self> {
                        use ::valence::{Decode, VarInt};
                        use ::valence::__private::anyhow::Context;

                        Ok(#decode_fields)
                    }
                }
            })
        }
        Data::Enum(e) => {
            let decode_arms = e
                .variants
                .iter()
                .enumerate()
                .map(|(variant_index, variant)| {
                    let name = &variant.ident;
                    let variant_index = variant_index as i32;

                    match &variant.fields {
                        Fields::Named(fields) => {
                            let fields = fields
                                .named
                                .iter()
                                .map(|f| {
                                    let field = &f.ident;
                                    quote!(#field: Decode::decode(_r)?,)
                                })
                                .collect::<TokenStream>();

                            quote! {
                                #variant_index => Ok(Self::#name { #fields }),
                            }
                        }
                        Fields::Unnamed(fields) => {
                            let init = (0..fields.unnamed.len())
                                .map(|_| quote!(Decode::decode(_r)?,))
                                .collect::<TokenStream>();

                            quote! {
                                #variant_index => Ok(Self::#name(#init)),
                            }
                        }
                        Fields::Unit => TokenStream::new(),
                    }
                })
                .collect::<TokenStream>();

            add_trait_bounds(&mut input.generics, quote!(::valence::Decode<#lifetime>));

            let (impl_generics, ty_generics, where_clause) =
                decode_split_for_impl(input.generics, lifetime.clone());

            Ok(quote! {
                #[allow(unused_imports)]
                impl #impl_generics ::valence::Decode<#lifetime> for #name #ty_generics
                #where_clause
                {
                    fn decode(_r: &mut &#lifetime [u8]) -> ::valence::__private::anyhow::Result<Self> {
                        use ::valence::{Decode, VarInt};
                        use ::valence::__private::anyhow::{Context, bail};

                        // TODO: anyhow context.
                        let disc = VarInt::decode(_r)?.0;
                        match disc {
                            #decode_arms
                            n => bail!("unexpected enum discriminant {}", disc),
                        }
                    }
                }
            })
        }
        Data::Union(u) => Err(Error::new(
            u.union_token.span(),
            "cannot derive `Decode` on unions",
        )),
    }
}

/// Adding our lifetime to the generics before calling `.split_for_impl()` would
/// also add it to the resulting ty_generics, which we don't want. So I'm doing
/// this hack.
fn decode_split_for_impl(
    mut generics: Generics,
    lifetime: Lifetime,
) -> (TokenStream, TokenStream, TokenStream) {
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let mut impl_generics = impl_generics.to_token_stream();
    let ty_generics = ty_generics.to_token_stream();
    let where_clause = where_clause.to_token_stream();

    if generics.lifetimes().next().is_none() {
        generics
            .params
            .push(GenericParam::Lifetime(LifetimeDef::new(lifetime)));

        impl_generics = generics.split_for_impl().0.to_token_stream();
    }

    (impl_generics, ty_generics, where_clause)
}

fn add_trait_bounds(generics: &mut Generics, trait_: TokenStream) {
    for param in &mut generics.params {
        if let GenericParam::Type(type_param) = param {
            type_param.bounds.push(parse_quote!(#trait_))
        }
    }
}
