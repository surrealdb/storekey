use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};
use std::hash::{BuildHasher, Hash};
use std::io::BufRead;
use std::mem::MaybeUninit;
use std::ops::Bound;
use std::time::Duration;

use crate::DecodeError;

use super::reader::BorrowReader;
use super::{BorrowDecode, Decode, Reader};

impl<F> Decode<F> for bool {
	fn decode<R: BufRead>(r: &mut Reader<R>) -> Result<Self, DecodeError> {
		match r.read_u8()? {
			2 => Ok(false),
			3 => Ok(true),
			_ => Err(DecodeError::InvalidFormat),
		}
	}
}

impl<F> Decode<F> for char {
	fn decode<R: BufRead>(r: &mut Reader<R>) -> Result<Self, DecodeError> {
		let c = r.read_u32()?;
		char::from_u32(c).ok_or(DecodeError::InvalidFormat)
	}
}

impl<F> Decode<F> for String {
	fn decode<R: BufRead>(r: &mut Reader<R>) -> Result<Self, DecodeError> {
		r.read_string()
	}
}

impl<F, D: Decode<F>> Decode<F> for Option<D> {
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

impl<F, D: Decode<F>> Decode<F> for Bound<D> {
	fn decode<R: BufRead>(r: &mut Reader<R>) -> Result<Self, DecodeError> {
		match r.read_u8()? {
			2 => Ok(Bound::Unbounded),
			3 => Ok(Bound::Included(Decode::<F>::decode(r)?)),
			4 => Ok(Bound::Excluded(Decode::<F>::decode(r)?)),
			_ => Err(DecodeError::InvalidFormat),
		}
	}
}

impl<'a, F, O> Decode<F> for Cow<'a, O>
where
	O: ToOwned + ?Sized,
	O::Owned: Decode<F>,
{
	fn decode<R: BufRead>(r: &mut Reader<R>) -> Result<Self, DecodeError> {
		Ok(Cow::Owned(O::Owned::decode(r)?))
	}
}

impl<F, O: Decode<F>, E: Decode<F>> Decode<F> for Result<O, E> {
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

impl<F, D: Decode<F>> Decode<F> for Vec<D> {
	fn decode<R: BufRead>(r: &mut Reader<R>) -> Result<Self, DecodeError> {
		// TODO: Castaway optimize Vec<u8>?
		let mut buffer = Vec::new();

		while !r.read_terminal()? {
			buffer.push(D::decode(r)?);
		}

		Ok(buffer)
	}
}

impl<F, D: Decode<F>> Decode<F> for Box<D> {
	fn decode<R: BufRead>(r: &mut Reader<R>) -> Result<Self, DecodeError> {
		Ok(Box::new(D::decode(r)?))
	}
}

impl<F, K: Decode<F> + Hash + Eq, V: Decode<F>, S: BuildHasher + Default> Decode<F>
	for HashMap<K, V, S>
{
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

impl<F, K: Decode<F> + Ord, V: Decode<F>> Decode<F> for BTreeMap<K, V> {
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

impl<F, T: Decode<F> + Sized, const SIZE: usize> Decode<F> for [T; SIZE] {
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
		impl<Format, $($t: Decode<Format>),*> Decode<Format> for ($($t,)*){
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
		impl<F> Decode<F> for $ty {
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

impl<'de, F> BorrowDecode<'de, F> for bool {
	fn borrow_decode(r: &mut BorrowReader<'de>) -> Result<Self, DecodeError> {
		match r.read_u8()? {
			2 => Ok(false),
			3 => Ok(true),
			_ => Err(DecodeError::InvalidFormat),
		}
	}
}

impl<'de, F> BorrowDecode<'de, F> for char {
	fn borrow_decode(r: &mut BorrowReader<'de>) -> Result<Self, DecodeError> {
		char::from_u32(r.read_u32()?).ok_or(DecodeError::InvalidFormat)
	}
}

impl<'de, F> BorrowDecode<'de, F> for String {
	fn borrow_decode(r: &mut BorrowReader<'de>) -> Result<Self, DecodeError> {
		r.read_string()
	}
}

impl<'de, F, D: BorrowDecode<'de, F>> BorrowDecode<'de, F> for Option<D> {
	fn borrow_decode(r: &mut BorrowReader<'de>) -> Result<Self, DecodeError> {
		match r.read_u8()? {
			2 => Ok(None),
			3 => Ok(Some(D::borrow_decode(r)?)),
			_ => Err(DecodeError::InvalidFormat),
		}
	}
}

impl<'de, F, D: BorrowDecode<'de, F>> BorrowDecode<'de, F> for Bound<D> {
	fn borrow_decode(r: &mut BorrowReader<'de>) -> Result<Self, DecodeError> {
		match r.read_u8()? {
			2 => Ok(Bound::Unbounded),
			3 => Ok(Bound::Included(BorrowDecode::<'de, F>::borrow_decode(r)?)),
			4 => Ok(Bound::Excluded(BorrowDecode::<'de, F>::borrow_decode(r)?)),
			_ => Err(DecodeError::InvalidFormat),
		}
	}
}

impl<'de, F, O: BorrowDecode<'de, F>, E: BorrowDecode<'de, F>> BorrowDecode<'de, F>
	for Result<O, E>
{
	fn borrow_decode(r: &mut BorrowReader<'de>) -> Result<Self, DecodeError> {
		match r.read_u8()? {
			2 => Ok(Ok(O::borrow_decode(r)?)),
			3 => Ok(Err(E::borrow_decode(r)?)),
			_ => Err(DecodeError::InvalidFormat),
		}
	}
}

impl<'de, F> BorrowDecode<'de, F> for Cow<'de, [u8]> {
	fn borrow_decode(r: &mut BorrowReader<'de>) -> Result<Self, DecodeError> {
		r.read_cow()
	}
}

impl<'de, F> BorrowDecode<'de, F> for Cow<'de, str> {
	fn borrow_decode(r: &mut BorrowReader<'de>) -> Result<Self, DecodeError> {
		r.read_str_cow()
	}
}

impl<'de, F, D: BorrowDecode<'de, F>> BorrowDecode<'de, F> for Cow<'de, D>
where
	D: ToOwned<Owned = D> + BorrowDecode<'de, F>,
{
	fn borrow_decode(r: &mut BorrowReader<'de>) -> Result<Self, DecodeError> {
		Ok(Cow::Owned(D::borrow_decode(r)?))
	}
}

impl<'de, F> BorrowDecode<'de, F> for Duration {
	fn borrow_decode(r: &mut BorrowReader<'de>) -> Result<Self, DecodeError> {
		let secs = r.read_u64()?;
		let subsec_nanos = r.read_u32()?;
		if subsec_nanos > 999_999_999 {
			return Err(DecodeError::message("Duration subsec nanoseconds was out of range"));
		}
		Ok(Duration::new(secs, subsec_nanos))
	}
}

impl<'de, F, D: BorrowDecode<'de, F>> BorrowDecode<'de, F> for Box<D> {
	fn borrow_decode(r: &mut BorrowReader<'de>) -> Result<Self, DecodeError> {
		Ok(Box::new(D::borrow_decode(r)?))
	}
}

impl<'de, F, D: BorrowDecode<'de, F>> BorrowDecode<'de, F> for Vec<D> {
	fn borrow_decode(r: &mut BorrowReader<'de>) -> Result<Self, DecodeError> {
		// TODO: Castaway optimize Vec<u8>?
		let mut buffer = Vec::new();

		while !r.read_terminal()? {
			buffer.push(D::borrow_decode(r)?);
		}

		Ok(buffer)
	}
}

impl<
		'de,
		F,
		K: BorrowDecode<'de, F> + Hash + Eq,
		V: BorrowDecode<'de, F>,
		S: BuildHasher + Default,
	> BorrowDecode<'de, F> for HashMap<K, V, S>
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

impl<'de, F, K: BorrowDecode<'de, F> + Ord, V: BorrowDecode<'de, F>> BorrowDecode<'de, F>
	for BTreeMap<K, V>
{
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

impl<'de, F, T: BorrowDecode<'de, F> + Sized, const SIZE: usize> BorrowDecode<'de, F>
	for [T; SIZE]
{
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
		impl <'de,Format, $($t: BorrowDecode<'de,Format>),*> BorrowDecode<'de,Format> for ($($t,)*){
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
		impl<'de, F> BorrowDecode<'de, F> for $ty {
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
