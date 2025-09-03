//! Binary serialization library for Rust values which preserves lexicographic sort order. Order-preserving
//! encoding is useful for creating keys for sorted key-value stores with byte string typed keys,
//! such as [leveldb](https://github.com/google/leveldb) and
//! [sled](https://github.com/spacejam/sled).
//!
//! `storekey` is *not* a self-describing format. In other words, Type information is *not*
//! serialized alongside values, and thus the type of serialized data must be known in order to
//! perform deserialization.
//!
//! #### Supported Data Types
//!
//! `storekey` currently supports all Rust primitives, strings, options, structs, enums, vecs, and
//! tuples. See [`Encode`] for details on the serialization format.
//!
//! #### Type Evolution
//!
//! In general, the exact type of a serialized value must be known in order to correctly
//! deserialize it. For structs and enums, the type is effectively frozen once any values of the
//! type have been serialized: changes to the struct or enum will cause deserialization of already
//! serialized values to fail or return incorrect values. The only exception is adding new variants
//! to the end of an existing enum. Enum variants may *not* change type, be removed, or be
//! reordered. All changes to structs, including adding, removing, reordering, or changing the type
//! of a field are forbidden.
//!
//! These restrictions lead to a few best-practices when using `storekey` serialization:
//!
//! * Don't use `storekey` unless you need lexicographic ordering of serialized values! A more
//!   general encoding library such as [Cap'n Proto](https://github.com/dwrensha/capnproto-rust) or
//!   [bincode](https://github.com/TyOverby/binary-encode) will serve you better if this feature is
//!   not necessary.
//!
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
///
/// # Format
///
/// The *storekey* serialization format tries to preserve ordering, before and after serialization.
/// This means that in general when `a < b` then `enc(a) < enc(b)` when encoded with the storekey
/// format.
///
/// This need to preserve ordering means that storekey cannot store lengths of types which have a
/// size only known at runtime like `Vec` and other collections.
///
/// In order to be able to decode a type after encoding storekey needs some way to mark the end of
/// a runtime sized type. We cannot store the length itself as a prefix, as is common with
/// serialization formats, as that would not preserve ordering. Instead storekey uses a zero byte
/// to mark the end of a slice. This also means that the encoding format needs to escape zero bytes
/// if they occur when within the runtime size type. So for example if you have a `Vec<u8>` with
/// values `[0,1]` then the first `0` will be escaped with a `1`, of course this means the the
/// second value also needs to be escaped resulting in the final encoding of `1,0,1,1,0` for the
/// given `Vec`.
///
/// # Implementing Encode.
///
/// Most of the time, when using storekey, you can rely on the derive macros to correctly implement
/// storekey format for you. Encode is also implemented for a bunch of rust collections. However
/// sometimes it is necessary to implement Encode yourself.
///
/// Most of the time implementing [`Encode`] is pretty straight forward. For a simple struct
/// implementing encode can be as simple as calling encode on all it's fields in order.
///
/// ```
/// # use storekey::*;
/// # use std::io::Write;
///
/// struct MyStruct{
///		field_a: u32,
///		field_b: String,
/// }
///
/// impl Encode for MyStruct{
///		fn encode<W: Write>(&self, w: &mut Writer<W>) -> Result<(), EncodeError>{
///			self.field_a.encode(w)?;
///			self.field_b.encode(w)?;
///			Ok(())
///		}
///	}
/// ```
///
/// For enums the generall pattern is to first encode the discriminant and then encode the
/// variant it's fields.
///
/// ```
/// # use storekey::*;
/// # use std::io::Write;
///
/// enum MyEnum{
///		VariantA(u32),
///		VariantB(String),
/// }
///
/// impl Encode for MyEnum{
///		fn encode<W: Write>(&self, w: &mut Writer<W>) -> Result<(), EncodeError>{
///			match self {
///				MyEnum::VariantA(x) => {
///					// One good pattern is to avoid using 0 or 1 as a discriminant as these might need
///					// to be escaped
///					w.write_u8(2)?;
///				    x.encode(w)?;
///				}
///				MyEnum::VariantB(x) => {
///					w.write_u8(3)?;
///				    x.encode(w)?;
///				}
///			}
///			Ok(())
///		}
///	}
/// ```
///
/// Finally for runtime sized types it you need to mark locations where the decoder might expect a
/// terminator byte to happen. Doing so will cause the write to automatically escape a 0 byte if
/// the next encoded byte happens to be one which is required to properly deserialize runtime sized
/// types.
///
///
/// ```
/// # use storekey::*;
/// # use std::io::Write;
///
/// struct MyVec(Vec<u8>);
///
/// impl Encode for MyVec{
///		fn encode<W: Write>(&self, w: &mut Writer<W>) -> Result<(), EncodeError>{
///			for v in self.0.iter(){
///				// Before every entry in the vector it is possible that a terminator might happen
///				// as the vec could have been shorter. So we need to mark these spots so that the
///				// writer knows to escape the null byte.
///				w.mark_terminator();
///				v.encode(w)?;
///			}
///			// We have finished the list so we need to mark the end.
///			w.write_terminator()?;
///			Ok(())
///		}
///	}
/// ```
///
pub trait Encode {
	fn encode<W: Write>(&self, w: &mut Writer<W>) -> Result<(), EncodeError>;
}

/// Types which can be decoded from storekey format with owned ownership.
///
/// Please refer to the documentation of the [`Encode`] trait for an explanation of how the data is
/// encoded.
///
/// # Implementing decode
///
/// Implementing decode is mostly very straight forward.
///
/// ```
/// # use storekey::*;
/// use std::io::BufRead;
///
/// struct MyStruct{
///		field_a: u32,
///		field_b: String,
/// }
///
/// impl Decode for MyStruct{
///		fn decode<R: BufRead>(r: &mut Reader<R>) -> Result<Self, DecodeError>{
///			let field_a = Decode::decode(r)?;
///			let field_b = Decode::decode(r)?;
///			Ok(MyStruct{
///				field_a,
///				field_b
///			})
///		}
///	}
///
/// enum MyEnum{
///		VariantA(u32),
///		VariantB(String),
/// }
///
/// impl Decode for MyEnum{
///		fn decode<R: BufRead>(r: &mut Reader<R>) -> Result<Self, DecodeError>{
///			match r.read_u8()? {
///				// One good pattern is to avoid using 0 or 1 as a discriminant as these might need
///				// to be escaped
///			    2 => Ok(MyEnum::VariantA(Decode::decode(r)?)),
///			    3 => Ok(MyEnum::VariantB(Decode::decode(r)?)),
///			    _ => Err(DecodeError::InvalidFormat)
///			}
///		}
///	}
/// ```
///
/// For runtime size typed it is often best to read values in a while loop using the
/// [`Reader::read_terminal`] method.
///
/// ```
/// # use storekey::*;
/// use std::io::BufRead;
///
/// struct MyVec(Vec<u8>);
///
/// impl Decode for MyVec{
///		fn decode<R: BufRead>(r: &mut Reader<R>) -> Result<Self, DecodeError>{
///			let mut res = Vec::new();
///			while r.read_terminal()? {
///				res.push(Decode::decode(r)?);
///			}
///			Ok(MyVec(res))
///		}
///	}
/// ```
pub trait Decode: Sized {
	fn decode<R: BufRead>(r: &mut Reader<R>) -> Result<Self, DecodeError>;
}

/// Types which can be decoded from storekey format with borrowed ownership.
///
/// This trait is very similar to the [`Decode`] trait with the exception that this trait allows
/// for zero-copy deserialization. Allowing the deserialization of the escaped variants of [`str`]
/// [`EscapedStr`] and `[u8]` [`EscapedSlice`] as well as deserializing `Cow<str>` and
/// `Cow<[u8]>` borrowing directly from the reader if possible.
pub trait BorrowDecode<'de>: Sized {
	fn borrow_decode(r: &mut BorrowReader<'de>) -> Result<Self, DecodeError>;
}

/// Encode an encodable type into a type which implements [`std::io::Write`].
pub fn encode<W: Write, E: Encode + ?Sized>(w: W, e: &E) -> Result<(), EncodeError> {
	let mut writer = Writer::new(w);
	e.encode(&mut writer)
}

/// Encode an encodable type into a vector.
///
/// Writing into a vector cannot cause an IO error and therefore this method returns only custom
/// errors raised via the [`EncodeError::Custom`] variant.
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

/// Decode an decodable type from a type which implements [`std::io::BufRead`].
pub fn decode<R: BufRead, D: Decode>(r: R) -> Result<D, DecodeError> {
	let mut reader = Reader::new(r);
	let res = D::decode(&mut reader)?;
	if !reader.is_empty()? {
		return Err(DecodeError::BytesRemaining);
	}
	Ok(res)
}

/// Decode a decodable type by borrowing from the given slice.
pub fn decode_borrow<'de, D: BorrowDecode<'de>>(r: &'de [u8]) -> Result<D, DecodeError> {
	let mut reader = BorrowReader::new(r);
	let res = D::borrow_decode(&mut reader)?;
	if !reader.is_empty() {
		return Err(DecodeError::BytesRemaining);
	}
	Ok(res)
}
