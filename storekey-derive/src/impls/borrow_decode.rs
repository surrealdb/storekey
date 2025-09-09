use proc_macro2::{Literal, Span, TokenStream};
use quote::quote;
use syn::{DeriveInput, Ident, Result, parse2, spanned::Spanned};

use crate::impls::{build_generics_types, extract_formats};

fn impl_format(input: &DeriveInput, format: Option<&TokenStream>) -> Result<TokenStream> {
	let mut store = None;

	let (format_generic, format) = if let Some(f) = format {
		(quote! {}, f)
	} else {
		(quote! { FormatGen,  }, &*store.insert(quote! {FormatGen}))
	};

	let mut lifetime = quote! { 'de };
	let mut found_lifetime = false;
	for l in input.generics.lifetimes() {
		if found_lifetime {
			return Err(syn::Error::new(
				l.span(),
				"derive(BorrowDecode) can only handle structs with no or a single lifetime",
			));
		}

		lifetime = quote! { #l };
		found_lifetime = true;
	}

	let name = &input.ident;

	let inner = match &input.data {
		syn::Data::Struct(data_struct) => {
			let members = data_struct.fields.members();
			quote! {
				Ok(Self{
					#(#members: ::storekey::BorrowDecode::<#format>::borrow_decode(_r)?),*
				})
			}
		}
		syn::Data::Enum(data_enum) => {
			if data_enum.variants.is_empty() {
				return Err(syn::Error::new(
					data_enum.variants.span(),
					"derive(Decode) needs enums to have atleast a single variant.",
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

				let bind_fields = match &v.fields {
					syn::Fields::Named(_) => {
						let members = v.fields.members();

						quote! {
							#idx => Ok(Self::#name{
								#(#members: ::storekey::BorrowDecode::<#format>::borrow_decode(_r)?),*
							})
						}
					}
					syn::Fields::Unnamed(fields_unnamed) => {
						let decode = fields_unnamed.unnamed.iter().map(|_| {
							quote! {
								::storekey::BorrowDecode::<#format>::borrow_decode(_r)?
							}
						});

						quote! {
							#idx => Ok(Self::#name(
									#(#decode),*
							))
						}
					}
					syn::Fields::Unit => {
						quote! {
							#idx => Ok(Self::#name)
						}
					}
				};

				variants.push(bind_fields);
			}

			quote! {
				let variant: #decode_type = ::storekey::BorrowDecode::<#format>::borrow_decode(_r)?;
				match variant {
					#(#variants,)*
					_ => Err(::storekey::DecodeError::InvalidFormat)
				}
			}
		}
		syn::Data::Union(u) => {
			return Err(syn::Error::new(
				u.union_token.span,
				"derive(BorrowDecode) is not supported for Unions",
			));
		}
	};

	let (_, ty_generics, where_clause) = input.generics.split_for_impl();
	let type_bounds = build_generics_types(
		parse2(quote! { ::storekey::BorrowDecode<#lifetime, #format> }).unwrap(),
		&input.generics,
	);
	let consts = input.generics.const_params();

	Ok(quote! {
		impl <#lifetime, #format_generic #type_bounds #(#consts,)* > ::storekey::BorrowDecode<#lifetime, #format> for #name #ty_generics #where_clause {
			fn borrow_decode(_r: &mut ::storekey::BorrowReader<#lifetime>) -> ::std::result::Result<Self, ::storekey::DecodeError>{
				#inner
			}
		}
	})
}

pub fn borrow_decode(input: TokenStream) -> Result<TokenStream> {
	let input = parse2::<DeriveInput>(input)?;

	let formats = extract_formats(&input.attrs)?;

	let formats = if formats.is_empty() {
		vec![impl_format(&input, None)?]
	} else {
		formats
			.iter()
			.map(|x| impl_format(&input, Some(x)))
			.collect::<Result<Vec<TokenStream>>>()?
	};

	Ok(quote! { #(#formats)* })
}
