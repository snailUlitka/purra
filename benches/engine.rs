use std::fs;
use std::hint::black_box;

use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use purra::files::{
    DirectoryOptions, TextFile, collect_directory_files, collect_directory_files_with_options,
    read_text, replace_in_place, write_atomic,
};
use purra::{TextEngine, TextPreset, TextRule};
use tempfile::tempdir;

const MIB: usize = 1024 * 1024;

fn engine_benchmarks(criterion: &mut Criterion) {
    let ai = TextPreset::ai().engine();
    let ascii = TextPreset::ascii().engine();
    let flags = TextEngine::new([TextRule::new('🇺', "🇨"), TextRule::new('🇸', "🇦")]).unwrap();
    let clean_ascii = "plain text and numbers 1234567890\n".repeat(MIB / 34);
    let sparse_unicode = "ordinary text with one em dash — near the end\n".repeat(MIB / 48);
    let dense_unicode = "—’“\u{a0}".repeat(MIB / 10);
    let dense_expansions = "…ﬁ→≠".repeat(MIB / 11);
    let flag_text = "🇺🇸".repeat(MIB / 8);

    let mut group = criterion.benchmark_group("engine_replace");
    for (name, engine, input) in [
        ("clean_ascii", &ai, &clean_ascii),
        ("sparse_ai_unicode", &ai, &sparse_unicode),
        ("dense_ai_unicode", &ai, &dense_unicode),
        ("dense_ascii_expansions", &ascii, &dense_expansions),
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
    let engine = TextPreset::ai().engine();
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

fn directory_filter_benchmarks(criterion: &mut Criterion) {
    let directory = tempdir().unwrap();
    let nested = directory.path().join("nested");
    fs::create_dir(&nested).unwrap();
    fs::write(nested.join(".gitignore"), "*.tmp\n").unwrap();
    for index in 0..16 {
        let child = nested.join(format!("section-{index:02}"));
        fs::create_dir(&child).unwrap();
        fs::write(child.join(".gitignore"), "!keep.tmp\n").unwrap();
        fs::write(child.join("keep.tmp"), "plain text\n").unwrap();
        for file in 0..16 {
            let extension = if file % 2 == 0 { "txt" } else { "tmp" };
            fs::write(
                child.join(format!("file-{file:02}.{extension}")),
                "plain text\n",
            )
            .unwrap();
        }
    }

    let pruning = directory.path().join("pruning");
    let cache = pruning.join("cache");
    fs::create_dir_all(&cache).unwrap();
    fs::write(pruning.join("input.txt"), "plain text\n").unwrap();
    for index in 0..16 {
        let child = cache.join(format!("shard-{index:02}"));
        fs::create_dir(&child).unwrap();
        for file in 0..128 {
            fs::write(child.join(format!("entry-{file:03}.txt")), "plain text\n").unwrap();
        }
    }
    let rules = DirectoryOptions {
        recursive: true,
        ..DirectoryOptions::default()
    };
    let prune = DirectoryOptions {
        ignores: vec!["/cache/".into()],
        ..rules.clone()
    };
    let mut group = criterion.benchmark_group("directory_filters");
    for (name, root, options) in [
        ("nested_gitignore", &nested, &rules),
        ("unpruned_2048_file_subtree", &pruning, &rules),
        ("pruned_2048_file_subtree", &pruning, &prune),
    ] {
        group.bench_function(name, |bencher| {
            bencher.iter(|| {
                collect_directory_files_with_options(black_box(root), black_box(options)).unwrap()
            });
        });
    }
    group.finish();
}

fn atomic_write_benchmarks(criterion: &mut Criterion) {
    let input = "a".repeat(MIB);
    let replacement = "b".repeat(MIB);
    let mut group = criterion.benchmark_group("atomic_write");
    group.throughput(Throughput::Bytes(MIB as u64));

    for backup in [true, false] {
        let name = if backup {
            "with_backup"
        } else {
            "without_backup"
        };
        group.bench_function(name, |bencher| {
            bencher.iter_batched_ref(
                || {
                    let directory = tempdir().unwrap();
                    let path = directory.path().join("large.txt");
                    fs::write(&path, &input).unwrap();
                    (directory, path)
                },
                |(_, path)| {
                    if backup {
                        black_box(replace_in_place(black_box(path), &replacement).unwrap());
                    } else {
                        write_atomic(black_box(path), &replacement, None).unwrap();
                    }
                },
                BatchSize::PerIteration,
            );
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    engine_benchmarks,
    file_benchmarks,
    directory_filter_benchmarks,
    atomic_write_benchmarks
);
criterion_main!(benches);
