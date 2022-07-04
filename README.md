<br>

<p align="center">
    <a href="https://github.com/surrealdb/storekey#gh-dark-mode-only" target="_blank">
        <img width="200" src="/img/white/logo.svg" alt="Storekey Logo">
    </a>
    <a href="https://github.com/surrealdb/storekey#gh-light-mode-only" target="_blank">
        <img width="200" src="/img/black/logo.svg" alt="Storekey Logo">
    </a>
</p>

<p align="center">Binary encoding for Rust values which ensures lexicographic sort ordering. Order-preserving encoding is useful for creating keys for sorted key-value stores with byte string typed keys, such as <a href="https://github.com/surrealdb/echodb">EchoDB</a>, <a href="https://github.com/surrealdb/yokudb">YokuDB</a>, <a href="https://github.com/surrealdb/indxdb">IndxDB</a>, <a href="https://github.com/tikv/tikv">TiKV</a>, and <a href="https://github.com/surrealdb/surrealdb">SurrealDB</a>.</p>

<br>

<p align="center">
	<a href="https://github.com/surrealdb/storekey"><img src="https://img.shields.io/badge/status-stable-ff00bb.svg?style=flat-square"></a>
	&nbsp;
	<a href="https://docs.rs/storekey/"><img src="https://img.shields.io/docsrs/storekey?style=flat-square"></a>
	&nbsp;
	<a href="https://crates.io/crates/storekey"><img src="https://img.shields.io/crates/v/storekey?style=flat-square"></a>
	&nbsp;
	<a href="https://github.com/surrealdb/storekey"><img src="https://img.shields.io/badge/license-Apache_License_2.0-00bfff.svg?style=flat-square"></a>
</p>

#### Features

- Binary encoding whilst preserving lexicographic sort order
- Useful for creating keys for sorted key-value data stores
- Aims to encode values into the fewest number of bytes possible
- The exact type of a serialized value must be known in order to deserialize it
- Supports all Rust primitives, strings, options, structs, enums, vecs, and tuples

#### Original

This code is forked originally from [bytekey-fix](https://crates.io/crates/bytekey-fix), which is originally forked from [bytekey](https://crates.io/crates/bytekey), both licensed under the Apache License 2.0 license. See LICENSE for full license text.
