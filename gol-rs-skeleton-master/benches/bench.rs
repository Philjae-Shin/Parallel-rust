use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
use gol_rs::{args::Args, gol::{self, event::Event}};
use sdl2::keyboard::Keycode;

fn bench_gol(c: &mut Criterion) {
    let mut group = c.benchmark_group("Gol Benchmark");
    group
        .sampling_mode(criterion::SamplingMode::Flat)
        .sample_size(10);
    for thread in 1..=num_cpus::get() {
        group.bench_with_input(
            BenchmarkId::new("Threads", thread),
            &thread,
            |bencher, thread|
            {
                let runtime = tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()
                    .unwrap();
                bencher.to_async(runtime).iter(|| async {
                    let args = Args::default()
                        .turns(1000)
                        .threads(*thread)
                        .image_width(512)
                        .image_height(512);
                    let (_key_presses_tx, key_presses_rx) = flume::bounded::<Keycode>(10);
                    let (events_tx, events_rx) = flume::bounded::<Event>(1000);
                    tokio::spawn(gol::run(args, events_tx, key_presses_rx));
                    loop {
                        if events_rx.recv_async().await.is_err() {
                            break;
                        }
                    }
                })
            }
        );
    }
    group.finish();
}


criterion_group!(benches, bench_gol);
criterion_main!(benches);
