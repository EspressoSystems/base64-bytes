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
//! a serializer to distinguish these cases. For binary formats, it emits the blob in a single
//! [`serialize_bytes`](serde::Serializer::serialize_bytes) call and reads it back in a single
//! [`deserialize_byte_buf`](serde::Deserializer::deserialize_byte_buf) call. For human-readable
//! formats, it uses a much more compact and conventional base 64 encoding.
//!
//! Length-prefixed binary formats such as `bincode` and `postcard` encode this identically to the
//! byte-at-a-time `Vec<u8>` serialization, so the wire format is unchanged. Self-describing formats
//! that distinguish byte strings from arrays, such as CBOR and MessagePack, now emit a byte string;
//! deserialization still accepts either.
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
        s.serialize_bytes(v.as_ref())
    }
}

/// Deserialize a byte vector.
///
/// The binary branch asks the format for the whole blob at once, so the destination buffer is
/// sized from the format's length prefix before any bytes are read. Readers of untrusted input
/// must bound that themselves, e.g. `bincode::options().with_limit(n)`; an unbounded
/// `bincode::deserialize_from` over a socket will allocate whatever length the peer claims.
pub fn deserialize<'a, D: Deserializer<'a>>(d: D) -> Result<Vec<u8>, D::Error> {
    if d.is_human_readable() {
        Ok(BASE64
            .decode(String::deserialize(d)?)
            .map_err(|err| D::Error::custom(format!("invalid base64: {err}")))?)
    } else {
        serde_bytes::deserialize(d)
    }
}

#[cfg(test)]
mod test {
    use crate::BASE64;
    use base64::Engine;
    use rand::RngCore;
    use serde::de::{value::Error as ValueError, Deserializer, Error, Visitor};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
    struct Test {
        #[serde(with = "crate")]
        bytes: Vec<u8>,
    }

    /// Encodings produced by 0.1.0, which emitted the binary branch one byte at a time.
    ///
    /// Pinned as literals rather than derived from the current code, so that a change to *how*
    /// the bytes are emitted cannot quietly change *what* is emitted. Both directions are
    /// checked: old readers must accept what this version writes, and this version must accept
    /// what old writers produced.
    const V0_1_0: &[(&[u8], &[u8], &str)] = &[
        (&[], &[0, 0, 0, 0, 0, 0, 0, 0], ""),
        (&[0], &[1, 0, 0, 0, 0, 0, 0, 0, 0], "AA=="),
        (
            &[
                0, 17, 34, 51, 68, 85, 102, 119, 136, 153, 170, 187, 204, 221, 238, 255,
            ],
            &[
                16, 0, 0, 0, 0, 0, 0, 0, 0, 17, 34, 51, 68, 85, 102, 119, 136, 153, 170, 187, 204,
                221, 238, 255,
            ],
            "ABEiM0RVZneImaq7zN3u/w==",
        ),
    ];

    #[test]
    fn binary_encoding_is_unchanged_since_0_1_0() {
        for (bytes, bincoded, base64) in V0_1_0 {
            let t = Test {
                bytes: bytes.to_vec(),
            };

            assert_eq!(&bincode::serialize(&t).unwrap(), bincoded);
            assert_eq!(bincode::deserialize::<Test>(bincoded).unwrap(), t);

            assert_eq!(serde_json::to_value(&t).unwrap()["bytes"], *base64);
            assert_eq!(
                serde_json::from_value::<Test>(serde_json::json!({ "bytes": base64 })).unwrap(),
                t
            );
        }
    }

    /// A deserializer which serves the binary branch only if the blob is requested in one call.
    ///
    /// Anything else -- notably serde's default `Vec<u8>` path, which asks for a sequence and then
    /// visits each byte -- lands in `deserialize_any` and fails.
    struct WholeBlobOnly<'a> {
        bytes: &'a [u8],
    }

    impl<'de> Deserializer<'de> for WholeBlobOnly<'_> {
        type Error = ValueError;

        fn is_human_readable(&self) -> bool {
            false
        }

        fn deserialize_bytes<V: Visitor<'de>>(self, v: V) -> Result<V::Value, Self::Error> {
            v.visit_bytes(self.bytes)
        }

        fn deserialize_byte_buf<V: Visitor<'de>>(self, v: V) -> Result<V::Value, Self::Error> {
            v.visit_byte_buf(self.bytes.to_vec())
        }

        fn deserialize_any<V: Visitor<'de>>(self, _: V) -> Result<V::Value, Self::Error> {
            Err(ValueError::custom("blob was not requested in one call"))
        }

        serde::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string option unit
            unit_struct newtype_struct seq tuple tuple_struct map struct enum identifier
            ignored_any
        }
    }

    #[test]
    fn binary_deserialization_requests_the_blob_in_one_call() {
        for (bytes, ..) in V0_1_0 {
            // The per-element path this crate used to take must not be served, or the assertion
            // below would hold no matter what `crate::deserialize` does.
            <Vec<u8>>::deserialize(WholeBlobOnly { bytes }).unwrap_err();

            assert_eq!(
                crate::deserialize(WholeBlobOnly { bytes }).unwrap(),
                bytes.to_vec()
            );
        }
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
