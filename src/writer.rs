use std::io::Write;

use super::types::EscapedSlice;
use super::EncodeError;

/// The writer type used in [`storekey::Encode`].
///
/// Handles writing into the encoding buffer and escaping bytes if needed.
///
/// Will only escape bytes where they might conflict with a terminal zero byte.
/// To do have this function correctly you need to call [`Writer::mark_terminator`] function where
/// appropriate.
#[derive(Debug)]
pub struct Writer<W: Write> {
	inner: W,
	escape_zero: bool,
}

macro_rules! impl_prims {
	(signed $ty:ident, $name:ident) => {
		#[inline]
		pub fn $name(&mut self, v: $ty) -> Result<(), EncodeError> {
			self.write_array((v ^ $ty::MIN).to_be_bytes())
		}
	};
	($ty:ident, $name:ident) => {
		#[inline]
		pub fn $name(&mut self, v: $ty) -> Result<(), EncodeError> {
			self.write_array(v.to_be_bytes())
		}
	};
}

impl<W: Write> Writer<W> {
	pub const fn new(w: W) -> Self {
		Writer {
			inner: w,
			escape_zero: false,
		}
	}

	/// Marks a position where a null byte is used as a terminator.
	///
	/// Should be called if when decoding a zero byte as the next character would result in the
	/// serialize ending prematurely.
	pub fn mark_terminator(&mut self) {
		self.escape_zero = true;
	}

	/// Write an already escaped slice.
	pub fn write_escaped_slice(&mut self, slice: &EscapedSlice) -> Result<(), EncodeError> {
		self.escape_zero = false;
		self.inner.write_all(slice.as_bytes())?;
		Ok(())
	}

	/// Writes an runtime sized slice, escaping null bytes and ending the slice with a terminal
	/// zero byte.
	#[inline]
	pub fn write_slice(&mut self, slice: &[u8]) -> Result<(), EncodeError> {
		self.escape_zero = false;
		for b in slice {
			if *b <= 1 {
				self.inner.write_all(&[1])?;
			}
			self.inner.write_all(&[*b])?;
		}
		self.inner.write_all(&[0])?;
		Ok(())
	}

	/// Write a fixed size array into the buffer.
	///
	/// As it is fixed size it will not write a terminal zero byte after the end.
	///
	/// This function will escape the first byte of the array if needed.
	/// All other `write_*` functions which write fixed sized types call this function.
	#[inline]
	pub fn write_array<const LEN: usize>(&mut self, array: [u8; LEN]) -> Result<(), EncodeError> {
		if LEN == 0 {
			return Ok(());
		}
		if self.escape_zero {
			self.escape_zero = false;
			if array[0] <= 1 {
				self.inner.write_all(&[1])?;
			}
		}
		self.inner.write_all(&array)?;
		Ok(())
	}

	pub fn write_terminator(&mut self) -> Result<(), EncodeError> {
		self.inner.write_all(&[0])?;
		Ok(())
	}

	pub fn write_f32(&mut self, v: f32) -> Result<(), EncodeError> {
		let v = v.to_bits() as i32;
		let t = (v >> 31) | i32::MIN;
		self.write_u32((v ^ t) as u32)
	}

	pub fn write_f64(&mut self, v: f64) -> Result<(), EncodeError> {
		let v = v.to_bits() as i64;
		let t = (v >> 63) | i64::MIN;
		self.write_u64((v ^ t) as u64)
	}

	impl_prims! {signed i8,write_i8}
	impl_prims! {u8,write_u8}
	impl_prims! {signed i16,write_i16}
	impl_prims! {u16,write_u16}
	impl_prims! {signed i32,write_i32}
	impl_prims! {u32,write_u32}
	impl_prims! {signed i64,write_i64}
	impl_prims! {u64,write_u64}
	impl_prims! {signed i128,write_i128}
	impl_prims! {u128,write_u128}
}
