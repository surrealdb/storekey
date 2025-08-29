use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};
use std::fmt::Debug;
use std::hash::Hash;

use crate::{decode, decode_borrow, encode_vec, BorrowDecode, Decode, Encode};

macro_rules! test_primitives {
	($t:ident,$name:ident) => {
		#[test]
		fn $name() {
			let v: $t = 0;
			let enc = encode_vec(&v);
			let dec: $t = decode(enc.as_slice()).unwrap();
			assert_eq!(v, dec, concat!("Conversion for ", stringify!($t), " failed"));
			let dec: $t = decode_borrow(enc.as_slice()).unwrap();
			assert_eq!(v, dec, concat!("Conversion for ", stringify!($t), " failed"));
			let v: $t = $t::MIN;
			let enc = encode_vec(&v);
			let dec: $t = decode(enc.as_slice()).unwrap();
			assert_eq!(v, dec, concat!("Conversion for ", stringify!($t), " failed"));
			let dec: $t = decode_borrow(enc.as_slice()).unwrap();
			assert_eq!(v, dec, concat!("Conversion for ", stringify!($t), " failed"));
			let v: $t = $t::MAX;
			let enc = encode_vec(&v);
			let dec: $t = decode(enc.as_slice()).unwrap();
			assert_eq!(v, dec, concat!("Conversion for ", stringify!($t), " failed"));
			let dec: $t = decode_borrow(enc.as_slice()).unwrap();
			assert_eq!(v, dec, concat!("Conversion for ", stringify!($t), " failed"));
		}
	};
}

test_primitives!(u8, primitive_u8);
test_primitives!(i8, primitive_i8);
test_primitives!(u16, primitive_u16);
test_primitives!(i16, primitive_i16);
test_primitives!(u32, primitive_u32);
test_primitives!(i32, primitive_i32);
test_primitives!(u64, primitive_u64);
test_primitives!(i64, primitive_i64);
test_primitives!(u128, primitive_u128);
test_primitives!(i128, primitive_i128);

#[test]
fn primitive_f32() {
	let v: f32 = 0.0;
	let enc = encode_vec(&v);
	let dec: f32 = decode(enc.as_slice()).unwrap();
	assert_eq!(v, dec, concat!("Conversion for ", stringify!(f32), " failed"));
	let dec: f32 = decode_borrow(enc.as_slice()).unwrap();
	assert_eq!(v, dec, concat!("Conversion for ", stringify!(f32), " failed"));
	let v: f32 = f32::MIN;
	let enc = encode_vec(&v);
	let dec: f32 = decode(enc.as_slice()).unwrap();
	assert_eq!(v, dec, concat!("Conversion for ", stringify!(f32), " failed"));
	let dec: f32 = decode_borrow(enc.as_slice()).unwrap();
	assert_eq!(v, dec, concat!("Conversion for ", stringify!(f32), " failed"));
	let v: f32 = f32::MAX;
	let enc = encode_vec(&v);
	let dec: f32 = decode(enc.as_slice()).unwrap();
	assert_eq!(v, dec, concat!("Conversion for ", stringify!(f32), " failed"));
	let dec: f32 = decode_borrow(enc.as_slice()).unwrap();
	assert_eq!(v, dec, concat!("Conversion for ", stringify!(f32), " failed"));
}

#[test]
fn primitive_f64() {
	let v: f64 = 0.0;
	let enc = encode_vec(&v);
	let dec: f64 = decode(enc.as_slice()).unwrap();
	assert_eq!(v, dec, concat!("Conversion for ", stringify!(f64), " failed"));
	let dec: f64 = decode_borrow(enc.as_slice()).unwrap();
	assert_eq!(v, dec, concat!("Conversion for ", stringify!(f64), " failed"));
	let v: f64 = f64::MIN;
	let enc = encode_vec(&v);
	let dec: f64 = decode(enc.as_slice()).unwrap();
	assert_eq!(v, dec, concat!("Conversion for ", stringify!(f64), " failed"));
	let dec: f64 = decode_borrow(enc.as_slice()).unwrap();
	assert_eq!(v, dec, concat!("Conversion for ", stringify!(f64), " failed"));
	let v: f64 = f64::MAX;
	let enc = encode_vec(&v);
	let dec: f64 = decode(enc.as_slice()).unwrap();
	assert_eq!(v, dec, concat!("Conversion for ", stringify!(f64), " failed"));
	let dec: f64 = decode_borrow(enc.as_slice()).unwrap();
	assert_eq!(v, dec, concat!("Conversion for ", stringify!(f64), " failed"));
}

#[test]
fn vec() {
	fn test_vec<T: Decode + Encode + for<'a> BorrowDecode<'a> + Debug + PartialEq>(vec: Vec<T>) {
		let enc = dbg!(encode_vec(&vec));
		let dec: Vec<T> = decode(enc.as_slice()).unwrap();
		assert_eq!(vec, dec);
		let dec: Vec<T> = decode_borrow(enc.as_slice()).unwrap();
		assert_eq!(vec, dec);
	}

	test_vec::<u8>(vec![]);
	test_vec(vec![1u8, 2u8, 3u8]);
	test_vec(vec![0u32]);
	test_vec(vec![0x01_01_01_01u32]);
	test_vec(vec!["hello".to_string()]);
	test_vec(vec![vec![0x01_01_01_01u32]]);
}

#[test]
fn hashmap() {
	fn test_hashmap<K, V, const S: usize>(map: [(K, V); S])
	where
		K: Decode + Encode + for<'a> BorrowDecode<'a> + Debug + PartialEq + Hash + Eq,
		V: Decode + Encode + for<'a> BorrowDecode<'a> + Debug + PartialEq,
	{
		let map: HashMap<K, V> = map.into_iter().collect();

		let enc = dbg!(encode_vec(&map));
		let dec: HashMap<K, V> = decode(enc.as_slice()).unwrap();
		assert_eq!(map.len(), dec.len());
		for (k, v) in map.iter() {
			assert_eq!(dec.get(&k).unwrap(), v, "Value for key {:?} was not correct", k);
		}

		let dec: HashMap<K, V> = decode_borrow(enc.as_slice()).unwrap();
		assert_eq!(map.len(), dec.len());
		for (k, v) in map.iter() {
			assert_eq!(dec.get(&k).unwrap(), v, "Value for key {:?} was not correct", k);
		}
	}

	test_hashmap::<u8, u8, 0>([]);
	test_hashmap([(0u8, 0u8), (1u8, 1u8)]);
	test_hashmap([
		("hello world".to_string(), 0u8),
		("\x00world".to_string(), 1u8),
		("\x01world".to_string(), 2u8),
		("\x00".to_string(), 3u8),
		("\x01".to_string(), 4u8),
		("\x00\x01".to_string(), 0u8),
	]);
	test_hashmap([(vec![0, 0, 0], vec![0, 0, 0]), (vec![1, 1, 1], vec![0, 0, 0])]);
}

#[test]
fn btree() {
	fn test_btree<K, V, const S: usize>(map: [(K, V); S])
	where
		K: Decode + Encode + for<'a> BorrowDecode<'a> + Debug + PartialEq + Ord,
		V: Decode + Encode + for<'a> BorrowDecode<'a> + Debug + PartialEq,
	{
		let map: BTreeMap<K, V> = map.into_iter().collect();

		let enc = dbg!(encode_vec(&map));
		let dec: BTreeMap<K, V> = decode(enc.as_slice()).unwrap();
		assert_eq!(map.len(), dec.len());
		for (k, v) in map.iter() {
			assert_eq!(dec.get(&k).unwrap(), v, "Value for key {:?} was not correct", k);
		}

		let dec: BTreeMap<K, V> = decode_borrow(enc.as_slice()).unwrap();
		assert_eq!(map.len(), dec.len());
		for (k, v) in map.iter() {
			assert_eq!(dec.get(&k).unwrap(), v, "Value for key {:?} was not correct", k);
		}
	}

	test_btree::<u8, u8, 0>([]);
	test_btree([(0u8, 0u8), (1u8, 1u8)]);
	test_btree([
		("hello world".to_string(), 0u8),
		("\x00world".to_string(), 1u8),
		("\x01world".to_string(), 2u8),
		("\x00".to_string(), 3u8),
		("\x01".to_string(), 4u8),
		("\x00\x01".to_string(), 0u8),
	]);
	test_btree([(vec![0, 0, 0], vec![0, 0, 0]), (vec![1, 1, 1], vec![0, 0, 0])]);
}

#[test]
fn ordering() {
	fn test_order<O: PartialOrd + Encode>(a: O, b: O) {
		let a_enc = encode_vec(&a);
		let b_enc = encode_vec(&b);
		assert_eq!(a.partial_cmp(&b), a_enc.partial_cmp(&b_enc))
	}

	fn b<K, V, const S: usize>(map: [(K, V); S]) -> BTreeMap<K, V>
	where
		K: Ord,
	{
		map.into_iter().collect()
	}

	test_order(0u8, 0);
	test_order(0u8, 1);
	test_order(0u8, 255);

	test_order(0.0, 1.0);
	test_order(0.0, 2.0);
	test_order(f32::INFINITY, f32::MAX);
	test_order(f32::NEG_INFINITY, f32::MIN);

	test_order(0.0f64, 1.0);
	test_order(0.0f64, 2.0);
	// Max safe integer.
	test_order(9007199254740990.0f64, 9007199254740990.0 - 1.0);
	test_order(-9007199254740990.0f64, -9007199254740990.0 + 1.0);
	test_order(f64::INFINITY, f64::MAX);
	test_order(f64::NEG_INFINITY, f64::MIN);

	test_order("a", "b");
	test_order("\x00", "\x00");
	test_order("\x00", "\x00\x00");
	test_order("\x00", "\x01");
	test_order("a\x00", "a\x01");

	test_order(vec![0], vec![1]);
	test_order(vec![0, 0], vec![0, 1]);
	test_order(vec![0], vec![0, 0]);
	test_order(vec![255], vec![0, 1]);

	test_order(b::<u8, u8, 0>([]), b([]));
	test_order(b([(0u8, 1u8)]), b([(0u8, 0u8)]));
	test_order(b([(0u8, 0u8), (1, 1)]), b([(0, 0), (1, 0)]));
}

#[test]
fn cow() {
	let data = "hello";
	let enc = encode_vec(data);
	let dec: Cow<'_, str> = decode_borrow(enc.as_slice()).unwrap();
	assert_eq!(data, dec.as_ref());
	assert!(matches!(dec, Cow::Borrowed(_)));

	let data = "hello\x00";
	let enc = encode_vec(data);
	let dec: Cow<'_, str> = decode_borrow(enc.as_slice()).unwrap();
	assert_eq!(data, dec.as_ref());
	assert!(matches!(dec, Cow::Owned(_)));
}
