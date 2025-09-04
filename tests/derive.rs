use std::{borrow::Cow, fmt::Debug};

use storekey::{
	decode, decode_borrow, encode_vec, encode_vec_format, BorrowDecode, Decode, Encode, EscapedStr,
	ToEscaped,
};

fn roundtrip<T: Encode + Decode + for<'a> BorrowDecode<'a> + Debug + PartialEq>(a: T) {
	let enc = encode_vec(&a).unwrap();
	let dec = decode(enc.as_slice()).unwrap();
	assert_eq!(a, dec);
	let dec = decode_borrow(enc.as_slice()).unwrap();
	assert_eq!(a, dec);
}

#[derive(Encode, Decode, BorrowDecode, PartialEq, Debug)]
struct Test {
	a: u32,
	b: u64,
	c: f32,
	d: String,
}

#[test]
fn basic_struct() {
	roundtrip(Test {
		a: 3828,
		b: 192,
		#[allow(clippy::excessive_precision)]
		c: 19_293.231_2,
		d: "Test string\x00".to_string(),
	});
}

#[derive(Encode, Decode, BorrowDecode, PartialEq, Debug)]
struct Test2 {
	c: Vec<Test>,
	d: String,
}

#[test]
fn nested_struct() {
	roundtrip(Test2 {
		c: vec![
			Test {
				a: 129,
				b: 999999999,
				c: -1929.312e33,
				d: "\x00\x01bla bla".to_string(),
			},
			Test {
				a: 1209399,
				b: 999999999999,
				c: -1929.312e34,
				d: "\x01\x00bla bla".to_string(),
			},
		],
		d: "Test string\x00".to_string(),
	});
}

#[derive(Encode, Decode, BorrowDecode, PartialEq, Debug)]
struct TestGen<A> {
	a: A,
}

#[test]
fn generic_struct() {
	roundtrip(TestGen {
		a: Test {
			a: 129,
			b: 999999999,
			c: -1929.312e33,
			d: "\x00\x01bla bla".to_string(),
		},
	});
}

#[derive(BorrowDecode, PartialEq, Debug)]
struct TestBorrowed<'de> {
	a: u32,
	b: u64,
	c: f32,
	d: &'de EscapedStr,
}

#[test]
fn lifetime_struct() {
	let before = Test {
		a: 3828,
		b: 192,
		#[allow(clippy::excessive_precision)]
		c: 19293.2312,
		d: "Test string\x00".to_string(),
	};

	let enc = encode_vec(&before).unwrap();
	let dec: TestBorrowed = decode_borrow(&enc).unwrap();
	assert_eq!(before.a, dec.a);
	assert_eq!(before.b, dec.b);
	assert_eq!(before.c, dec.c);
	assert_eq!(before.d.as_str(), dec.d);
}

#[derive(Encode, ToEscaped, PartialEq, Debug)]
struct TestEscaped<'a> {
	slice: &'a [u8],
	str: &'a str,
}

#[test]
fn escaped() {
	let before = TestEscaped {
		slice: &[0, 1],
		str: "\x00\x01",
	};
	let encode = encode_vec(&before).unwrap();
	let after: TestEscapedEscaped = decode_borrow(encode.as_slice()).unwrap();
	assert_eq!(before.slice, after.slice);
	assert_eq!(before.str, after.str);
	let encode_after = encode_vec(&after).unwrap();
	assert_eq!(encode, encode_after);
}

#[derive(Encode, BorrowDecode, PartialEq, Debug)]
struct TestCow<'a> {
	slice: Cow<'a, [u8]>,
	str: Cow<'a, str>,
}

#[test]
fn cow() {
	let before = TestEscaped {
		slice: &[0, 1],
		str: "\x00\x01",
	};
	let encode = encode_vec(&before).unwrap();
	let after: TestCow = decode_borrow(encode.as_slice()).unwrap();

	assert_eq!(before.slice, after.slice.as_ref());
	assert_eq!(before.str, after.str.as_ref());

	assert!(matches!(after.slice, Cow::Owned(_)));
	assert!(matches!(after.str, Cow::Owned(_)));

	let before = TestEscaped {
		slice: &[2, 3],
		str: "hello there",
	};
	let encode = encode_vec(&before).unwrap();
	let after: TestCow = decode_borrow(encode.as_slice()).unwrap();

	assert_eq!(before.slice, after.slice.as_ref());
	assert_eq!(before.str, after.str.as_ref());

	assert!(matches!(after.slice, Cow::Borrowed(_)));
	assert!(matches!(after.str, Cow::Borrowed(_)));
}

#[derive(Encode, Decode, BorrowDecode, PartialEq, Debug)]
enum TestEnum {
	Unit,
	Unnamed(u32, Vec<u8>),
	Named {
		a: Vec<u8>,
		b: String,
	},
}

#[test]
fn basic_enum() {
	roundtrip(TestEnum::Unit);
	roundtrip(TestEnum::Unnamed(938492u32, vec![0, 0, 0, 1, 1]));
	roundtrip(TestEnum::Named {
		a: vec![0, 0, 0, 1, 1],
		b: "\x00\x01 other things".to_string(),
	});
}

pub enum OtherFormat {}

#[derive(Encode)]
pub struct EncodeDiff(u16);

impl Encode<OtherFormat> for EncodeDiff {
	fn encode<W: std::io::Write>(
		&self,
		w: &mut storekey::Writer<W>,
	) -> Result<(), storekey::EncodeError> {
		w.write_u32(self.0 as u32)
	}
}

#[derive(Encode)]
#[storekey(format = "()")]
#[storekey(format = "OtherFormat")]
pub struct FormatContainer {
	a: EncodeDiff,
	b: EncodeDiff,
}

#[test]
fn formats() {
	let example = FormatContainer {
		a: EncodeDiff(1),
		b: EncodeDiff(2),
	};
	let data = encode_vec(&example).unwrap();
	assert_eq!(data, [0, 1, 0, 2]);
	let data = encode_vec_format::<OtherFormat, _>(&example).unwrap();
	assert_eq!(data, [0, 0, 0, 1, 0, 0, 0, 2]);
}
