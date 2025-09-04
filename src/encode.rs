use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};
use std::io::Write;
use std::ops::Bound;
use std::time::Duration;

use super::{Encode, EncodeError, Writer};

impl<F, E: Encode<F> + ?Sized> Encode<F> for &E {
	fn encode<W: Write>(&self, w: &mut Writer<W>) -> Result<(), EncodeError> {
		E::encode(self, w)
	}
}

impl<F, E: Encode<F>> Encode<F> for Option<E> {
	fn encode<W: Write>(&self, w: &mut Writer<W>) -> Result<(), EncodeError> {
		match self.as_ref() {
			None => w.write_u8(2),
			Some(x) => {
				w.write_u8(3)?;
				E::encode(x, w)
			}
		}
	}
}

impl<F, E: Encode<F>> Encode<F> for Bound<E> {
	fn encode<W: Write>(&self, w: &mut Writer<W>) -> Result<(), EncodeError> {
		match self {
			Bound::Excluded(v) => {
				w.write_u8(4)?;
				Encode::encode(v, w)
			}
			Bound::Included(v) => {
				w.write_u8(3)?;
				Encode::encode(v, w)
			}
			Bound::Unbounded => w.write_u8(2),
		}
	}
}

impl<F, O: Encode<F>, E: Encode<F>> Encode<F> for Result<O, E> {
	fn encode<W: Write>(&self, w: &mut Writer<W>) -> Result<(), EncodeError> {
		match self.as_ref() {
			Ok(x) => {
				w.write_u8(2)?;
				O::encode(x, w)
			}
			Err(e) => {
				w.write_u8(3)?;
				E::encode(e, w)
			}
		}
	}
}

impl<F> Encode<F> for bool {
	fn encode<W: Write>(&self, w: &mut Writer<W>) -> Result<(), EncodeError> {
		if *self {
			w.write_u8(3)
		} else {
			w.write_u8(2)
		}
	}
}

impl<F> Encode<F> for char {
	fn encode<W: Write>(&self, w: &mut Writer<W>) -> Result<(), EncodeError> {
		w.write_u32(*self as u32)
	}
}

impl<F> Encode<F> for str {
	fn encode<W: Write>(&self, w: &mut Writer<W>) -> Result<(), EncodeError> {
		w.write_slice(self.as_bytes())
	}
}

impl<F> Encode<F> for String {
	fn encode<W: Write>(&self, w: &mut Writer<W>) -> Result<(), EncodeError> {
		w.write_slice(self.as_bytes())
	}
}

impl<F> Encode<F> for Duration {
	fn encode<W: Write>(&self, w: &mut Writer<W>) -> Result<(), EncodeError> {
		w.write_u64(self.as_secs())?;
		w.write_u32(self.subsec_nanos())
	}
}

impl<F, E: Encode<F>> Encode<F> for [E] {
	fn encode<W: Write>(&self, w: &mut Writer<W>) -> Result<(), EncodeError> {
		for e in self.iter() {
			w.mark_terminator();
			e.encode(w)?;
		}
		w.write_terminator()
	}
}

impl<F, E: Encode<F>> Encode<F> for Vec<E> {
	fn encode<W: Write>(&self, w: &mut Writer<W>) -> Result<(), EncodeError> {
		for e in self.iter() {
			w.mark_terminator();
			e.encode(w)?;
		}
		w.write_terminator()
	}
}

impl<F, E: Encode<F>> Encode<F> for Box<E> {
	fn encode<W: Write>(&self, w: &mut Writer<W>) -> Result<(), EncodeError> {
		self.as_ref().encode(w)
	}
}

impl<F, E: Encode<F> + ToOwned + ?Sized> Encode<F> for Cow<'_, E> {
	fn encode<W: Write>(&self, w: &mut Writer<W>) -> Result<(), EncodeError> {
		self.as_ref().encode(w)
	}
}

impl<F, K: Encode<F>, V: Encode<F>, S> Encode<F> for HashMap<K, V, S> {
	fn encode<W: Write>(&self, w: &mut Writer<W>) -> Result<(), EncodeError> {
		for (k, v) in self.iter() {
			w.mark_terminator();
			k.encode(w)?;
			v.encode(w)?;
		}
		w.write_terminator()
	}
}

impl<F, K: Encode<F>, V: Encode<F>> Encode<F> for BTreeMap<K, V> {
	fn encode<W: Write>(&self, w: &mut Writer<W>) -> Result<(), EncodeError> {
		for (k, v) in self.iter() {
			w.mark_terminator();
			k.encode(w)?;
			v.encode(w)?;
		}
		w.write_terminator()
	}
}

impl<F, T: Encode<F>, const SIZE: usize> Encode<F> for [T; SIZE] {
	fn encode<W: Write>(&self, w: &mut Writer<W>) -> Result<(), EncodeError> {
		for i in self.iter() {
			i.encode(w)?;
		}
		Ok(())
	}
}

macro_rules! impl_encode_tuple{
    ($($t:ident),*$(,)?) => {
		impl <Format,$($t: Encode<Format>),*> Encode<Format> for ($($t,)*){
			#[allow(non_snake_case)]
			fn encode<W: Write>(&self, _w: &mut Writer<W>) -> Result<(),EncodeError> {
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
		impl<F> Encode<F> for $ty {
			fn encode<W: Write>(&self, w: &mut Writer<W>) -> Result<(), EncodeError> {
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
