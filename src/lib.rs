//! Intelligent serialization for binary blobs.
//!
//! Where `Vec<u8>` always serializes as an array of bytes, this crate provides serialization
//! functions which try to make an intelligent decision about how to serialize a byte vector based
//! on the serialization format.
//!
//! For binary formats like [`bincode`](https://docs.rs/bincode/latest/bincode/), the array-of-bytes
//! serialization works great: it is compact and introduces very little overhead. But for
//! human-readable types such as [`serde_json`](https://docs.rs/serde_json/latest/serde_json/), it's
//! far from ideal. The text encoding of an array introduces substantial overhead, and the resulting
//! array of opaque bytes isn't particularly readable anyways.
//!
//! `base64-bytes` uses the [`is_human_readable`](serde::Serializer::is_human_readable) property of
//! a serializer to distinguish these cases. For binary formats, it uses the default `Vec<u8>`
//! serialization. For human-readable formats, it uses a much more compact and conventional base 64
//! encoding.
//!
//! # Usage
//!
//! The interface consists of [`serialize`] and [`deserialize`] functions. While these _can_ be
//! called directly, they are intended to be used with serde's
//! [field attributes](https://serde.rs/field-attrs.html) controlling serialization, like:
//!
//! ```
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Deserialize, Serialize)]
//! struct SomeType {
//!     #[serde(
//!         serialize_with = "base64_bytes::serialize",
//!         deserialize_with = "base64_bytes::deserialize",
//!     )]
//!     bytes: Vec<u8>,
//! }
//! ```
//!
//! Or, as a shorthand:
//!
//! ```
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Deserialize, Serialize)]
//! struct SomeType {
//!     #[serde(with = "base64_bytes")]
//!     bytes: Vec<u8>,
//! }
//! ```
//!

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use serde::{
    de::{Deserialize, Deserializer, Error},
    ser::{Serialize, Serializer},
};

/// Serialize a byte vector.
pub fn serialize<S: Serializer, T: AsRef<[u8]>>(v: &T, s: S) -> Result<S::Ok, S::Error> {
    if s.is_human_readable() {
        BASE64.encode(v).serialize(s)
    } else {
        v.as_ref().serialize(s)
    }
}

/// Deserialize a byte vector.
pub fn deserialize<'a, D: Deserializer<'a>>(d: D) -> Result<Vec<u8>, D::Error> {
    if d.is_human_readable() {
        Ok(BASE64
            .decode(String::deserialize(d)?)
            .map_err(|err| D::Error::custom(format!("invalid base64: {err}")))?)
    } else {
        Ok(Vec::deserialize(d)?)
    }
}

#[cfg(test)]
mod test {
    use crate::BASE64;
    use base64::Engine;
    use rand::RngCore;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
    struct Test {
        #[serde(with = "crate")]
        bytes: Vec<u8>,
    }

    #[test]
    fn test_bytes_serde() {
        let mut rng = rand::thread_rng();

        for len in [0, 1, 10, 1000] {
            let mut t = Test {
                bytes: vec![0; len],
            };
            rng.fill_bytes(&mut t.bytes);

            // The binary serialization should be highly efficient: just the length followed by the
            // raw bytes.
            let binary = bincode::serialize(&t).unwrap();
            assert_eq!(binary[..8], (len as u64).to_le_bytes());
            assert_eq!(t.bytes, binary[8..]);
            // Check deserialization.
            assert_eq!(t, bincode::deserialize::<Test>(&binary).unwrap());

            // The JSON serialization should return a base 64 string.
            let json = serde_json::to_value(&t).unwrap();
            assert_eq!(json["bytes"].as_str().unwrap(), BASE64.encode(&t.bytes));
            // Check deserialization.
            assert_eq!(t, serde_json::from_value::<Test>(json).unwrap());
        }
    }
}
