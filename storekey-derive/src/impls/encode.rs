use proc_macro2::{Literal, Span, TokenStream};
use quote::{format_ident, quote};
use syn::{DeriveInput, Ident, Result, parse2, spanned::Spanned};

use crate::impls::{build_generics_types, extract_formats};

pub fn impl_format(input: &DeriveInput, format: &TokenStream) -> Result<TokenStream> {
	let name = &input.ident;

	let inner = match &input.data {
		syn::Data::Struct(data_struct) => {
			let members = data_struct.fields.members();
			quote! {
				#(::storekey::Encode::<#format>::encode(&self.#members,_w)?;)*
			}
		}
		syn::Data::Enum(data_enum) => {
			if data_enum.variants.is_empty() {
				return Err(syn::Error::new(
					data_enum.variants.span(),
					"derive(Encode) needs enums to have atleast a single variant.",
				));
			}

			let mut variants = Vec::new();

			let decode_type = if data_enum.variants.len() > (u8::MAX as usize) - 2 {
				if data_enum.variants.len() > u16::MAX as usize {
					Ident::new("u32", Span::call_site())
				} else {
					Ident::new("u16", Span::call_site())
				}
			} else {
				Ident::new("u8", Span::call_site())
			};

			for (idx, v) in data_enum.variants.iter().enumerate() {
				let name = &v.ident;

				let idx = if data_enum.variants.len() > (u8::MAX as usize) - 2 {
					if data_enum.variants.len() > u16::MAX as usize {
						Literal::u32_suffixed(idx as u32)
					} else {
						Literal::u16_suffixed(idx as u16)
					}
				} else {
					Literal::u8_suffixed((idx as u8) + 2)
				};

				match &v.fields {
					syn::Fields::Named(_) => {
						let members = v.fields.members();
						let members_b = v.fields.members();

						variants.push(quote! {
							Self::#name{
								#(#members),*
							} => {
								let discriminant: #decode_type = #idx;
								::storekey::Encode::<#format>::encode(&discriminant,_w)?;
								#(::storekey::Encode::<#format>::encode(&#members_b,_w)?;)*
							}
						});
					}
					syn::Fields::Unnamed(fields_unnamed) => {
						let fields = fields_unnamed
							.unnamed
							.iter()
							.enumerate()
							.map(|(idx, _)| format_ident!("field_{idx}"))
							.collect::<Vec<_>>();

						variants.push(quote! {
							Self::#name(
								#(#fields),*
							) => {
								let discriminant: #decode_type = #idx;
								::storekey::Encode::<#format>::encode(&discriminant,_w)?;
								#(::storekey::Encode::<#format>::encode(&#fields,_w)?;)*
							}
						});
					}
					syn::Fields::Unit => variants.push(quote! {
						Self::#name => {
							let discriminant: #decode_type = #idx;
							::storekey::Encode::<#format>::encode(&discriminant,_w)?;
						}
					}),
				};
			}

			quote! {
				match self{
					#(#variants),*
				}
			}
		}
		syn::Data::Union(u) => {
			return Err(syn::Error::new(
				u.union_token.span,
				"derive(Encode) is not supported for Unions",
			));
		}
	};

	let (_, ty_generics, where_clause) = input.generics.split_for_impl();
	let type_bounds =
		build_generics_types(parse2(quote! { ::storekey::Encode }).unwrap(), &input.generics);
	let lifetimes = input.generics.lifetimes();
	let consts = input.generics.const_params();

	Ok(quote! {
		impl <#(#lifetimes,)* #type_bounds #(#consts,)* > ::storekey::Encode<#format> for #name  #ty_generics #where_clause {
			fn encode<W: ::std::io::Write>(&self, _w: &mut ::storekey::Writer<W>) -> ::std::result::Result<(), ::storekey::EncodeError>{
				#inner
				Ok(())
			}
		}
	})
}

pub fn encode(input: TokenStream) -> Result<TokenStream> {
	let input = parse2::<DeriveInput>(input)?;

	let formats = extract_formats(&input.attrs)?;

	let formats = formats.iter().map(|x| impl_format(&input, x)).collect::<Result<Vec<_>>>()?;

	Ok(quote! { #(#formats)* })
}
