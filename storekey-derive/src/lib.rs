use proc_macro::TokenStream;

mod impls;

#[proc_macro_derive(Encode, attributes(storekey))]
pub fn encode(input: TokenStream) -> TokenStream {
	match impls::encode(input.into()) {
		Ok(x) => x.into(),
		Err(e) => e.into_compile_error().into(),
	}
}

#[proc_macro_derive(Decode, attributes(storekey))]
pub fn decode(input: TokenStream) -> TokenStream {
	match impls::decode(input.into()) {
		Ok(x) => x.into(),
		Err(e) => e.into_compile_error().into(),
	}
}

#[proc_macro_derive(BorrowDecode, attributes(storekey))]
pub fn borrow_decode(input: TokenStream) -> TokenStream {
	match impls::borrow_decode(input.into()) {
		Ok(x) => x.into(),
		Err(e) => e.into_compile_error().into(),
	}
}

#[proc_macro_derive(ToEscaped)]
pub fn to_escaped(input: TokenStream) -> TokenStream {
	match impls::to_escaped(input.into()) {
		Ok(x) => x.into(),
		Err(e) => e.into_compile_error().into(),
	}
}
