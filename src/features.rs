#[cfg(feature = "uuid")]
mod uuid {
	use std::io::{BufRead, Write};

	use ::uuid::Uuid;

	use crate::{BorrowDecode, BorrowReader, Decode, Encode, Reader, Result, Writer};

	impl Encode for Uuid {
		fn encode<W: Write>(&self, w: &mut Writer<W>) -> Result<()> {
			w.write_array(*self.as_bytes())
		}
	}

	impl Decode for Uuid {
		fn decode<R: BufRead>(r: &mut Reader<R>) -> Result<Self> {
			Ok(Uuid::from_bytes(r.read_array()?))
		}
	}

	impl<'de> BorrowDecode<'de> for Uuid {
		fn borrow_decode(r: &mut BorrowReader<'de>) -> Result<Self> {
			Ok(Uuid::from_bytes(r.read_array()?))
		}
	}
}
