use std::borrow::Cow;
use std::io::BufRead;

use super::types::{EscapedSlice, EscapedStr};
use super::DecodeError;

/// Struct used in [`storekey::Decode`] for reading types from the buffer.
///
/// This type handles unescaping bytes the buffer mostly transparently.
/// It has an internal flag for marking when an escaped byte can be read from the buffer.
/// Reading from the buffer in any way unmarks this flag.
pub struct Reader<R> {
	inner: R,
	expect_escaped: bool,
}

macro_rules! impl_prims {
	(signed $ty:ident, $name:ident) => {
		#[inline]
		pub fn $name(&mut self) -> Result<$ty, DecodeError> {
			Ok($ty::from_be_bytes(self.read_array()?) ^ $ty::MIN)
		}
	};
	($ty:ident, $name:ident) => {
		#[inline]
		pub fn $name(&mut self) -> Result<$ty, DecodeError> {
			Ok($ty::from_be_bytes(self.read_array()?))
		}
	};
}

impl<R: BufRead> Reader<R> {
	/// Create a new reader.
	pub const fn new(r: R) -> Self {
		Reader {
			inner: r,
			expect_escaped: false,
		}
	}

	/// Returns if the reader is empty / contains no more data.
	#[inline]
	pub fn is_empty(&mut self) -> Result<bool, DecodeError> {
		Ok(self.inner.fill_buf()?.is_empty())
	}

	/// Mark the next byte as possibly containing an escaped bytes.
	#[inline]
	pub fn expect_escaped(&mut self) {
		self.expect_escaped = true;
	}

	/// Try to read a terminator byte if there is one.
	///
	/// Returns true if the next byte is a terminator, otherwise returns false and the reader does
	/// not advance.
	///
	/// Sets the `expect_escaped` flag marking the next byte as being possibly escaped.
	#[inline]
	pub fn read_terminal(&mut self) -> Result<bool, DecodeError> {
		self.expect_escaped = true;
		let buf = self.inner.fill_buf()?;
		match buf.first() {
			Some(0) => {
				self.inner.consume(1);
				Ok(true)
			}
			Some(_) => Ok(false),
			None => Err(DecodeError::UnexpectedEnd),
		}
	}

	/// Reads an fixed size array of u8 from the reader, unescaping possible escaped bytes.
	///
	/// All other `read_*` functions of `Reader` which read a fixed size type call this function to
	/// read a certain amount of bytes from the reader.
	///
	///	This type does not expect a null terminator after the end of the array as it is reading a
	///	fixed size type.
	///
	///	Calling this function unsets the expected escape flag before returning.
	#[inline]
	pub fn read_array<const SIZE: usize>(&mut self) -> Result<[u8; SIZE], DecodeError> {
		const { assert!(SIZE > 0, "read_array should at minimum read a single byte") };
		if self.expect_escaped {
			self.expect_escaped = false;
			let mut buffer = [0];
			self.inner.read_exact(&mut buffer[..])?;
			if buffer[0] != 1 {
				let mut res = [0u8; SIZE];
				self.inner.read_exact(&mut res[1..])?;
				res[0] = buffer[0];
				return Ok(res);
			}
		}

		let mut res = [0u8; SIZE];
		self.inner.read_exact(&mut res[..])?;
		Ok(res)
	}

	/// Reads a runtime sized `Vec<u8>` from the reader, expected the sequence of bytes to be
	/// ended by a terminal zero byte.
	///
	///	Calling this function unsets the expected escape flag before returning.
	#[inline]
	pub fn read_vec(&mut self) -> Result<Vec<u8>, DecodeError> {
		self.expect_escaped = false;
		let mut buffer = Vec::new();

		let mut read_u8 = || -> Result<u8, DecodeError> {
			let mut buffer = [0u8];
			if self.inner.read(&mut buffer)? == 0 {
				return Err(DecodeError::UnexpectedEnd);
			};
			Ok(buffer[0])
		};

		loop {
			let next = read_u8()?;
			if next == 1 {
				let next = read_u8()?;
				buffer.push(next);
				continue;
			}
			if next == 0 {
				break;
			}
			buffer.push(next)
		}
		Ok(buffer)
	}

	/// Reads a runtime sized `String` from the reader, expected the sequence of bytes to be
	/// ended by a terminal zero byte.
	///
	///	Calling this function unsets the expected escape flag before returning.
	#[inline]
	pub fn read_string(&mut self) -> Result<String, DecodeError> {
		let buf = self.read_vec()?;
		String::from_utf8(buf).map_err(|_| DecodeError::Utf8)
	}

	#[inline]
	pub fn read_f32(&mut self) -> Result<f32, DecodeError> {
		let v = self.read_u32()? as i32;
		let t = ((v ^ i32::MIN) >> 31) | i32::MIN;
		Ok(f32::from_bits((v ^ t) as u32))
	}

	#[inline]
	pub fn read_f64(&mut self) -> Result<f64, DecodeError> {
		let v = self.read_u64()? as i64;
		let t = ((v ^ i64::MIN) >> 63) | i64::MIN;
		Ok(f64::from_bits((v ^ t) as u64))
	}

	impl_prims! {signed i8, read_i8}
	impl_prims! {u8, read_u8}
	impl_prims! {signed i16,read_i16}
	impl_prims! {u16,read_u16}
	impl_prims! {signed i32,read_i32}
	impl_prims! {u32,read_u32}
	impl_prims! {signed i64,read_i64}
	impl_prims! {u64,read_u64}
	impl_prims! {signed i128,read_i128}
	impl_prims! {u128,read_u128}
}

/// Struct used in [`storekey::BorrowDecode`] for reading types from the buffer.
///
/// This type handles unescaping bytes the buffer mostly transparently.
/// It has an internal flag for marking when an escaped byte can be read from the buffer.
/// Reading from the buffer in any way unmarks this flag.
pub struct BorrowReader<'de> {
	inner: &'de [u8],
	expect_escaped: bool,
}

impl<'de> BorrowReader<'de> {
	/// Create a new reader.
	pub const fn new(slice: &'de [u8]) -> Self {
		BorrowReader {
			inner: slice,
			expect_escaped: false,
		}
	}

	#[inline]
	pub fn is_empty(&self) -> bool {
		self.inner.is_empty()
	}

	#[inline]
	fn advance(&mut self, s: usize) {
		self.inner = &self.inner[s..];
	}

	/// Mark the next byte as possibly containing an escaped bytes.
	#[inline]
	pub fn expect_escaped(&mut self) {
		self.expect_escaped = true;
	}

	/// Try to read a terminator byte if there is one.
	///
	/// Returns true if the next byte is a terminator, otherwise returns false and the reader does
	/// not advance.
	///
	/// Sets the `expect_escaped` flag marking the next byte as being possibly escaped.
	#[inline]
	pub fn read_terminal(&mut self) -> Result<bool, DecodeError> {
		self.expect_escaped = true;
		let term = self.inner.first().ok_or(DecodeError::UnexpectedEnd)?;
		if *term == 0 {
			self.advance(1);
			Ok(true)
		} else {
			Ok(false)
		}
	}

	/// Reads an fixed size array of u8 from the reader, unescaping possible escaped bytes.
	///
	/// All other `read_*` functions of `Reader` which read a fixed size type call this function to
	/// read a certain amount of bytes from the reader.
	///
	///	This type does not expect a null terminator after the end of the array as it is reading a
	///	fixed size type.
	///
	///	Calling this function unsets the expected escape flag before returning.
	#[inline]
	pub fn read_array<const SIZE: usize>(&mut self) -> Result<[u8; SIZE], DecodeError> {
		if self.expect_escaped {
			self.expect_escaped = false;
			if *self.inner.first().ok_or(DecodeError::UnexpectedEnd)? == 1 {
				self.advance(1);
			}
		}
		let slice = self.inner.get(..SIZE).ok_or(DecodeError::UnexpectedEnd)?;
		let mut res = [0u8; SIZE];
		res.copy_from_slice(slice);
		self.advance(SIZE);
		Ok(res)
	}

	#[inline]
	fn read_into_vec(&mut self, buffer: &mut Vec<u8>) -> Result<(), DecodeError> {
		self.expect_escaped = false;
		let mut iter = self.inner.iter();
		loop {
			let Some(next) = iter.next().copied() else {
				return Err(DecodeError::UnexpectedEnd);
			};
			if next == 1 {
				let Some(next) = iter.next().copied() else {
					return Err(DecodeError::UnexpectedEnd);
				};
				buffer.push(next);
				continue;
			}
			if next == 0 {
				break;
			}
			buffer.push(next)
		}
		self.inner = iter.as_slice();
		Ok(())
	}

	/// Reads a runtime sized `Cow<[u8]>` from the reader, expected the sequence of bytes to be
	/// ended by a terminal zero byte.
	///
	/// If the string encoded in the buffer does not contain escaped characters this function will
	/// return a `Cow::Borrowed`.
	///
	///	Calling this function unsets the expected escape flag before returning.
	#[inline]
	pub fn read_cow(&mut self) -> Result<Cow<'de, [u8]>, DecodeError> {
		self.expect_escaped = false;
		for i in 0.. {
			match self.inner.get(i) {
				Some(0) => {
					// hit the end without encountering a escape character so the slice can be
					// borrowed.
					let slice = &self.inner[..i];
					self.advance(i + 1);
					return Ok(Cow::Borrowed(slice));
				}
				Some(1) => {
					// Hit an escape character so we need to create a buffer.
					let mut buffer = self.inner[..i].to_vec();
					buffer.push(*self.inner.get(i + 1).ok_or(DecodeError::UnexpectedEnd)?);
					self.advance(i + 2);
					self.read_into_vec(&mut buffer)?;
					return Ok(Cow::Owned(buffer));
				}
				Some(_) => {}
				None => return Err(DecodeError::UnexpectedEnd),
			}
		}
		unreachable!()
	}

	/// Reads a runtime sized `Vec<u8>` from the reader, expected the sequence of bytes to be
	/// ended by a terminal zero byte.
	///
	///	Calling this function unsets the expected escape flag before returning.
	#[inline]
	pub fn read_vec(&mut self) -> Result<Vec<u8>, DecodeError> {
		self.expect_escaped = false;
		let mut buffer = Vec::new();
		self.read_into_vec(&mut buffer)?;
		Ok(buffer)
	}

	/// Reads a runtime sized `Cow<str>` from the reader, expected the sequence of bytes to be
	/// ended by a terminal zero byte.
	///
	/// If the string encoded in the buffer does not contain escaped characters this function will
	/// return a `Cow::Borrowed`.
	///
	///	Calling this function unsets the expected escape flag before returning.
	#[inline]
	pub fn read_str_cow(&mut self) -> Result<Cow<'de, str>, DecodeError> {
		match self.read_cow()? {
			Cow::Borrowed(x) => {
				Ok(Cow::Borrowed(str::from_utf8(x).map_err(|_| DecodeError::Utf8)?))
			}
			Cow::Owned(x) => Ok(Cow::Owned(String::from_utf8(x).map_err(|_| DecodeError::Utf8)?)),
		}
	}

	/// Reads a runtime sized `String` from the reader, expected the sequence of bytes to be
	/// ended by a terminal zero byte.
	///
	///	Calling this function unsets the expected escape flag before returning.
	#[inline]
	pub fn read_string(&mut self) -> Result<String, DecodeError> {
		let buffer = self.read_vec()?;
		String::from_utf8(buffer).map_err(|_| DecodeError::Utf8)
	}

	/// Reads an escaped slice from the reader, expecting the sequence of bytes to be ended by a
	/// terminal zero byte.
	///
	/// This function never allocates and always returns a borrowed value.
	///
	///	Calling this function unsets the expected escape flag before returning.
	#[inline]
	pub fn read_escaped_slice(&mut self) -> Result<&'de EscapedSlice, DecodeError> {
		self.expect_escaped = false;
		let mut i = 0;
		loop {
			match self.inner.get(i) {
				Some(0) => {
					let res = unsafe { EscapedSlice::from_slice(&self.inner[..i + 1]) };
					self.advance(i + 1);
					return Ok(res);
				}
				Some(1) => {
					i += 2;
				}
				Some(_) => {
					i += 1;
				}
				None => return Err(DecodeError::UnexpectedEnd),
			}
		}
	}

	/// Reads an escaped str from the reader, expecting the sequence of bytes to be ended by a
	/// terminal zero byte.
	///
	/// This function never allocates and always returns a borrowed value.
	///
	///	Calling this function unsets the expected escape flag before returning.
	#[inline]
	pub fn read_escaped_str(&mut self) -> Result<&'de EscapedStr, DecodeError> {
		let str = str::from_utf8(self.read_escaped_slice()?.as_bytes())
			.map_err(|_| DecodeError::UnexpectedEnd)?;
		Ok(unsafe { EscapedStr::from_str(str) })
	}

	#[inline]
	pub fn read_f32(&mut self) -> Result<f32, DecodeError> {
		let v = self.read_u32()? as i32;
		let t = ((v ^ i32::MIN) >> 31) | i32::MIN;
		Ok(f32::from_bits((v ^ t) as u32))
	}

	#[inline]
	pub fn read_f64(&mut self) -> Result<f64, DecodeError> {
		let v = self.read_u64()? as i64;
		let t = ((v ^ i64::MIN) >> 63) | i64::MIN;
		Ok(f64::from_bits((v ^ t) as u64))
	}

	impl_prims! {signed i8, read_i8}
	impl_prims! {u8, read_u8}
	impl_prims! {signed i16,read_i16}
	impl_prims! {u16,read_u16}
	impl_prims! {signed i32,read_i32}
	impl_prims! {u32,read_u32}
	impl_prims! {signed i64,read_i64}
	impl_prims! {u64,read_u64}
	impl_prims! {signed i128,read_i128}
	impl_prims! {u128,read_u128}
}
