use std::fmt::{self};
use std::io::Write;
use std::{slice, str};

use crate::{DecodeError, EncodeError};

use super::reader::BorrowReader;
use super::{BorrowDecode, Encode, Writer};

/// A slice buffer which is in an escaped format:
/// containing possible 0u8 and 1u8 bytes escaped with a 1u8 as well as a final terminating null
/// byte.
#[repr(transparent)]
pub struct EscapedSlice([u8]);

impl EscapedSlice {
	/// Create an escaped slice from a byte slice.
	///
	/// # Safety
	/// User must ensure the slice is properly escaped.
	pub unsafe fn from_slice(b: &[u8]) -> &EscapedSlice {
		// Safety: Safe because EscapedSlice has repr(transparent)
		unsafe { std::mem::transmute(b) }
	}

	/// Returns the raw underlying byte representation of the escaped string, including escaped
	/// bytes.
	pub fn as_bytes(&self) -> &[u8] {
		&self.0
	}

	/// Returns an iterator over the bytes in this slice, unescaping escaped bytes.
	pub fn iter(&self) -> EscapedIter<'_> {
		EscapedIter(self.0[..self.0.len() - 1].iter())
	}
}

impl<F> Encode<F> for EscapedSlice {
	fn encode<W: Write>(&self, w: &mut Writer<W>) -> Result<(), EncodeError> {
		w.write_escaped_slice(self)
	}
}

impl<'de, F> BorrowDecode<'de, F> for &'de EscapedSlice {
	fn borrow_decode(w: &mut BorrowReader<'de>) -> Result<Self, DecodeError> {
		w.read_escaped_slice()
	}
}

impl fmt::Debug for EscapedSlice {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let mut escaped = false;
		let iter = self.0[..self.0.len() - 1].iter().copied().filter(|x| {
			if escaped {
				escaped = false;
				true
			} else if *x == 1 {
				escaped = true;
				false
			} else {
				true
			}
		});

		f.debug_list().entries(iter).finish()
	}
}

impl PartialEq<[u8]> for EscapedSlice {
	fn eq(&self, other: &[u8]) -> bool {
		let mut iter = other.iter().copied();
		for a in self.iter().copied() {
			let Some(b) = iter.next() else {
				return false;
			};
			if a != b {
				return false;
			}
		}
		iter.next().is_none()
	}
}

impl PartialEq<EscapedSlice> for [u8] {
	fn eq(&self, other: &EscapedSlice) -> bool {
		EscapedSlice::eq(other, self)
	}
}

impl Eq for EscapedSlice {}
impl PartialEq for EscapedSlice {
	fn eq(&self, other: &Self) -> bool {
		self.0 == other.0
	}
}

impl<'a> IntoIterator for &'a EscapedSlice {
	type Item = &'a u8;

	type IntoIter = EscapedIter<'a>;

	fn into_iter(self) -> Self::IntoIter {
		self.iter()
	}
}

pub struct EscapedIter<'a>(slice::Iter<'a, u8>);

impl<'a> Iterator for EscapedIter<'a> {
	type Item = &'a u8;

	fn next(&mut self) -> Option<Self::Item> {
		let c = self.0.next()?;
		if *c == 1 {
			self.0.next()
		} else {
			Some(c)
		}
	}
}

/// A slice buffer which is in an escaped format:
/// containing possible 0u8 and 1u8 bytes escaped with a 1u8 as well as a final terminating null
/// byte
#[repr(transparent)]
pub struct EscapedStr(str);

impl EscapedStr {
	/// Create an escaped str from a normal str.
	///
	/// # Safety
	/// User must ensure the str is properly escaped.
	pub unsafe fn from_str(b: &str) -> &EscapedStr {
		// Safety: Safe because EscapedStr has repr(transparent)
		unsafe { std::mem::transmute(b) }
	}

	/// Returns the raw underlying byte representation of the escaped string, including escaped
	/// bytes.
	pub fn as_bytes(&self) -> &[u8] {
		self.0.as_bytes()
	}

	pub fn as_slice(&self) -> &EscapedSlice {
		unsafe { EscapedSlice::from_slice(self.0.as_bytes()) }
	}

	/// Returns a iterator over the characters in this string, unescaping escaped characters.
	pub fn chars(&self) -> EscapedChars<'_> {
		EscapedChars(self.0[..self.0.len() - 1].chars())
	}
}

impl<F> Encode<F> for EscapedStr {
	fn encode<W: Write>(&self, w: &mut Writer<W>) -> Result<(), EncodeError> {
		w.write_escaped_slice(self.as_slice())
	}
}

impl<'de, F> BorrowDecode<'de, F> for &'de EscapedStr {
	fn borrow_decode(w: &mut BorrowReader<'de>) -> Result<Self, DecodeError> {
		w.read_escaped_str()
	}
}

impl PartialEq<str> for EscapedStr {
	fn eq(&self, other: &str) -> bool {
		let mut iter = other.chars();
		for a in self.chars() {
			let Some(b) = iter.next() else {
				return false;
			};
			if a != b {
				return false;
			}
		}
		iter.next().is_none()
	}
}

impl PartialEq<EscapedStr> for str {
	fn eq(&self, other: &EscapedStr) -> bool {
		EscapedStr::eq(other, self)
	}
}

impl Eq for EscapedStr {}
impl PartialEq for EscapedStr {
	fn eq(&self, other: &Self) -> bool {
		self.0 == other.0
	}
}

impl fmt::Debug for EscapedStr {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		use std::fmt::Write;
		f.write_char('"')?;
		for c in self.chars() {
			for c in c.escape_debug() {
				f.write_char(c)?;
			}
		}
		f.write_char('"')?;
		Ok(())
	}
}

impl fmt::Display for EscapedStr {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		use std::fmt::Write;
		for c in self.chars() {
			f.write_char(c)?;
		}
		Ok(())
	}
}

pub struct EscapedChars<'a>(str::Chars<'a>);

impl<'a> Iterator for EscapedChars<'a> {
	type Item = char;

	fn next(&mut self) -> Option<Self::Item> {
		let c = self.0.next()?;
		if c == '\x01' {
			self.0.next()
		} else {
			Some(c)
		}
	}
}
