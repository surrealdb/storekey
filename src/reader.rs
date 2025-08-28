use std::{borrow::Cow, io::BufRead};

use super::{
	types::{EscapedSlice, EscapedStr},
	Error, Result,
};

pub struct Reader<R> {
	inner: R,
	expect_escaped: bool,
}

macro_rules! impl_prims {
	($ty:ident, $name:ident) => {
		#[inline]
		pub fn $name(&mut self) -> Result<$ty> {
			Ok($ty::from_be_bytes(self.read_array()?))
		}
	};
}

impl<R: BufRead> Reader<R> {
	pub const fn new(r: R) -> Self {
		Reader {
			inner: r,
			expect_escaped: false,
		}
	}

	/// Returns if the reader is empty / contains no more data.
	#[inline]
	pub fn is_empty(&mut self) -> Result<bool> {
		Ok(self.inner.fill_buf()?.is_empty())
	}

	#[inline]
	pub fn read_terminal(&mut self) -> Result<bool> {
		self.expect_escaped = true;
		let buf = dbg!(self.inner.fill_buf())?;
		match buf.get(0) {
			Some(0) => {
				self.inner.consume(1);
				Ok(true)
			}
			Some(_) => Ok(false),
			None => Err(Error::UnexpectedEnd),
		}
	}

	#[inline]
	pub fn read_array<const SIZE: usize>(&mut self) -> Result<[u8; SIZE]> {
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

	#[inline]
	pub fn read_vec(&mut self) -> Result<Vec<u8>> {
		self.expect_escaped = false;
		let mut buffer = Vec::new();

		let mut read_u8 = || -> Result<u8> {
			let mut buffer = [0u8];
			if self.inner.read(&mut buffer)? == 0 {
				return Err(Error::UnexpectedEnd);
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

	#[inline]
	pub fn read_string(&mut self) -> Result<String> {
		let buf = self.read_vec()?;
		String::from_utf8(buf).map_err(|_| Error::Utf8)
	}

	#[inline]
	pub fn read_f32(&mut self) -> Result<f32> {
		let v = self.read_i32()?;
		let t = ((v ^ i32::MIN) >> 31) | i32::MIN;
		Ok(f32::from_bits((v ^ t) as u32))
	}

	#[inline]
	pub fn read_f64(&mut self) -> Result<f64> {
		let v = self.read_i64()?;
		let t = ((v ^ i64::MIN) >> 63) | i64::MIN;
		Ok(f64::from_bits((v ^ t) as u64))
	}

	impl_prims! {i8, read_i8}
	impl_prims! {u8, read_u8}
	impl_prims! {i16,read_i16}
	impl_prims! {u16,read_u16}
	impl_prims! {i32,read_i32}
	impl_prims! {u32,read_u32}
	impl_prims! {i64,read_i64}
	impl_prims! {u64,read_u64}
	impl_prims! {i128,read_i128}
	impl_prims! {u128,read_u128}
}

pub struct BorrowReader<'de> {
	inner: &'de [u8],
	expect_escaped: bool,
}

impl<'de> BorrowReader<'de> {
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

	#[inline]
	pub fn read_terminal(&mut self) -> Result<bool> {
		self.expect_escaped = true;
		let term = self.inner.get(0).ok_or(Error::UnexpectedEnd)?;
		if *term == 0 {
			self.advance(1);
			Ok(true)
		} else {
			Ok(false)
		}
	}

	#[inline]
	pub fn read_array<const SIZE: usize>(&mut self) -> Result<[u8; SIZE]> {
		if self.expect_escaped {
			self.expect_escaped = false;
			if *self.inner.get(0).ok_or(Error::UnexpectedEnd)? == 1 {
				self.advance(1);
			}
		}
		let slice = self.inner.get(..SIZE).ok_or(Error::UnexpectedEnd)?;
		let mut res = [0u8; SIZE];
		res.copy_from_slice(slice);
		self.advance(SIZE);
		Ok(res)
	}

	#[inline]
	fn read_into_vec(&mut self, buffer: &mut Vec<u8>) -> Result<()> {
		self.expect_escaped = false;
		let mut iter = self.inner.iter();
		loop {
			let Some(next) = iter.next().copied() else {
				return Err(Error::UnexpectedEnd);
			};
			if next == 1 {
				let Some(next) = iter.next().copied() else {
					return Err(Error::UnexpectedEnd);
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

	#[inline]
	pub fn read_cow(&mut self) -> Result<Cow<'de, [u8]>> {
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
					buffer.push(*self.inner.get(i + 1).ok_or(Error::UnexpectedEnd)?);
					self.advance(i + 2);
					self.read_into_vec(&mut buffer)?;
					return Ok(Cow::Owned(buffer));
				}
				Some(_) => {}
				None => return Err(Error::UnexpectedEnd),
			}
		}
		unreachable!()
	}

	#[inline]
	pub fn read_vec(&mut self) -> Result<Vec<u8>> {
		self.expect_escaped = false;
		let mut buffer = Vec::new();
		self.read_into_vec(&mut buffer)?;
		Ok(buffer)
	}

	#[inline]
	pub fn read_str_cow(&mut self) -> Result<Cow<'de, str>> {
		match self.read_cow()? {
			Cow::Borrowed(x) => Ok(Cow::Borrowed(str::from_utf8(x).map_err(|_| Error::Utf8)?)),
			Cow::Owned(x) => Ok(Cow::Owned(String::from_utf8(x).map_err(|_| Error::Utf8)?)),
		}
	}

	#[inline]
	pub fn read_string(&mut self) -> Result<String> {
		let buffer = self.read_vec()?;
		String::from_utf8(buffer).map_err(|_| Error::Utf8)
	}

	#[inline]
	pub fn read_escaped_slice(&mut self) -> Result<&'de EscapedSlice> {
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
				None => return Err(Error::UnexpectedEnd),
			}
		}
	}

	#[inline]
	pub fn read_escaped_str(&mut self) -> Result<&'de EscapedStr> {
		let str = str::from_utf8(self.read_escaped_slice()?.as_bytes())
			.map_err(|_| Error::UnexpectedEnd)?;
		Ok(unsafe { EscapedStr::from_str(str) })
	}

	#[inline]
	pub fn read_f32(&mut self) -> Result<f32> {
		let v = self.read_i32()?;
		let t = ((v ^ i32::MIN) >> 31) | i32::MIN;
		Ok(f32::from_bits((v ^ t) as u32))
	}

	#[inline]
	pub fn read_f64(&mut self) -> Result<f64> {
		let v = self.read_i64()?;
		let t = ((v ^ i64::MIN) >> 63) | i64::MIN;
		Ok(f64::from_bits((v ^ t) as u64))
	}

	impl_prims! {i8, read_i8}
	impl_prims! {u8, read_u8}
	impl_prims! {i16,read_i16}
	impl_prims! {u16,read_u16}
	impl_prims! {i32,read_i32}
	impl_prims! {u32,read_u32}
	impl_prims! {i64,read_i64}
	impl_prims! {u64,read_u64}
	impl_prims! {i128,read_i128}
	impl_prims! {u128,read_u128}
}
