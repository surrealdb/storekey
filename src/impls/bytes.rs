#![cfg(feature = "bytes")]

use std::io::{BufRead, Write};

use ::bytes::Bytes;

use crate::{BorrowDecode, BorrowReader, Decode, DecodeError, Encode, EncodeError, Reader, Writer};

impl<F> Encode<F> for Bytes {
	fn encode<W: Write>(&self, w: &mut Writer<W>) -> Result<(), EncodeError> {
		w.write_slice(self.as_ref())
	}
}

impl<F> Decode<F> for Bytes {
	fn decode<R: BufRead>(r: &mut Reader<R>) -> Result<Self, DecodeError> {
		Ok(Bytes::from(r.read_vec()?))
	}
}

impl<'de, F> BorrowDecode<'de, F> for Bytes {
	fn borrow_decode(r: &mut BorrowReader<'de>) -> Result<Self, DecodeError> {
		Ok(Bytes::from(r.read_vec()?))
	}
}
