//! Compares this crate's encoding against a plain `Vec<u8>` field, which is what it replaces.
//!
//! Throughput is reported over the payload length, not the encoded length, so the two formats share
//! a denominator. JSON therefore moves roughly 1.37x more bytes on the wire than its number says.

use criterion::{
    criterion_group, criterion_main, measurement::WallTime, BenchmarkGroup, BenchmarkId, Criterion,
    Throughput,
};
use rand::RngCore;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
struct Blob {
    #[serde(with = "base64_bytes")]
    bytes: Vec<u8>,
}

#[derive(Deserialize, Serialize)]
struct Plain {
    bytes: Vec<u8>,
}

const KIBI: usize = 1024;
const MEBI: usize = KIBI * KIBI;
const SIZES: &[usize] = &[KIBI, 64 * KIBI, MEBI, 4 * MEBI];

fn sample(len: usize) -> Vec<u8> {
    let mut bytes = vec![0; len];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes
}

fn label(len: usize) -> String {
    if len >= MEBI {
        format!("{}MiB", len / MEBI)
    } else {
        format!("{}KiB", len / KIBI)
    }
}

fn bench_bincode<T: Serialize + DeserializeOwned>(
    g: &mut BenchmarkGroup<WallTime>,
    kind: &str,
    len: usize,
    t: &T,
) {
    let encoded = bincode::serialize(t).unwrap();
    g.bench_function(
        BenchmarkId::new(format!("serialize/{kind}"), label(len)),
        |b| b.iter(|| bincode::serialize(t).unwrap()),
    );
    g.bench_function(
        BenchmarkId::new(format!("deserialize/{kind}"), label(len)),
        |b| b.iter(|| bincode::deserialize::<T>(&encoded).unwrap()),
    );
}

fn bench_json<T: Serialize + DeserializeOwned>(
    g: &mut BenchmarkGroup<WallTime>,
    kind: &str,
    len: usize,
    t: &T,
) {
    let encoded = serde_json::to_vec(t).unwrap();
    g.bench_function(
        BenchmarkId::new(format!("serialize/{kind}"), label(len)),
        |b| b.iter(|| serde_json::to_vec(t).unwrap()),
    );
    g.bench_function(
        BenchmarkId::new(format!("deserialize/{kind}"), label(len)),
        |b| b.iter(|| serde_json::from_slice::<T>(&encoded).unwrap()),
    );
}

fn bench(c: &mut Criterion) {
    let mut g = c.benchmark_group("bincode");
    for &len in SIZES {
        let bytes = sample(len);
        g.throughput(Throughput::Bytes(len as u64));
        bench_bincode(
            &mut g,
            "base64-bytes",
            len,
            &Blob {
                bytes: bytes.clone(),
            },
        );
        bench_bincode(&mut g, "vec-u8", len, &Plain { bytes });
    }
    g.finish();

    let mut g = c.benchmark_group("json");
    for &len in SIZES {
        let bytes = sample(len);
        g.throughput(Throughput::Bytes(len as u64));
        bench_json(
            &mut g,
            "base64-bytes",
            len,
            &Blob {
                bytes: bytes.clone(),
            },
        );
        bench_json(&mut g, "vec-u8", len, &Plain { bytes });
    }
    g.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
