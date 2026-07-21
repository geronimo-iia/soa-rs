use criterion::{Criterion, criterion_group, criterion_main};
use rand::{RngCore, SeedableRng, rngs::StdRng};
use soa_rs::{Soa, Soars};

struct Rng(StdRng);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(StdRng::seed_from_u64(seed))
    }

    fn next_f32(&mut self) -> f32 {
        f32::from_ne_bytes(self.0.next_u32().to_ne_bytes())
    }

    fn collect_vec4<T>(&mut self, count: usize) -> T
    where
        T: FromIterator<Vec4>,
    {
        core::iter::repeat_with(|| Vec4::new_rng(self))
            .take(count)
            .collect()
    }
}

#[derive(Soars, Debug, Clone, Copy, PartialEq, PartialOrd)]
#[soa_derive(Debug, PartialEq, PartialOrd)]
struct Vec4(
    #[align(64)] f32,
    #[align(64)] f32,
    #[align(64)] f32,
    #[align(64)] f32,
);

impl Vec4 {
    fn new_rng(rng: &mut Rng) -> Self {
        Self(
            rng.next_f32(),
            rng.next_f32(),
            rng.next_f32(),
            rng.next_f32(),
        )
    }

    fn dot(&self, other: &Self) -> f32 {
        self.0 * other.0 + self.1 * other.1 + self.2 * other.2 + self.3 * other.3
    }
}

impl Vec4Ref<'_> {
    fn dot(&self, other: &Self) -> f32 {
        self.0 * other.0 + self.1 * other.1 + self.2 * other.2 + self.3 * other.3
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    let mut rng = Rng::new(42);

    let soa1: Soa<_> = rng.collect_vec4(1 << 16);
    let soa2: Soa<_> = rng.collect_vec4(1 << 16);
    c.bench_function("soa", |b| {
        b.iter(|| {
            soa1.iter()
                .zip(soa2.iter())
                .map(|(a, b)| a.dot(&b))
                .sum::<f32>()
        })
    });

    let vec1: Vec<_> = rng.collect_vec4(1 << 16);
    let vec2: Vec<_> = rng.collect_vec4(1 << 16);
    c.bench_function("vec", |b| {
        b.iter(|| {
            vec1.iter()
                .zip(vec2.iter())
                .map(|(a, b)| a.dot(b))
                .sum::<f32>()
        })
    });

    c.bench_function("chunked-soa", |b| {
        b.iter(|| {
            soa1.chunks_exact(8)
                .zip(soa2.chunks_exact(8))
                .fold([0.; 8], |acc, (a, b)| {
                    core::array::from_fn(|i| acc[i] + a.idx(i).dot(&b.idx(i)))
                })
                .into_iter()
                .sum::<f32>()
        })
    });

    #[rustfmt::skip]
    c.bench_function("chunked-vec", |b| {
        b.iter(|| {
            vec1.chunks_exact(8).zip(vec2.chunks_exact(8)).fold(
                [0.; 8],
                |acc, (a, b)| {
                    core::array::from_fn(|i| {
                        acc[i] + a[i].dot(&b[i])
                    })
                },
            ).into_iter().sum::<f32>()
        })
    });
}

fn bench_serde_deserialize(c: &mut Criterion) {
    #[derive(Soars, serde::Deserialize, Clone, Copy)]
    #[soa_derive(include(Ref), serde::Serialize)]
    struct Point {
        x: f32,
        y: f32,
        z: f32,
    }

    let soa: Soa<Point> = (0..4096)
        .map(|i| Point {
            x: i as f32,
            y: i as f32 * 2.,
            z: i as f32 * 3.,
        })
        .collect();
    let json = serde_json::to_string(&soa).unwrap();

    let mut group = c.benchmark_group("serde_deserialize");
    group.bench_function("json_4096", |b| {
        b.iter(|| {
            let _: Soa<Point> = serde_json::from_str(&json).unwrap();
        })
    });
    group.finish();
}

fn bench_truncate(c: &mut Criterion) {
    #[derive(Soars, Clone, Copy)]
    struct Item {
        x: f64,
        y: f64,
        z: f64,
        w: f64,
    }

    let mut group = c.benchmark_group("truncate");
    group.bench_function("1024_items", |b| {
        b.iter_batched(
            || {
                (0u64..1024)
                    .map(|i| Item {
                        x: i as f64,
                        y: i as f64,
                        z: i as f64,
                        w: i as f64,
                    })
                    .collect::<Soa<_>>()
            },
            |mut soa| {
                soa.truncate(0);
                soa
            },
            criterion::BatchSize::SmallInput,
        )
    });
    group.finish();
}

fn bench_append(c: &mut Criterion) {
    const N: usize = 1 << 14;
    let mut group = c.benchmark_group("append");
    group.bench_function("two_16k_vecs4", |b| {
        b.iter_batched(
            || {
                let a: Soa<Vec4> = (0..N).map(|_| Vec4(1., 2., 3., 4.)).collect();
                let b: Soa<Vec4> = (0..N).map(|_| Vec4(5., 6., 7., 8.)).collect();
                (a, b)
            },
            |(mut a, mut b)| {
                a.append(&mut b);
                a
            },
            criterion::BatchSize::LargeInput,
        )
    });
    group.finish();
}

criterion_group!(benches, criterion_benchmark, bench_serde_deserialize, bench_truncate, bench_append);
criterion_main!(benches);
