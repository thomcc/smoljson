//! This is a minimalist JSON library that trades away several desirable
//! qualities (ergonomics, performance, ...) in favor of small code size and
//! fast compiles.
//!
//! It doesn't support serde, or any other custom derive. I'm not particularly
//! happy with the API, and will likely change it to be better in the future. As
//! a result, docs are somewhat sparse.
//!
//! ## Basic Usage
//! ```
//! use smoljson::Value;
//! let v = Value::from_str(r#"{"foo": [1, 2, {"bar": 3}]}"#).unwrap();
//! let expected = smoljson::json!({"foo": [1, 2, {"bar": 3}]});
//! assert_eq!(v, expected);
//! ```
//!
//! ## Intended use case
//!
//! The intended use case is situtions where small number of low maintenance
//! (rare changes to struct layout) data structures need (de)serialization in a
//! project where keeping a low compile time is important.
//!
//! ## JSON with Comments (CJSON) support
//!
//! By default, strictly correct JSON is required, and this does not allow
//! comments. However this crate supports "JSON with Comments", aka `cjson`.
//! This is a non-standard extension to JSON that allows JavaScript-style
//! comments. It's somewhat common now, and is supported by things like VSCode
//! (and as described by the
//! [`jsonc-parser`](https://github.com/microsoft/node-jsonc-parser) NPM
//! package).
//!
//! This is generally enabled on a per-use case using the [`Dialect`], but the
//! default settings can be controlled using some cargo features. See
//! [`Dialect::DEFAULT`]'s documentation for details.

#![no_std]
#![allow(dead_code)]

#[doc(hidden)]
pub extern crate alloc;

#[doc(hidden)]
pub use core;

macro_rules! opt_extract {
    ($this:expr, $pat:pat => $res:expr) => {
        if let $pat = $this {
            $res
        } else {
            None
        }
    };
}

// `?` expansion currently harms both compile time (lots of llvm instrs
// generated) and runtime :(
macro_rules! tri {
    ($e:expr) => {
        match $e {
            Ok(v) => v,
            Err(e) => return Err(e),
        }
    };
}
#[macro_use]
mod mac;

pub mod read;
pub mod value;
pub mod write;
pub use read::{Dialect, Error, Reader};
pub use value::Value;

pub type ValOwn = Value<'static>;
