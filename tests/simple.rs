use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::fmt::Debug;
use storekey::{deserialize, serialize};

fn roundtrip<T: Serialize + DeserializeOwned + PartialEq + Debug>(t: T) {
	let serialized = serialize(&t).unwrap();
	let deserialized = deserialize::<T>(&serialized).unwrap();
	assert_eq!(t, deserialized);
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
		roundtrip(n);
		roundtrip(u64::MAX - n);
		roundtrip(n as i64);
		roundtrip((u64::MAX - n) as i64);
		n = if let Some(next) = n.checked_add(1).and_then(|n| n.checked_mul(2)) {
			next
		} else {
			break;
		};
	}
}

#[test]
fn floats() {
	macro_rules! float {
		($size: ty) => {
			let ordering = [<$size>::NEG_INFINITY, -1.0, 0.0, 1.0, <$size>::INFINITY, <$size>::NAN];
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
	roundtrip('a');
	less('a', 'b');

	for u in 1..=char::MAX as u32 {
		if let Some(c) = char::from_u32(u) {
			roundtrip(c);
		}
	}

	expect('a', &[b'a', 0]);

	assert!(serialize(&'\0').is_err());
}

#[test]
fn strings() {
	roundtrip("".to_owned());
	roundtrip("hello world!".to_owned());
	roundtrip("adiós".to_owned());
	less("aaa", "bbb");
}

#[test]
fn enums() {
	expect(Ok::<u8, ()>(5), &[0, 0, 0, 0, 5]);
	expect(Err::<(), u8>(10), &[0, 0, 0, 1, 10]);
}

#[test]
fn borrowed_strings() {
	#[derive(Debug, PartialEq, Serialize, Deserialize)]
	struct Borrowed<'a> {
		string: &'a str,
	}

	assert_eq!(
		deserialize::<Borrowed<'_>>(b"\0").unwrap(),
		Borrowed {
			string: ""
		}
	);
	assert_eq!(
		deserialize::<Borrowed<'_>>(b"test\0").unwrap(),
		Borrowed {
			string: "test"
		}
	);
}
