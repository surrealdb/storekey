use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident, quote};
use syn::parse::Parse;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{
	Attribute, DeriveInput, Generics, LitStr, Result, Token, TypeParamBound, custom_keyword, parse2,
};

mod borrow_decode;
mod decode;
mod encode;

pub use borrow_decode::borrow_decode;
pub use decode::decode;
pub use encode::encode;

custom_keyword!(format);

fn build_generics_types(bound: TypeParamBound, generics: &Generics) -> TokenStream {
	let mut types = Punctuated::<_, Token![,]>::new();

	for t in generics.type_params() {
		let mut ty = t.clone();
		if ty.colon_token.is_none() {
			ty.colon_token = Some(Token![:](proc_macro2::Span::call_site()));
		}
		ty.bounds.push(bound.clone());
		types.push(ty);
	}

	if !types.trailing_punct() && !types.is_empty() {
		types.push_punct(Default::default());
	}

	types.into_token_stream()
}

fn extract_formats(attrs: &[Attribute]) -> Result<Vec<TokenStream>> {
	struct Format(TokenStream);

	impl Parse for Format {
		fn parse(input: syn::parse::ParseStream) -> Result<Self> {
			input.parse::<format>()?;
			input.parse::<Token![=]>()?;
			let l = input.parse::<LitStr>()?;
			Ok(Format(l.parse::<TokenStream>()?))
		}
	}

	let mut res = Vec::new();

	for at in attrs {
		if at.path().is_ident("storekey") {
			res.push(at.parse_args::<Format>()?.0);
		}
	}

	Ok(res)
}

pub fn to_escaped(input: TokenStream) -> Result<TokenStream> {
	let input = parse2::<DeriveInput>(input)?;

	let name = input.ident;

	let new_name = format_ident!("{}Escaped", name);

	let mut old_lifetime = quote! { 'de };
	let mut new_lifetime = quote! { 'de };
	let mut found_lifetime = false;

	let (_, ty_generics, where_clause) = input.generics.split_for_impl();
	let lifetimes = input.generics.lifetimes();
	let consts = input.generics.const_params().collect::<Vec<_>>();
	let type_params = input.generics.type_params().map(|x| &x.ident);
	let consts_params = input.generics.const_params().map(|x| &x.ident);
	let type_bounds =
		build_generics_types(parse2(quote! { ::storekey::ToEscaped }).unwrap(), &input.generics);

	for l in input.generics.lifetimes() {
		if found_lifetime {
			return Err(syn::Error::new(
				l.span(),
				"derive(ToEscaped) can only handle structs with no or a single lifetime",
			));
		}

		if l.lifetime.ident == "de" {
			new_lifetime = quote! { 'esc };
		}
		old_lifetime = quote! { #l };
		found_lifetime = true;
	}

	let ty = match input.data {
		syn::Data::Struct(data_struct) => match data_struct.fields {
			syn::Fields::Named(fields_named) => {
				let fields = fields_named.named.iter().map(|x| {
					let attr = &x.attrs;
					let vis = &x.vis;
					let ident = &x.ident;
					let colon_token = &x.colon_token;
					let ty = &x.ty;
					quote! {
						#(#attr)*
						#vis #ident #colon_token <#ty as ::storekey::ToEscaped>::Escaped<#old_lifetime>
					}
				});

				quote! {
					struct #new_name<#old_lifetime, #type_bounds #(#consts,)*> {
						#(#fields),*
					}
				}
			}
			syn::Fields::Unnamed(fields_unnamed) => {
				let fields = fields_unnamed.unnamed.iter().map(|x| {
					let attr = &x.attrs;
					let vis = &x.vis;
					let ty = &x.ty;
					quote! {
						#(#attr)*
						#vis  <#ty as ::storekey::ToEscaped>::Escaped<#old_lifetime>
					}
				});
				quote! {
					struct #new_name<#old_lifetime, #type_bounds #(#consts,)*>	(
						#(#fields),*
					)
				}
			}
			syn::Fields::Unit => quote! {
				struct #new_name < #type_bounds #(#consts,)* >;
			},
		},
		syn::Data::Enum(data_enum) => {
			let mut variants = Vec::new();

			for v in data_enum.variants {
				let name = v.ident;
				match v.fields {
					syn::Fields::Named(fields_named) => {
						let fields = fields_named.named.iter().map(|f| {
							let attr = &f.attrs;
							let vis = &f.vis;
							let ident = &f.ident;
							let colon_token = &f.colon_token;
							let ty = &f.ty;
							quote! {
								#(#attr)*
								#vis #ident #colon_token <#ty as ::storekey::ToEscaped>::Escaped<#old_lifetime>
							}
						});
						variants.push(quote! {
							Self::#name {
								#(#fields),*
							}
						});
					}
					syn::Fields::Unnamed(fields_unnamed) => {
						let fields = fields_unnamed.unnamed.iter().map(|x| {
							let attr = &x.attrs;
							let vis = &x.vis;
							let ty = &x.ty;
							quote! {
								#(#attr)*
								#vis  <#ty as ::storekey::ToEscaped>::Escaped<#old_lifetime>
							}
						});
						variants.push(quote! {
							Self::#name (
								#(#fields),*
							)
						});
					}
					syn::Fields::Unit => variants.push(quote! { Self::#name }),
				}
			}

			quote! {
				enum #new_name<#old_lifetime, #type_bounds #(#consts,)*> {
					#(#variants),*
				}
			}
		}
		syn::Data::Union(u) => {
			return Err(syn::Error::new(
				u.union_token.span,
				"derive(ToEscaped) is not supported for Unions",
			));
		}
	};

	Ok(quote! {
		#[derive(::storekey::Encode, ::storekey::BorrowDecode)]
		#ty

		impl<#(#lifetimes,)* #type_bounds #(#consts,)* > ::storekey::ToEscaped for #name #ty_generics #where_clause {
			type Escaped<#new_lifetime> = #new_name < #new_lifetime, #(#type_params,)* #(#consts_params,)* >;

		}
	})
}
