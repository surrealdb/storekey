use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};
use std::hash::{BuildHasher, Hash};
use std::io::BufRead;
use std::mem::MaybeUninit;
use std::time::Duration;

use crate::DecodeError;

use super::reader::BorrowReader;
use super::{BorrowDecode, Decode, Reader};

impl Decode for bool {
	fn decode<R: BufRead>(r: &mut Reader<R>) -> Result<Self, DecodeError> {
		match r.read_u8()? {
			2 => Ok(false),
			3 => Ok(true),
			_ => Err(DecodeError::InvalidFormat),
		}
	}
}

impl Decode for char {
	fn decode<R: BufRead>(r: &mut Reader<R>) -> Result<Self, DecodeError> {
		let c = r.read_u32()?;
		char::from_u32(c).ok_or(DecodeError::InvalidFormat)
	}
}

impl Decode for String {
	fn decode<R: BufRead>(r: &mut Reader<R>) -> Result<Self, DecodeError> {
		r.read_string()
	}
}

impl<D: Decode> Decode for Option<D> {
	fn decode<R: BufRead>(r: &mut Reader<R>) -> Result<Self, DecodeError> {
		match r.read_u8()? {
			// Don't use 0 or 1 as those need to be escaped.
			// Todo: Maybe keep it backwards compatible.
			2 => Ok(None),
			3 => Ok(Some(Decode::decode(r)?)),
			_ => Err(DecodeError::InvalidFormat),
		}
	}
}

impl<O: Decode, E: Decode> Decode for Result<O, E> {
	fn decode<R: BufRead>(r: &mut Reader<R>) -> Result<Self, DecodeError> {
		match r.read_u8()? {
			// Don't use 0 or 1 as those need to be escaped.
			// Todo: Maybe keep it backwards compatible.
			2 => Ok(Ok(Decode::decode(r)?)),
			3 => Ok(Err(Decode::decode(r)?)),
			_ => Err(DecodeError::InvalidFormat),
		}
	}
}

impl<D: Decode> Decode for Vec<D> {
	fn decode<R: BufRead>(r: &mut Reader<R>) -> Result<Self, DecodeError> {
		// TODO: Castaway optimize Vec<u8>?
		let mut buffer = Vec::new();

		while !r.read_terminal()? {
			buffer.push(D::decode(r)?);
		}

		Ok(buffer)
	}
}

impl<D: Decode> Decode for Box<D> {
	fn decode<R: BufRead>(r: &mut Reader<R>) -> Result<Self, DecodeError> {
		Ok(Box::new(D::decode(r)?))
	}
}

impl<K: Decode + Hash + Eq, V: Decode, S: BuildHasher + Default> Decode for HashMap<K, V, S> {
	fn decode<R: BufRead>(r: &mut Reader<R>) -> Result<Self, DecodeError> {
		let mut res = HashMap::default();

		while !r.read_terminal()? {
			let k = K::decode(r)?;
			let v = V::decode(r)?;
			res.insert(k, v);
		}

		Ok(res)
	}
}

impl<K: Decode + Ord, V: Decode> Decode for BTreeMap<K, V> {
	fn decode<R: BufRead>(r: &mut Reader<R>) -> Result<Self, DecodeError> {
		let mut res = BTreeMap::default();

		while !r.read_terminal()? {
			let k = K::decode(r)?;
			let v = V::decode(r)?;
			res.insert(k, v);
		}

		Ok(res)
	}
}

impl<T: Decode + Sized, const SIZE: usize> Decode for [T; SIZE] {
	fn decode<R: BufRead>(r: &mut Reader<R>) -> Result<Self, DecodeError> {
		let mut res: MaybeUninit<[T; SIZE]> = MaybeUninit::uninit();
		// dropper to properly clean up after a possible panics.
		//
		// Holds onto the mutable reference and the init count so it can drop all initialized
		// entries when if the function quits early.
		struct Dropper<'a, T, const SIZE: usize>(usize, &'a mut [MaybeUninit<T>; SIZE]);
		impl<T, const SIZE: usize> Drop for Dropper<'_, T, SIZE> {
			fn drop(&mut self) {
				for i in 0..self.0 {
					unsafe { self.1[i].assume_init_drop() }
				}
			}
		}

		// safety: Transmute is safe because the MaybeUninit<[T; S]> has the same representation as
		// [MaybeUninit<T>; S]
		let mut dropper = Dropper::<T, SIZE>(0, unsafe {
			std::mem::transmute::<&mut MaybeUninit<[T; SIZE]>, &mut [MaybeUninit<T>; SIZE]>(
				&mut res,
			)
		});

		while dropper.0 < SIZE {
			dropper.1[dropper.0] = MaybeUninit::new(T::decode(r)?);
			dropper.0 += 1;
		}

		// We have successfully initialized the array so new we forget the dropper so it won't
		// unitialize the fields.
		std::mem::forget(dropper);

		// safety: All fields are now initialized.
		unsafe { Ok(res.assume_init()) }
	}
}

macro_rules! impl_decode_tuple {
    ($($t:ident),*$(,)?) => {
		impl <$($t: Decode),*> Decode for ($($t,)*){
			#[allow(non_snake_case)]
			fn decode<R: BufRead>(_r: &mut Reader<R>) -> Result<Self, DecodeError> {
				$(let $t = $t::decode(_r)?;)*

				Ok((
					$($t,)*
				))
			}
		}

    };
}

impl_decode_tuple!();
impl_decode_tuple!(A);
impl_decode_tuple!(A, B);
impl_decode_tuple!(A, B, C);
impl_decode_tuple!(A, B, C, D);
impl_decode_tuple!(A, B, C, D, E);
impl_decode_tuple!(A, B, C, D, E, F);

macro_rules! impl_decode_prim {
	($ty:ident,$name:ident) => {
		impl Decode for $ty {
			fn decode<R: BufRead>(r: &mut Reader<R>) -> Result<Self, DecodeError> {
				r.$name()
			}
		}
	};
}

impl_decode_prim!(u8, read_u8);
impl_decode_prim!(i8, read_i8);
impl_decode_prim!(u16, read_u16);
impl_decode_prim!(i16, read_i16);
impl_decode_prim!(u32, read_u32);
impl_decode_prim!(i32, read_i32);
impl_decode_prim!(u64, read_u64);
impl_decode_prim!(i64, read_i64);
impl_decode_prim!(u128, read_u128);
impl_decode_prim!(i128, read_i128);
impl_decode_prim!(f32, read_f32);
impl_decode_prim!(f64, read_f64);

impl<'de> BorrowDecode<'de> for bool {
	fn borrow_decode(r: &mut BorrowReader<'de>) -> Result<Self, DecodeError> {
		match r.read_u8()? {
			2 => Ok(false),
			3 => Ok(true),
			_ => Err(DecodeError::InvalidFormat),
		}
	}
}

impl<'de> BorrowDecode<'de> for char {
	fn borrow_decode(r: &mut BorrowReader<'de>) -> Result<Self, DecodeError> {
		char::from_u32(r.read_u32()?).ok_or(DecodeError::InvalidFormat)
	}
}

impl<'de> BorrowDecode<'de> for String {
	fn borrow_decode(r: &mut BorrowReader<'de>) -> Result<Self, DecodeError> {
		r.read_string()
	}
}

impl<'de, D: BorrowDecode<'de>> BorrowDecode<'de> for Option<D> {
	fn borrow_decode(r: &mut BorrowReader<'de>) -> Result<Self, DecodeError> {
		match r.read_u8()? {
			2 => Ok(None),
			3 => Ok(Some(D::borrow_decode(r)?)),
			_ => Err(DecodeError::InvalidFormat),
		}
	}
}

impl<'de, O: BorrowDecode<'de>, E: BorrowDecode<'de>> BorrowDecode<'de> for Result<O, E> {
	fn borrow_decode(r: &mut BorrowReader<'de>) -> Result<Self, DecodeError> {
		match r.read_u8()? {
			2 => Ok(Ok(O::borrow_decode(r)?)),
			3 => Ok(Err(E::borrow_decode(r)?)),
			_ => Err(DecodeError::InvalidFormat),
		}
	}
}

impl<'de> BorrowDecode<'de> for Cow<'de, [u8]> {
	fn borrow_decode(r: &mut BorrowReader<'de>) -> Result<Self, DecodeError> {
		r.read_cow()
	}
}

impl<'de> BorrowDecode<'de> for Cow<'de, str> {
	fn borrow_decode(r: &mut BorrowReader<'de>) -> Result<Self, DecodeError> {
		r.read_str_cow()
	}
}

impl<'de, D: BorrowDecode<'de>> BorrowDecode<'de> for Cow<'de, D>
where
	D: ToOwned<Owned = D> + BorrowDecode<'de>,
{
	fn borrow_decode(r: &mut BorrowReader<'de>) -> Result<Self, DecodeError> {
		Ok(Cow::Owned(D::borrow_decode(r)?))
	}
}

impl<'de> BorrowDecode<'de> for Duration {
	fn borrow_decode(r: &mut BorrowReader<'de>) -> Result<Self, DecodeError> {
		let secs = r.read_u64()?;
		let subsec_nanos = r.read_u32()?;
		if subsec_nanos > 999_999_999 {
			return Err(DecodeError::message("Duration subsec nanoseconds was out of range"));
		}
		Ok(Duration::new(secs, subsec_nanos))
	}
}

impl<'de, D: BorrowDecode<'de>> BorrowDecode<'de> for Box<D> {
	fn borrow_decode(r: &mut BorrowReader<'de>) -> Result<Self, DecodeError> {
		Ok(Box::new(D::borrow_decode(r)?))
	}
}

impl<'de, D: BorrowDecode<'de>> BorrowDecode<'de> for Vec<D> {
	fn borrow_decode(r: &mut BorrowReader<'de>) -> Result<Self, DecodeError> {
		// TODO: Castaway optimize Vec<u8>?
		let mut buffer = Vec::new();

		while !r.read_terminal()? {
			buffer.push(D::borrow_decode(r)?);
		}

		Ok(buffer)
	}
}

impl<'de, K: BorrowDecode<'de> + Hash + Eq, V: BorrowDecode<'de>, S: BuildHasher + Default>
	BorrowDecode<'de> for HashMap<K, V, S>
{
	fn borrow_decode(r: &mut BorrowReader<'de>) -> Result<Self, DecodeError> {
		let mut res = HashMap::default();

		while !r.read_terminal()? {
			let k = K::borrow_decode(r)?;
			let v = V::borrow_decode(r)?;
			res.insert(k, v);
		}

		Ok(res)
	}
}

impl<'de, K: BorrowDecode<'de> + Ord, V: BorrowDecode<'de>> BorrowDecode<'de> for BTreeMap<K, V> {
	fn borrow_decode(r: &mut BorrowReader<'de>) -> Result<Self, DecodeError> {
		let mut res = BTreeMap::default();

		while !r.read_terminal()? {
			let k = K::borrow_decode(r)?;
			let v = V::borrow_decode(r)?;
			res.insert(k, v);
		}

		Ok(res)
	}
}

impl<'de, T: BorrowDecode<'de> + Sized, const SIZE: usize> BorrowDecode<'de> for [T; SIZE] {
	fn borrow_decode(r: &mut BorrowReader<'de>) -> Result<Self, DecodeError> {
		// TODO: Castaway optimize [T;SIZE]?
		let mut res: MaybeUninit<[T; SIZE]> = MaybeUninit::uninit();
		// dropper to properly clean up after a possible panics.
		//
		// Holds onto the mutable reference and the init count so it can drop all initialized
		// entries if the function quits early.
		struct Dropper<'a, T, const SIZE: usize>(usize, &'a mut [MaybeUninit<T>; SIZE]);
		impl<T, const SIZE: usize> Drop for Dropper<'_, T, SIZE> {
			fn drop(&mut self) {
				for i in 0..self.0 {
					unsafe { self.1[i].assume_init_drop() }
				}
			}
		}

		// safety: Transmute is safe because the MaybeUninit<[T; S]> has the same representation as
		// [MaybeUninit<T>; S]
		let mut dropper = Dropper::<T, SIZE>(0, unsafe {
			std::mem::transmute::<&mut MaybeUninit<[T; SIZE]>, &mut [MaybeUninit<T>; SIZE]>(
				&mut res,
			)
		});

		while dropper.0 < SIZE {
			dropper.1[dropper.0] = MaybeUninit::new(T::borrow_decode(r)?);
			dropper.0 += 1;
		}

		// We have successfully initialized the array so new we forget the dropper so it won't
		// unitialize the fields.
		std::mem::forget(dropper);

		// safety: All fields are now initialized.
		unsafe { Ok(res.assume_init()) }
	}
}

macro_rules! impl_borrow_decode_tuple {
    ($($t:ident),*$(,)?) => {
		impl <'de, $($t: BorrowDecode<'de>),*> BorrowDecode<'de> for ($($t,)*){
			#[allow(non_snake_case)]
			fn borrow_decode(_r: &mut BorrowReader<'de>) -> Result<Self, DecodeError> {
				$(let $t = $t::borrow_decode(_r)?;)*

				Ok((
					$($t,)*
				))
			}
		}

    };
}

impl_borrow_decode_tuple!();
impl_borrow_decode_tuple!(A);
impl_borrow_decode_tuple!(A, B);
impl_borrow_decode_tuple!(A, B, C);
impl_borrow_decode_tuple!(A, B, C, D);
impl_borrow_decode_tuple!(A, B, C, D, E);
impl_borrow_decode_tuple!(A, B, C, D, E, F);

macro_rules! impl_borrow_decode_prim {
	($ty:ident,$name:ident) => {
		impl<'de> BorrowDecode<'de> for $ty {
			fn borrow_decode(r: &mut BorrowReader<'de>) -> Result<Self, DecodeError> {
				r.$name()
			}
		}
	};
}

impl_borrow_decode_prim!(u8, read_u8);
impl_borrow_decode_prim!(i8, read_i8);
impl_borrow_decode_prim!(u16, read_u16);
impl_borrow_decode_prim!(i16, read_i16);
impl_borrow_decode_prim!(u32, read_u32);
impl_borrow_decode_prim!(i32, read_i32);
impl_borrow_decode_prim!(u64, read_u64);
impl_borrow_decode_prim!(i64, read_i64);
impl_borrow_decode_prim!(u128, read_u128);
impl_borrow_decode_prim!(i128, read_i128);
impl_borrow_decode_prim!(f32, read_f32);
impl_borrow_decode_prim!(f64, read_f64);
