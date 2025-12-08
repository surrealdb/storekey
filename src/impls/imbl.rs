#![cfg(feature = "imbl")]

use std::io::{BufRead, Write};

use imbl::{OrdMap, OrdSet, Vector};

use crate::{BorrowDecode, BorrowReader, Decode, DecodeError, Encode, EncodeError, Reader, Writer};

// Vector<T> implementation (similar to Vec<T>)
impl<F, T: Encode<F>> Encode<F> for Vector<T> {
	fn encode<W: Write>(&self, w: &mut Writer<W>) -> Result<(), EncodeError> {
		for e in self.iter() {
			w.mark_terminator();
			e.encode(w)?;
		}
		w.write_terminator()
	}
}

impl<F, T: Decode<F> + Clone> Decode<F> for Vector<T> {
	fn decode<R: BufRead>(r: &mut Reader<R>) -> Result<Self, DecodeError> {
		let mut buffer = Vector::new();

		while !r.read_terminal()? {
			buffer.push_back(T::decode(r)?);
		}

		Ok(buffer)
	}
}

impl<'de, F, T: BorrowDecode<'de, F> + Clone> BorrowDecode<'de, F> for Vector<T> {
	fn borrow_decode(r: &mut BorrowReader<'de>) -> Result<Self, DecodeError> {
		let mut buffer = Vector::new();

		while !r.read_terminal()? {
			buffer.push_back(T::borrow_decode(r)?);
		}

		Ok(buffer)
	}
}

// OrdMap<K, V> implementation (similar to BTreeMap<K, V>)
impl<F, K: Encode<F> + Ord, V: Encode<F>> Encode<F> for OrdMap<K, V> {
	fn encode<W: Write>(&self, w: &mut Writer<W>) -> Result<(), EncodeError> {
		for (k, v) in self.iter() {
			w.mark_terminator();
			k.encode(w)?;
			v.encode(w)?;
		}
		w.write_terminator()
	}
}

impl<F, K: Decode<F> + Ord + Clone, V: Decode<F> + Clone> Decode<F> for OrdMap<K, V> {
	fn decode<R: BufRead>(r: &mut Reader<R>) -> Result<Self, DecodeError> {
		let mut res = OrdMap::new();

		while !r.read_terminal()? {
			let k = K::decode(r)?;
			let v = V::decode(r)?;
			res.insert(k, v);
		}

		Ok(res)
	}
}

impl<'de, F, K: BorrowDecode<'de, F> + Ord + Clone, V: BorrowDecode<'de, F> + Clone>
	BorrowDecode<'de, F> for OrdMap<K, V>
{
	fn borrow_decode(r: &mut BorrowReader<'de>) -> Result<Self, DecodeError> {
		let mut res = OrdMap::new();

		while !r.read_terminal()? {
			let k = K::borrow_decode(r)?;
			let v = V::borrow_decode(r)?;
			res.insert(k, v);
		}

		Ok(res)
	}
}

// OrdSet<T> implementation (similar to BTreeSet<T>)
impl<F, T: Encode<F> + Ord> Encode<F> for OrdSet<T> {
	fn encode<W: Write>(&self, w: &mut Writer<W>) -> Result<(), EncodeError> {
		for item in self.iter() {
			w.mark_terminator();
			item.encode(w)?;
		}
		w.write_terminator()
	}
}

impl<F, T: Decode<F> + Ord + Clone> Decode<F> for OrdSet<T> {
	fn decode<R: BufRead>(r: &mut Reader<R>) -> Result<Self, DecodeError> {
		let mut res = OrdSet::new();

		while !r.read_terminal()? {
			let item = T::decode(r)?;
			res.insert(item);
		}

		Ok(res)
	}
}

impl<'de, F, T: BorrowDecode<'de, F> + Ord + Clone> BorrowDecode<'de, F> for OrdSet<T> {
	fn borrow_decode(r: &mut BorrowReader<'de>) -> Result<Self, DecodeError> {
		let mut res = OrdSet::new();

		while !r.read_terminal()? {
			let item = T::borrow_decode(r)?;
			res.insert(item);
		}

		Ok(res)
	}
}
