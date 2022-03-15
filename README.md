# storekey

Binary encoding for Rust values which ensures lexicographic sort ordering. Order-preserving encoding is useful for creating keys for sorted key-value stores with byte string typed keys, such as [EchoDB](https://github.com/surrealdb/echodb), [YokuDB](https://github.com/surrealdb/yokudb), [IndxDB](https://github.com/surrealdb/indxdb), [TiKV](https://github.com/tikv/tikv), and [SurrealDB](https://github.com/surrealdb/surrealdb).

[![](https://img.shields.io/badge/status-stable-ff00bb.svg?style=flat-square)](https://github.com/surrealdb/storekey) [![docs.rs](https://img.shields.io/docsrs/storekey?style=flat-square)](https://docs.rs/storekey/) [![Crates.io](https://img.shields.io/crates/v/storekey?style=flat-square)](https://crates.io/crates/storekey) [![](https://img.shields.io/badge/license-Apache_License_2.0-00bfff.svg?style=flat-square)](https://github.com/surrealdb/storekey) 

#### Features

- Binary encoding whilst preserving lexicographic sort order
- Useful for creating keys for sorted key-value data stores
- Aims to encode values into the fewest number of bytes possible
- The exact type of a serialized value must be known in order to deserialize it
- Supports all Rust primitives, strings, options, structs, enums, vecs, and tuples

#### Original

This code is forked originally from [bytekey-fix](https://crates.io/crates/bytekey-fix), which is originally forked from [bytekey](https://crates.io/crates/bytekey), both licensed under the Apache License 2.0 license. See LICENSE for full license text.
