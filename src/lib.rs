use std::error::Error;
use std::fmt;
use std::io::{self, BufRead, Write};

#[cfg(feature = "derive")]
pub use storekey_derive::{BorrowDecode, Decode, Encode, ToEscaped};

mod decode;
mod encode;
mod impls;
mod reader;
mod to_escaped;
mod types;
mod writer;

#[cfg(test)]
mod test;

pub use reader::{BorrowReader, Reader};
pub use to_escaped::ToEscaped;
pub use types::{EscapedChars, EscapedIter, EscapedSlice, EscapedStr};
pub use writer::Writer;

#[derive(Debug)]
pub struct MessageError(pub String);

impl fmt::Display for MessageError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		self.0.fmt(f)
	}
}
impl std::error::Error for MessageError {}

#[derive(Debug)]
pub enum EncodeError {
	Io(io::Error),
	Custom(Box<dyn Error + Send + Sync>),
}

impl EncodeError {
	/// Utility function to turn any error into a custom error.
	pub fn custom<E: Error + Send + Sync + 'static>(e: E) -> Self {
		EncodeError::Custom(Box::new(e))
	}

	pub fn message<S: fmt::Display>(s: S) -> Self {
		Self::custom(MessageError(s.to_string()))
	}
}

impl fmt::Display for EncodeError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			EncodeError::Io(error) => write!(f, "Io Error: {error}"),
			EncodeError::Custom(x) => {
				write!(f, "{x}")
			}
		}
	}
}
impl std::error::Error for EncodeError {}
impl From<io::Error> for EncodeError {
	fn from(value: io::Error) -> Self {
		EncodeError::Io(value)
	}
}

#[derive(Debug)]
pub enum DecodeError {
	Io(io::Error),
	UnexpectedEnd,
	BytesRemaining,
	InvalidFormat,
	Utf8,
	Custom(Box<dyn Error + Send + Sync>),
}

impl DecodeError {
	/// Utility function to turn any error into a custom error.
	pub fn custom<E: Error + Send + Sync + 'static>(e: E) -> Self {
		DecodeError::Custom(Box::new(e))
	}

	pub fn message<S: fmt::Display>(s: S) -> Self {
		Self::custom(MessageError(s.to_string()))
	}
}

impl fmt::Display for DecodeError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			DecodeError::Io(error) => write!(f, "Io Error: {error}"),
			DecodeError::UnexpectedEnd => {
				write!(f, "Reader did not have enough data to properly decode type")
			}
			DecodeError::BytesRemaining => {
				write!(f, "Reader had data remaining after type was fully decoded.")
			}
			DecodeError::InvalidFormat => {
				write!(
					f,
					"Found an invalid byte sequence which could not be deserialized properly."
				)
			}
			DecodeError::Utf8 => write!(f, "Could not decode string due to invalid utf8"),
			DecodeError::Custom(x) => {
				write!(f, "{x}")
			}
		}
	}
}
impl std::error::Error for DecodeError {}

impl From<io::Error> for DecodeError {
	fn from(value: io::Error) -> Self {
		DecodeError::Io(value)
	}
}

/// Types which can be encoded to storekey format.
pub trait Encode {
	fn encode<W: Write>(&self, w: &mut Writer<W>) -> Result<(), EncodeError>;
}

/// Types which can be decoded from storekey format with owned ownership.
pub trait Decode: Sized {
	fn decode<R: BufRead>(r: &mut Reader<R>) -> Result<Self, DecodeError>;
}

/// Types which can be decoded from storekey format with borrowed ownership.
pub trait BorrowDecode<'de>: Sized {
	fn borrow_decode(r: &mut BorrowReader<'de>) -> Result<Self, DecodeError>;
}

pub fn encode<W: Write, E: Encode + ?Sized>(w: W, e: &E) -> Result<(), EncodeError> {
	let mut writer = Writer::new(w);
	e.encode(&mut writer)
}

pub fn encode_vec<E: Encode + ?Sized>(e: &E) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
	let mut buffer = Vec::new();
	let mut writer = Writer::new(&mut buffer);
	match e.encode(&mut writer) {
		Ok(_) => Ok(buffer),
		// Encoding should only fail on a custom error or an io error, but as this is encoded to vector it should not be
		// able to fail.
		Err(EncodeError::Io(_)) => unreachable!(),
		Err(EncodeError::Custom(x)) => Err(x),
	}
}

pub fn decode<R: BufRead, D: Decode>(r: R) -> Result<D, DecodeError> {
	let mut reader = Reader::new(r);
	let res = D::decode(&mut reader)?;
	if !reader.is_empty()? {
		return Err(DecodeError::BytesRemaining);
	}
	Ok(res)
}

pub fn decode_borrow<'de, D: BorrowDecode<'de>>(r: &'de [u8]) -> Result<D, DecodeError> {
	let mut reader = BorrowReader::new(r);
	let res = D::borrow_decode(&mut reader)?;
	if !reader.is_empty() {
		return Err(DecodeError::BytesRemaining);
	}
	Ok(res)
}
