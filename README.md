# base64-bytes

_Binary blobs with intelligent serialization._

This crate provides the `Base64Bytes` type. This type behaves like a `Vec<u8>` in almost all cases
except serialization. Where `Vec<u8>` always serializes as an array of bytes, `Base64Bytes` tries to
make an intelligent decision about how to serialize based on the serialization format.

For binary formats like [`bincode`](https://crates.io/crates/bincode), the array-of-bytes
serialization works great: it is compact and introduces very little overhead. But for human-readable
types such as [`json`](https://crates.io/crates/serde_json), it's far from ideal. The text encoding
of an array introduces substantial overhead, and the resulting array of opaque bytes isn't
particularly readable anyways.

`Base64Bytes` uses the [`is_human_readable`](https://docs.rs/serde/latest/serde/trait.Serializer.html#method.is_human_readable)
property of a serializer to distinguish these cases. For binary formats, it uses the default
`Vec<u8>` serialization. For human-readable formats, it uses a much more compact and conventional
base 64 encoding.
