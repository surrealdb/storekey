use std::{
	borrow::Cow,
	collections::{BTreeMap, HashMap},
	io::Write,
};

use super::{Encode, Result, Writer};

impl<E: Encode + ?Sized> Encode for &E {
	fn encode<W: Write>(&self, w: &mut Writer<W>) -> Result<()> {
		E::encode(self, w)
	}
}

impl Encode for str {
	fn encode<W: Write>(&self, w: &mut Writer<W>) -> Result<()> {
		w.write_slice(self.as_bytes())
	}
}

impl Encode for String {
	fn encode<W: Write>(&self, w: &mut Writer<W>) -> Result<()> {
		w.write_slice(self.as_bytes())
	}
}

impl<E: Encode> Encode for [E] {
	fn encode<W: Write>(&self, w: &mut Writer<W>) -> Result<()> {
		for e in self.iter() {
			w.mark_terminator();
			e.encode(w)?;
		}
		w.write_terminator()
	}
}

impl<E: Encode> Encode for Vec<E> {
	fn encode<W: Write>(&self, w: &mut Writer<W>) -> Result<()> {
		for e in self.iter() {
			w.mark_terminator();
			e.encode(w)?;
		}
		w.write_terminator()
	}
}

impl<E: Encode + ToOwned> Encode for Cow<'_, E> {
	fn encode<W: Write>(&self, w: &mut Writer<W>) -> Result<()> {
		self.as_ref().encode(w)
	}
}

impl<K: Encode, V: Encode, S> Encode for HashMap<K, V, S> {
	fn encode<W: Write>(&self, w: &mut Writer<W>) -> Result<()> {
		for (k, v) in self.iter() {
			w.mark_terminator();
			k.encode(w)?;
			v.encode(w)?;
		}
		w.write_terminator()
	}
}

impl<K: Encode, V: Encode> Encode for BTreeMap<K, V> {
	fn encode<W: Write>(&self, w: &mut Writer<W>) -> Result<()> {
		for (k, v) in self.iter() {
			w.mark_terminator();
			k.encode(w)?;
			v.encode(w)?;
		}
		w.write_terminator()
	}
}

impl<T: Encode, const SIZE: usize> Encode for [T; SIZE] {
	fn encode<W: Write>(&self, w: &mut Writer<W>) -> Result<()> {
		for i in self.iter() {
			i.encode(w)?;
		}
		Ok(())
	}
}

macro_rules! impl_encode_tuple{
    ($($t:ident),*$(,)?) => {
		impl <$($t: Encode),*> Encode for ($($t,)*){
			#[allow(non_snake_case)]
			fn encode<W: Write>(&self, _w: &mut Writer<W>) -> Result<()> {
				let ($($t,)*) = self;
				$(
					$t.encode(_w)?;
				)*
				Ok(())

			}
		}

    };
}

impl_encode_tuple!();
impl_encode_tuple!(A);
impl_encode_tuple!(A, B);
impl_encode_tuple!(A, B, C);
impl_encode_tuple!(A, B, C, D);
impl_encode_tuple!(A, B, C, D, E);
impl_encode_tuple!(A, B, C, D, E, F);

macro_rules! impl_encode_prim {
	($ty:ident,$name:ident) => {
		impl Encode for $ty {
			fn encode<W: Write>(&self, w: &mut Writer<W>) -> Result<()> {
				w.$name(*self)
			}
		}
	};
}

impl_encode_prim!(u8, write_u8);
impl_encode_prim!(i8, write_i8);
impl_encode_prim!(u16, write_u16);
impl_encode_prim!(i16, write_i16);
impl_encode_prim!(u32, write_u32);
impl_encode_prim!(i32, write_i32);
impl_encode_prim!(u64, write_u64);
impl_encode_prim!(i64, write_i64);
impl_encode_prim!(u128, write_u128);
impl_encode_prim!(i128, write_i128);
impl_encode_prim!(f32, write_f32);
impl_encode_prim!(f64, write_f64);
