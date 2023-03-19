use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use std::{hint::black_box, path::Path};

pub fn criterion_benchmark(c: &mut Criterion) {
    let files = [
        Path::new("benches/files/fonts.conf"),
        Path::new("benches/files/huge.xml"),
        Path::new("benches/files/large.plist"),
        Path::new("benches/files/medium.svg"),
    ];

    let names = files.iter().map(|path| {
        path.file_name()
            .expect("path has no filename")
            .to_str()
            .expect("failed to convert the path file into a string")
    });

    let data = files
        .iter()
        .map(|path| std::fs::read_to_string(path).expect("file not found"));

    for (name, data) in names.zip(data) {
        let mut group = c.benchmark_group("Fibonacci");
        group.throughput(Throughput::Bytes(data.len() as u64));
        group.sample_size(50);
        group.bench_function(name, |c| {
            c.iter(|| {
                for event in xml1::XmlIter::from(data.as_str()) {
                    black_box(event);
                }
            });
        });
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
