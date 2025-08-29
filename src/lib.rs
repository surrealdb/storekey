use std::fmt;
use std::io::{self, BufRead, Write};
use std::result::Result as StdResult;

#[cfg(feature = "derive")]
pub use storekey_derive::{BorrowDecode, Decode, Encode, ToEscaped};

mod decode;
mod encode;
mod reader;
mod to_escaped;
mod types;
mod writer;
mod features;

#[cfg(test)]
mod test;

pub use reader::BorrowReader;
pub use reader::Reader;
pub use to_escaped::ToEscaped;
pub use types::{EscapedChars, EscapedIter, EscapedSlice, EscapedStr};
pub use writer::Writer;

pub type Result<T> = StdResult<T, Error>;

#[derive(Debug)]
pub enum Error {
	Io(io::Error),
	UnexpectedEnd,
	BytesRemaining,
	UnexpectedDiscriminant,
	Utf8,
}

impl fmt::Display for Error {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Error::Io(error) => write!(f, "Io Error: {error}"),
			Error::UnexpectedEnd => {
				write!(f, "Reader did not have enough data to properly decode type")
			}
			Error::BytesRemaining => {
				write!(f, "Reader had data remaining after type was fully decoded.")
			}
			Error::UnexpectedDiscriminant => {
				write!(f, "Found an unexpected discriminant when deserializing an enum.")
			}
			Error::Utf8 => write!(f, "Could not decode string due to invalid utf8"),
		}
	}
}
impl std::error::Error for Error {}

impl From<io::Error> for Error {
	fn from(value: io::Error) -> Self {
		Error::Io(value)
	}
}

/// Types which can be encoded to storekey format.
pub trait Encode {
	fn encode<W: Write>(&self, w: &mut Writer<W>) -> Result<()>;
}

/// Types which can be decoded from storekey format with owned ownership.
pub trait Decode: Sized {
	fn decode<R: BufRead>(r: &mut Reader<R>) -> Result<Self>;
}

/// Types which can be decoded from storekey format with borrowed ownership.
pub trait BorrowDecode<'de>: Sized {
	fn borrow_decode(r: &mut BorrowReader<'de>) -> Result<Self>;
}

pub fn encode<W: Write, E: Encode + ?Sized>(w: W, e: &E) -> Result<()> {
	let mut writer = Writer::new(w);
	e.encode(&mut writer)
}

pub fn encode_vec<E: Encode + ?Sized>(e: &E) -> Vec<u8> {
	let mut buffer = Vec::new();
	let mut writer = Writer::new(&mut buffer);
	// Encoding should only fail on io error, but as this is encoded to vector it should not be
	// able to fail.
	e.encode(&mut writer).unwrap();
	buffer
}

pub fn decode<R: BufRead, D: Decode>(r: R) -> Result<D> {
	let mut reader = Reader::new(r);
	let res = D::decode(&mut reader)?;
	if !reader.is_empty()? {
		return Err(Error::BytesRemaining);
	}
	Ok(res)
}

pub fn decode_borrow<'de, D: BorrowDecode<'de>>(r: &'de [u8]) -> Result<D> {
	let mut reader = BorrowReader::new(r);
	let res = D::borrow_decode(&mut reader)?;
	if !reader.is_empty() {
		return Err(Error::BytesRemaining);
	}
	Ok(res)
}
