use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::fmt::Debug;
use storekey::{deserialize, serialize};

macro_rules! roundtrip_inner {
	($v: expr) => {
		#[allow(unused)]
		let mut v2 = $v.clone();
		let serialized = serialize(&$v).unwrap();
		v2 = deserialize(&serialized).unwrap();
		assert_eq!($v, v2);
	};
}

macro_rules! roundtrip {
	($v: expr) => {
		roundtrip_inner!($v.clone());

		let array = [$v.clone(), $v.clone()];

		roundtrip_inner!(array);
		//roundtrip_inner!(vec![$v.clone(); 2]);
	};
}

fn expect<T: Serialize + DeserializeOwned + PartialEq + Debug>(t: T, expected: &[u8]) {
	assert_eq!(serialize(&t).unwrap(), expected);

	assert_eq!(deserialize::<T>(expected).unwrap(), t);
}

fn less<T: Serialize + Debug + PartialEq + PartialOrd>(a: T, b: T) {
	// Exception for float NaNs.
	if a == a && b == b {
		assert!(a < b, "{a:?} < {b:?} (before serialization)");
	}
	let a = serialize(&a).unwrap();
	let b = serialize(&b).unwrap();
	assert!(a < b, "{a:?} < {b:?} (after serializtion)");
}

#[test]
fn unit() {
	expect((), &[]);
}

#[test]
fn boolean() {
	expect(false, &[0]);
	expect(true, &[1]);
	less(false, true);
}

#[test]
fn option() {
	expect(None::<u8>, &[0]);
	expect(Some::<u8>(5), &[1, 5]);
}

#[test]

fn int() {
	less(0, 1);
	less(30, 1000);
	less(0, u32::MAX);
	less(0, u64::MAX);
	less(i8::MIN, i8::MAX);
	less(-1, 0);
	less(-1, 1);
	less(i64::MIN, i64::MAX);
}

#[test]
fn fuzz_varint() {
	let mut n = 0u64;
	loop {
		roundtrip!(n);
		roundtrip!(u64::MAX - n);
		roundtrip!(n as i64);
		roundtrip!((u64::MAX - n) as i64);
		n = if let Some(next) = n.checked_add(1).and_then(|n| n.checked_mul(2)) {
			next
		} else {
			break;
		};
	}
}

#[test]
fn floats() {
	const NEGATIVE_NAN: u64 = 18444492273895866368;
	macro_rules! float {
		($size: ty) => {
			let ordering = [
				f64::from_bits(NEGATIVE_NAN) as $size,
				<$size>::NEG_INFINITY,
				-10.0,
				-1.0,
				-<$size>::MIN_POSITIVE,
				0.0,
				<$size>::MIN_POSITIVE,
				1.0,
				10.0,
				<$size>::INFINITY,
				<$size>::NAN,
			];
			for window in ordering.windows(2) {
				less(&window[0], &window[1]);
			}
		};
	}

	float!(f32);
	float!(f64);
}

#[test]
fn chars() {
	roundtrip!('a');
	less('a', 'b');

	for u in 1..=char::MAX as u32 {
		if let Some(c) = char::from_u32(u) {
			roundtrip!(c);
		}
	}

	expect('a', &[b'a', 0]);

	assert!(serialize(&'\0').is_err());
}

#[test]
fn enums() {
	expect(Ok::<u8, ()>(5), &[0, 0, 0, 0, 5]);
	expect(Err::<(), u8>(10), &[0, 0, 0, 1, 10]);
	expect(vec![Ok::<u8, ()>(5)], &[0, 0, 0, 0, 5, 1]);

	#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
	enum Test<'a> {
		A,
		B(u8),
		C(String),
		D(&'a str),
	}

	roundtrip!(Test::A);
	roundtrip!(Test::B(42));
	roundtrip!(Test::C("hello".to_owned()));
	roundtrip!(Test::D("hello"));
}

#[test]

fn vector() {
	roundtrip!(vec![2, 3, 4, 5]);
}

#[test]

fn bytes() {
	roundtrip!(vec![5u8; 9]);
}

#[test]

fn strings() {
	expect("foo".to_owned(), b"foo\0");
	roundtrip!("".to_owned());
	roundtrip!("hello world!".to_owned());
	roundtrip!("adiós".to_owned());
	less("aaa", "bbb");
}

#[test]

fn borrowed_bytes() {
	#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
	struct Borrowed<'a> {
		#[serde(with = "serde_bytes")]
		bytes: &'a [u8],
	}

	let buf = vec![1, 2, 3];
	let borrowed = Borrowed {
		bytes: &buf,
	};

	roundtrip!(borrowed);
}

#[test]
fn borrowed_string() {
	#[derive(Debug, PartialEq, Serialize, Deserialize)]
	struct Borrowed<'a> {
		one: &'a str,
		two: &'a str,
	}

	assert_eq!(
		deserialize::<Borrowed<'_>>(b"\0\0").unwrap(),
		Borrowed {
			one: "",
			two: ""
		}
	);
	assert_eq!(
		deserialize::<Borrowed<'_>>(b"foo\0test\0").unwrap(),
		Borrowed {
			one: "foo",
			two: "test",
		}
	);
}

#[test]
fn fixed_sized_array() {
	let array: [u8; 5] = [2, 3, 4, 5, 6];
	roundtrip!(array);
}

#[test]
fn structs() {
	#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
	struct Lq<'a> {
		__: u8,
		_a: u8,
		pub ns: &'a str,
		_b: u8,
		pub db: &'a str,
		_c: u8,
		_d: u8,
		_e: u8,
		pub lq: [u8; 16],
	}

	let lq = Lq {
		__: 0x2f, // /
		_a: 0x2a, // *
		ns: "test",
		_b: 0x2a, // *
		db: "test",
		_c: 0x21, // !
		_d: 0x6c, // l
		_e: 0x71, // v
		lq: [0; 16],
	};

	println!("{:?}", serialize(&lq));

	roundtrip!(lq);
}

#[test]
fn decimal() {
	let ordering = [
		Decimal::MIN,
		-Decimal::TEN,
		Decimal::from_f32(-3.141592654).unwrap(),
		Decimal::from_f32(-3.14).unwrap(),
		Decimal::NEGATIVE_ONE,
		Decimal::ZERO,
		Decimal::ONE,
		Decimal::TWO,
		Decimal::from_f32(3.14).unwrap(),
		Decimal::from_f32(3.141592654).unwrap(),
		Decimal::ONE_HUNDRED,
		Decimal::ONE_THOUSAND,
		Decimal::MAX,
	];
	for window in ordering.windows(2) {
		less(&window[0], &window[1]);
	}
}
