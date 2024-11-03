use anyhow::Result;
use core::panic;
use clap::{Command, Arg, value_parser};
use colored::Colorize;
use log::Level;
use gol_rs::{args::Args, gol::{self, event::{Event, State}, Params}, util::logger};
use sdl2::keyboard::Keycode;
use utils::{visualise::assert_eq_board, io::read_alive_cells};

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

    let passed_tests = test_gol(args).await.unwrap();

    println!(
        "\ntest result: {}. {} passed; finished in {:.2}s\n",
        "ok".green(),
        passed_tests,
        start.elapsed().as_secs_f32()
    );
    std::process::exit(0);
}

/// Gol tests 16x16, 64x64 and 512x512 images on 0, 1 and 100 turns using 1-16 worker threads.
async fn test_gol(args: Args) -> Result<usize> {
    let mut passed_tests = 0;
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
                let args = args
                    .clone()
                    .turns(expected_turns)
                    .threads(thread)
                    .image_width(width)
                    .image_height(height);
                log::debug!(target: "Test", "{} - {:?}", "Testing Gol".cyan(), Params::from(args.clone()));
                let (_key_presses_tx, key_presses_rx) = flume::bounded::<Keycode>(10);
                let (events_tx, events_rx) = flume::bounded::<Event>(1000);
                tokio::spawn(gol::run(args.clone(), events_tx, key_presses_rx));
                let mut final_turn_complete = false;
                loop {
                    match events_rx.recv_async().await {
                        Ok(Event::FinalTurnComplete { completed_turns, alive }) => {
                            final_turn_complete = true;
                            assert_eq!(
                                completed_turns, expected_turns as u32,
                                "Expected completed turns is {}, but got {}", expected_turns, completed_turns
                            );
                            assert_eq_board(args.clone(), &alive, &expected_alive);
                        },
                        Ok(Event::StateChange { new_state: State::Quitting, .. }) if final_turn_complete => break,
                        Err(_) => panic!("No FinalTurnComplete events received {:?}", Params::from(args)),
                        _ => (),
                    };

                }
                passed_tests += 1;
            }

        }
    }
    Ok(passed_tests)
}
