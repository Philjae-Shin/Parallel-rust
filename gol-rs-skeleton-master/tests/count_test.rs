use anyhow::Result;
use flume::{Receiver, Sender};
use core::panic;
use std::time::Duration;
use log::Level;
use colored::Colorize;
use gol_rs::args::Args;
use gol_rs::util::logger;
use gol_rs::gol::{Params, self, event::{Event, State}};
use sdl2::keyboard::Keycode;
use utils::{common::deadline, io::read_alive_counts};

mod utils;

#[tokio::main]
async fn main() {
    let start = std::time::Instant::now();
    logger::set_panic_hook();
    logger::init(Level::Debug, false);
    let args = Args::default()
        .turns(100000000)
        .threads(1)
        .image_width(512)
        .image_height(512);

    // Since key press handling and exit routines are not yet implemented
    // channels are initialised here to ensure they are not dropped before the program exits
    let (_key_presses_tx, key_presses_rx) = flume::bounded::<Keycode>(10);
    let (events_tx, events_rx) = flume::bounded::<Event>(1000);
    let passed_tests = test_alive(
        args,
        key_presses_rx,
        events_tx.clone(),
        events_rx.clone(),
    ).await.unwrap();

    println!(
        "\ntest result: {}. {} passed; finished in {:.2}s\n",
        "ok".green(),
        passed_tests,
        start.elapsed().as_secs_f32()
    );
    std::process::exit(0);
}

/// Count tests will automatically check the 512x512 cell counts for the first 5 messages.
/// You can manually check your counts by looking at CSVs provided in check/alive
async fn test_alive(
    args: Args,
    key_presses_rx: Receiver<Keycode>,
    events_tx: Sender<Event>,
    events_rx: Receiver<Event>,
) -> Result<usize> {
    let passed_tests = 1;
    log::debug!(target: "Test", "{} - {:?}", "Testing Alive Count".cyan(), Params::from(args.clone()));

    let alive_map = read_alive_counts(512, 512).unwrap();
    tokio::spawn(gol::run(args, events_tx.clone(), key_presses_rx));

    let mut ddl = deadline(
        Duration::from_secs(5),
        "No AliveCellsCount event received in 5 seconds"
    );

    let mut succeed = 0;
    loop {
        let event = events_rx.recv_async().await;
        match event {
            Ok(Event::AliveCellsCount { completed_turns, cells_count }) => {
                if completed_turns == 0 {
                    continue
                }
                ddl.abort();

                let expected = if completed_turns <= 10000 {
                    *alive_map.get(&completed_turns).unwrap()
                } else if completed_turns % 2 == 0 { 5565 } else { 5567 };

                assert_eq!(
                    cells_count, expected,
                    "At turn {} expected {} alive cells, got {} instead",
                    completed_turns, expected, cells_count
                );
                succeed += 1;

                log::debug!(
                    target: "Test",
                    "Complete Turns {:<8} Alive Cells {:<8}",
                    completed_turns.to_string().bright_green(),
                    cells_count.to_string().bright_green()
                );

                if succeed < 5 {
                    ddl = deadline(
                        Duration::from_secs(3),
                        "No AliveCellsCount event received in 3 seconds"
                    );
                } else {
                    break
                }
            },
            Ok(Event::StateChange { new_state: State::Quitting, .. }) if succeed >= 5 => break,
            Err(_) => panic!("Not enough AliveCellsCount events received"),
            _ => (),
        }
    }
    Ok(passed_tests)
}
