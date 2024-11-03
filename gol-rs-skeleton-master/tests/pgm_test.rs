use anyhow::Result;
use clap::{Command, value_parser, Arg};
use colored::Colorize;
use gol_rs::args::Args;
use gol_rs::util::logger;
use gol_rs::gol::{Params, self, event::{Event, State}};
use log::Level;
use sdl2::keyboard::Keycode;
use utils::{io::read_alive_cells, visualise::assert_eq_board};

mod utils;

#[tokio::main]
async fn main() {
    let start = std::time::Instant::now();
    logger::set_panic_hook();
    logger::init(Level::Debug, false);
    let command = Command::new("Gol Test")
        .arg(Arg::new("threads")
            .short('t')
            .long("threads")
            .required(false)
            .default_value("16")
            .value_parser(value_parser!(usize)))
        .get_matches();
    let threads = command.get_one::<usize>("threads").unwrap().to_owned();
    assert!(threads > 0, "Threads for testing should be greater than 0");
    let args = Args::default().threads(threads);

    let passed_tests = test_pgm(args).await.unwrap();

    println!(
        "\ntest result: {}. {} passed; finished in {:.2}s\n",
        "ok".green(),
        passed_tests,
        start.elapsed().as_secs_f32()
    );
    std::process::exit(0);
}

/// Pgm tests 16x16, 64x64 and 512x512 image output files on 0, 1 and 100 turns using 1-16 worker threads.
async fn test_pgm(args: Args) -> Result<usize> {
    let mut passed_test = 0;
    let size = [(16_usize, 16_usize), (64, 64), (512, 512)];
    let turns = [0_usize, 1, 100];

    for (width, height) in size {
        for expected_turns in turns {
            let path = format!(
                "check/images/{}x{}x{}.pgm",
                width,
                height,
                expected_turns
            );
            let expected_alive = read_alive_cells(path, width, height).unwrap();
            for thread in 1..=args.threads {
                let args = args.clone()
                    .turns(expected_turns)
                    .threads(thread)
                    .image_width(width)
                    .image_height(height);
                log::debug!(target: "Test", "{} - {:?}", "Testing Pgm".cyan(), Params::from(args.clone()));
                let (_key_presses_tx, key_presses_rx) = flume::bounded::<Keycode>(10);
                let (events_tx, events_rx) = flume::bounded::<Event>(1000);
                tokio::spawn(gol::run(args.clone(), events_tx, key_presses_rx));
                loop {
                    if let Ok(Event::StateChange { new_state: State::Quitting, .. }) = events_rx.recv_async().await {
                        break
                    }
                }
                let path = format!("out/{}x{}x{}.pgm", width, height, expected_turns);
                let output = read_alive_cells(path, args.image_width, args.image_height).unwrap();
                assert_eq_board(args, &output, &expected_alive);
                passed_test += 1;
            }
        }
    }
    Ok(passed_test)
}
