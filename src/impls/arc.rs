use std::io::{BufRead, Write};
use std::sync::Arc;

use crate::{BorrowDecode, BorrowReader, Decode, DecodeError, Encode, EncodeError, Reader, Writer};

impl<F, E: Encode<F>> Encode<F> for Arc<E> {
	fn encode<W: Write>(&self, w: &mut Writer<W>) -> Result<(), EncodeError> {
		self.as_ref().encode(w)
	}
}

impl<F, D: Decode<F>> Decode<F> for Arc<D> {
	fn decode<R: BufRead>(r: &mut Reader<R>) -> Result<Self, DecodeError> {
		Ok(Arc::new(D::decode(r)?))
	}
}

impl<'de, F, D: BorrowDecode<'de, F>> BorrowDecode<'de, F> for Arc<D> {
	fn borrow_decode(r: &mut BorrowReader<'de>) -> Result<Self, DecodeError> {
		Ok(Arc::new(D::borrow_decode(r)?))
	}
}

// Specialized implementation for Arc<str>
impl<F> Decode<F> for Arc<str> {
	fn decode<R: BufRead>(r: &mut Reader<R>) -> Result<Self, DecodeError> {
		Ok(Arc::from(<String as Decode<F>>::decode(r)?))
	}
}

impl<'de, F> BorrowDecode<'de, F> for Arc<str> {
	fn borrow_decode(r: &mut BorrowReader<'de>) -> Result<Self, DecodeError> {
		Ok(Arc::from(<String as BorrowDecode<'de, F>>::borrow_decode(r)?))
	}
}

// Specialized implementation for Arc<[u8]>
impl<F> Decode<F> for Arc<[u8]> {
	fn decode<R: BufRead>(r: &mut Reader<R>) -> Result<Self, DecodeError> {
		Ok(Arc::from(<Vec<u8> as Decode<F>>::decode(r)?))
	}
}

impl<'de, F> BorrowDecode<'de, F> for Arc<[u8]> {
	fn borrow_decode(r: &mut BorrowReader<'de>) -> Result<Self, DecodeError> {
		Ok(Arc::from(<Vec<u8> as BorrowDecode<'de, F>>::borrow_decode(r)?))
	}
}
