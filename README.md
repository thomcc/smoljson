# `smoljson`
[![Build Status](https://github.com/thomcc/smoljson/workflows/CI/badge.svg)](https://github.com/thomcc/smoljson/actions)
[![Docs](https://docs.rs/smoljson/badge.svg)](https://docs.rs/smoljson)
[![Latest Version](https://img.shields.io/crates/v/smoljson.svg)](https://crates.io/crates/smoljson)

This is a minimalist JSON library that trades away several desirable qualities (ergonomics, performance, ...) in favor of small code size and fast compiles.

It doesn't support serde, or any other custom derive. I'm not particularly happy with the API, and will likely change it to be better in the future. As a result, docs are somewhat sparse.

## Basic usage

```rust
use smoljson::Value;
let v = Value::from_str(r#"{"foo": [1, 2, {"bar": 3}]}"#).unwrap();
let expected = smoljson::json!({"foo": [1, 2, {"bar": 3}]});
assert_eq!(v, expected);
```
