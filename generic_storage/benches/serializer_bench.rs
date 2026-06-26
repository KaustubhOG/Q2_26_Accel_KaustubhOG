use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use generic_storage::{
    models::Person,
    serializers::{Borsh, Bincode, Json},
    storage::Storage,
};

fn bench_save(c: &mut Criterion) {
    let person = Person::new("André", 30);
    let mut group = c.benchmark_group("save");

    group.bench_with_input(BenchmarkId::new("borsh", "Person"), &person, |b, p| {
        let mut s: Storage<Person, Borsh> = Storage::new();
        b.iter(|| s.save(black_box(p)).unwrap());
    });

    group.bench_with_input(BenchmarkId::new("bincode", "Person"), &person, |b, p| {
        let mut s: Storage<Person, Bincode> = Storage::new();
        b.iter(|| s.save(black_box(p)).unwrap());
    });

    group.bench_with_input(BenchmarkId::new("json", "Person"), &person, |b, p| {
        let mut s: Storage<Person, Json> = Storage::new();
        b.iter(|| s.save(black_box(p)).unwrap());
    });

    group.finish();
}

fn bench_load(c: &mut Criterion) {
    let person = Person::new("André", 30);
    let mut group = c.benchmark_group("load");

    group.bench_function("borsh", |b| {
        let mut s: Storage<Person, Borsh> = Storage::new();
        s.save(&person).unwrap();
        b.iter(|| black_box(s.load().unwrap()));
    });

    group.bench_function("bincode", |b| {
        let mut s: Storage<Person, Bincode> = Storage::new();
        s.save(&person).unwrap();
        b.iter(|| black_box(s.load().unwrap()));
    });

    group.bench_function("json", |b| {
        let mut s: Storage<Person, Json> = Storage::new();
        s.save(&person).unwrap();
        b.iter(|| black_box(s.load().unwrap()));
    });

    group.finish();
}

fn bench_round_trip(c: &mut Criterion) {
    let person = Person::new("André", 30);
    let mut group = c.benchmark_group("round_trip");

    group.bench_function("borsh", |b| {
        b.iter(|| {
            let mut s: Storage<Person, Borsh> = Storage::new();
            s.save(black_box(&person)).unwrap();
            black_box(s.load().unwrap())
        });
    });

    group.bench_function("bincode", |b| {
        b.iter(|| {
            let mut s: Storage<Person, Bincode> = Storage::new();
            s.save(black_box(&person)).unwrap();
            black_box(s.load().unwrap())
        });
    });

    group.bench_function("json", |b| {
        b.iter(|| {
            let mut s: Storage<Person, Json> = Storage::new();
            s.save(black_box(&person)).unwrap();
            black_box(s.load().unwrap())
        });
    });

    group.finish();
}

criterion_group!(benches, bench_save, bench_load, bench_round_trip);
criterion_main!(benches);
