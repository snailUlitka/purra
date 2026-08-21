use std::fs;
use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use purra::files::{TextFile, collect_directory_files, read_text};
use purra::{Engine, Preset, Rule};
use tempfile::tempdir;

const MIB: usize = 1024 * 1024;

fn engine_benchmarks(criterion: &mut Criterion) {
    let ai = Preset::ai().engine();
    let flags = Engine::new([Rule::new('🇺', '🇨'), Rule::new('🇸', '🇦')]).unwrap();
    let clean_ascii = "plain text and numbers 1234567890\n".repeat(MIB / 34);
    let sparse_unicode = "ordinary text with one em dash — near the end\n".repeat(MIB / 48);
    let dense_unicode = "—’“\u{a0}".repeat(MIB / 10);
    let flag_text = "🇺🇸".repeat(MIB / 8);

    let mut group = criterion.benchmark_group("engine_replace");
    for (name, engine, input) in [
        ("clean_ascii", &ai, &clean_ascii),
        ("sparse_ai_unicode", &ai, &sparse_unicode),
        ("dense_ai_unicode", &ai, &dense_unicode),
        ("regional_indicators", &flags, &flag_text),
    ] {
        group.throughput(Throughput::Bytes(input.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("replace", name),
            input,
            |bencher, input| {
                bencher.iter(|| engine.replace(black_box(input)));
            },
        );
    }
    group.finish();
}

fn file_benchmarks(criterion: &mut Criterion) {
    let engine = Preset::ai().engine();
    let directory = tempdir().unwrap();
    let input = directory.path().join("large.txt");
    let input_text = "ordinary text with one em dash — near the end\n".repeat(MIB / 48);
    fs::write(&input, input_text).unwrap();

    let entries = directory.path().join("entries");
    fs::create_dir(&entries).unwrap();
    for index in 0..256 {
        fs::write(
            entries.join(format!("document-{index:03}.txt")),
            "plain text\n",
        )
        .unwrap();
    }

    let mut group = criterion.benchmark_group("file_workloads");
    group.throughput(Throughput::Bytes(fs::metadata(&input).unwrap().len()));
    group.bench_function("read_and_replace_1_mib", |bencher| {
        bencher.iter(|| {
            let TextFile::Text(text) = read_text(black_box(&input)).unwrap() else {
                panic!("benchmark fixture must be text");
            };
            black_box(engine.replace(&text));
        });
    });
    group.throughput(Throughput::Elements(256));
    group.bench_function("discover_256_files", |bencher| {
        bencher.iter(|| collect_directory_files(black_box(&entries), false).unwrap());
    });
    group.finish();
}

criterion_group!(benches, engine_benchmarks, file_benchmarks);
criterion_main!(benches);
