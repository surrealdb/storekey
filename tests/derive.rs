use std::fmt::Debug;
use storekey::{
	decode, decode_borrow, encode_vec, BorrowDecode, Decode, Encode, EscapedSlice, EscapedStr,
	ToEscaped,
};

fn roundtrip<T: Encode + Decode + for<'a> BorrowDecode<'a> + Debug + PartialEq>(a: T) {
	let enc = encode_vec(&a);
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
		b: 0192,
		c: 19293.2312,
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
		b: 0192,
		c: 19293.2312,
		d: "Test string\x00".to_string(),
	};

	let enc = encode_vec(&before);
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
