# base64-bytes

_Binary blobs with intelligent serialization._

Where `Vec<u8>` always serializes as an array of bytes, this crate provides serialization functions
which try to make an intelligent decision about how to serialize a byte vector based on the
serialization format.

For binary formats like [`bincode`](https://crates.io/crates/bincode), the array-of-bytes
serialization works great: it is compact and introduces very little overhead. But for human-readable
types such as [`json`](https://crates.io/crates/serde_json), it's far from ideal. The text encoding
of an array introduces substantial overhead, and the resulting array of opaque bytes isn't
particularly readable anyways.

`base64-bytes` uses the [`is_human_readable`](https://docs.rs/serde/latest/serde/trait.Serializer.html#method.is_human_readable)
property of a serializer to distinguish these cases. For binary formats, it emits the blob in a
single `serialize_bytes` call and reads it back in a single `deserialize_byte_buf` call. For
human-readable formats, it uses a much more compact and conventional base 64 encoding.

Length-prefixed binary formats such as `bincode` and `postcard` encode this identically to the
byte-at-a-time `Vec<u8>` serialization, so the wire format is unchanged. Self-describing formats
that distinguish byte strings from arrays, such as CBOR and MessagePack, now emit a byte string;
deserialization still accepts either.
