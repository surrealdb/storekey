#[cfg(feature = "uuid")]
mod uuid {
	use std::io::{BufRead, Write};

	use ::uuid::Uuid;

	use crate::{
		BorrowDecode, BorrowReader, Decode, DecodeError, Encode, EncodeError, Reader, Writer,
	};

	impl<F> Encode<F> for Uuid {
		fn encode<W: Write>(&self, w: &mut Writer<W>) -> Result<(), EncodeError> {
			w.write_array(*self.as_bytes())
		}
	}

	impl<F> Decode<F> for Uuid {
		fn decode<R: BufRead>(r: &mut Reader<R>) -> Result<Self, DecodeError> {
			Ok(Uuid::from_bytes(r.read_array()?))
		}
	}

	impl<'de, F> BorrowDecode<'de, F> for Uuid {
		fn borrow_decode(r: &mut BorrowReader<'de>) -> Result<Self, DecodeError> {
			Ok(Uuid::from_bytes(r.read_array()?))
		}
	}
}
