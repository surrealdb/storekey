use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident, quote};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{DeriveInput, Generics, Result, Token, TypeParamBound, parse2};

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

pub fn encode(input: TokenStream) -> Result<TokenStream> {
	let input = parse2::<DeriveInput>(input)?;

	let name = input.ident;

	let inner = match input.data {
		syn::Data::Struct(data_struct) => {
			let members = data_struct.fields.members();
			quote! {
				#(::storekey::Encode::encode(&self.#members,_w)?;)*
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

			for (idx, v) in data_enum.variants.iter().enumerate() {
				let name = &v.ident;
				let idx = idx as u32;
				match &v.fields {
					syn::Fields::Named(_) => {
						let members = v.fields.members();
						let members_b = v.fields.members();

						variants.push(quote! {
							Self::#name{
								#(#members),*
							} => {
								let discriminant: u32 = #idx;
								::storekey::Encode::encode(&discriminant,_w)?;
								#(::storekey::Encode::encode(&#members_b,_w)?;)*
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
								let discriminant: u32 = #idx;
								::storekey::Encode::encode(&discriminant,_w)?;
								#(::storekey::Encode::encode(&#fields,_w)?;)*
							}
						});
					}
					syn::Fields::Unit => variants.push(quote! {
						Self::#name => {
							let discriminant: u32 = #idx;
							::storekey::Encode::encode(&discriminant,_w)?;
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
		impl <#(#lifetimes,)* #type_bounds #(#consts,)* > ::storekey::Encode for #name  #ty_generics #where_clause {
			fn encode<W: ::std::io::Write>(&self, _w: &mut ::storekey::Writer<W>) -> ::storekey::Result<()>{
				#inner
				Ok(())
			}
		}
	})
}

pub fn decode(input: TokenStream) -> Result<TokenStream> {
	let input = parse2::<DeriveInput>(input)?;

	let name = input.ident;

	let inner = match input.data {
		syn::Data::Struct(data_struct) => {
			let members = data_struct.fields.members();
			quote! {
				Ok(Self{
					#(#members: ::storekey::Decode::decode(_r)?),*
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

			for (idx, v) in data_enum.variants.iter().enumerate() {
				let name = &v.ident;
				let idx = idx as u32;
				let bind_fields = match &v.fields {
					syn::Fields::Named(_) => {
						let members = v.fields.members();

						quote! {
							#idx => Ok(Self::#name{
								#(#members: ::storekey::Decode::decode(_r)?),*
							})
						}
					}
					syn::Fields::Unnamed(fields_unnamed) => {
						let decode = fields_unnamed.unnamed.iter().map(|_| {
							quote! {
								::storekey::Decode::decode(_r)?
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
				let variant: u32 = ::storekey::Decode::decode(_r)?;
				match variant {
					#(#variants,)*
					_ => Err(::storekey::Error::UnexpectedDiscriminant)
				}
			}
		}
		syn::Data::Union(u) => {
			return Err(syn::Error::new(
				u.union_token.span,
				"derive(Decode) is not supported for Unions",
			));
		}
	};

	let (_, ty_generics, where_clause) = input.generics.split_for_impl();
	let type_bounds =
		build_generics_types(parse2(quote! { ::storekey::Decode }).unwrap(), &input.generics);
	let lifetimes = input.generics.lifetimes();
	let consts = input.generics.const_params();

	Ok(quote! {
		impl <#(#lifetimes,)* #type_bounds #(#consts,)* > ::storekey::Decode for #name #ty_generics #where_clause {
			fn decode<R: ::std::io::BufRead>(_r: &mut ::storekey::Reader<R>) -> ::storekey::Result<Self>{
				#inner
			}
		}
	})
}

pub fn borrow_decode(input: TokenStream) -> Result<TokenStream> {
	let input = parse2::<DeriveInput>(input)?;

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

	let name = input.ident;

	let inner = match input.data {
		syn::Data::Struct(data_struct) => {
			let members = data_struct.fields.members();
			quote! {
				Ok(Self{
					#(#members: ::storekey::BorrowDecode::borrow_decode(_r)?),*
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

			for (idx, v) in data_enum.variants.iter().enumerate() {
				let name = &v.ident;
				let idx = idx as u32;
				let bind_fields = match &v.fields {
					syn::Fields::Named(_) => {
						let members = v.fields.members();

						quote! {
							#idx => Ok(Self::#name{
								#(#members: ::storekey::BorrowDecode::borrow_decode(_r)?),*
							})
						}
					}
					syn::Fields::Unnamed(fields_unnamed) => {
						let decode = fields_unnamed.unnamed.iter().map(|_| {
							quote! {
								::storekey::BorrowDecode::borrow_decode(_r)?
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
				let variant: u32 = ::storekey::BorrowDecode::borrow_decode(_r)?;
				match variant {
					#(#variants,)*
					_ => Err(::storekey::Error::UnexpectedDiscriminant)
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
		parse2(quote! { ::storekey::BorrowDecode<#lifetime> }).unwrap(),
		&input.generics,
	);
	let consts = input.generics.const_params();

	Ok(quote! {
		impl <#lifetime, #type_bounds #(#consts,)* > ::storekey::BorrowDecode<#lifetime> for #name #ty_generics #where_clause {
			fn borrow_decode(_r: &mut ::storekey::BorrowReader<#lifetime>) -> ::storekey::Result<Self>{
				#inner
			}
		}
	})
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
				struct #new_name<#type_bounds #(#consts,)*>;
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
			type Escaped<#new_lifetime> = #new_name<#new_lifetime, #(#type_params,)* #(#consts_params,)*>;

		}
	})
}
