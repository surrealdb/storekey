use std::collections::{BTreeMap, HashMap};

use super::types::{EscapedSlice, EscapedStr};

pub trait ToEscaped {
	type Escaped<'e>;
}

impl ToEscaped for String {
	type Escaped<'e> = String;
}

impl<T: ToEscaped> ToEscaped for Vec<T> {
	type Escaped<'e> = Vec<T::Escaped<'e>>;
}

impl<K: ToEscaped, V: ToEscaped, S> ToEscaped for HashMap<K, V, S> {
	type Escaped<'e> = HashMap<K::Escaped<'e>, V::Escaped<'e>>;
}

impl<K: ToEscaped, V: ToEscaped> ToEscaped for BTreeMap<K, V> {
	type Escaped<'e> = BTreeMap<K::Escaped<'e>, V::Escaped<'e>>;
}

impl ToEscaped for &str {
	type Escaped<'e> = &'e EscapedStr;
}

impl ToEscaped for &[u8] {
	type Escaped<'e> = &'e EscapedSlice;
}

macro_rules! impl_trivial {
    ($($t:ident),*$(,)?) => {

		$(impl ToEscaped for $t {
			type Escaped<'e> = $t;
		})*

	};
}

impl_trivial! {
	u8, i8,
	u16, i16,
	u32, i32,
	u64, i64,
	f32, f64,
}
