use std::{collections::HashMap, time::Duration, future::Future};
use anyhow::Result;
use clap::{value_parser, Arg, ArgAction, Command};
use colored::Colorize;
use flume::{Receiver, Sender};
use gol_rs::{args::Args, gol::{self, event::{Event, State}, Params}, util::{cell::{CellCoord, CellValue}, logger}};
use log::Level;
use sdl2::keyboard::Keycode;
use tokio::select;
use utils::{common::deadline, io::{read_alive_cells, read_alive_counts}, sdl, visualise::assert_eq_board};

mod utils;

#[tokio::main]
async fn main() {
    let start = std::time::Instant::now();
    logger::set_panic_hook();
    logger::init(Level::Debug, false);
    let command = Command::new("Gol Test")
        .arg(Arg::new("sdl")
            .long("sdl")
            .required(false)
            .default_value("false")
            .action(ArgAction::SetTrue)
            .value_parser(value_parser!(bool)))
        .get_matches();
    let sdl = command.get_one::<bool>("sdl").unwrap().to_owned();
    let args = Args::default().headless(!sdl);

    let passed_tests = test_sdl(args).await.unwrap();

    println!(
        "\ntest result: {}. {} passed; finished in {:.2}s\n",
        "ok".green(),
        passed_tests,
        start.elapsed().as_secs_f32()
    );
    std::process::exit(0);
}

/// Sdl tests program behaviour on key presses
async fn test_sdl(args: Args) -> Result<usize> {
    let args = args
        .turns(100000000)
        .threads(8)
        .image_width(512)
        .image_height(512);
    let passed_tests = 1;
    log::debug!(target: "Test", "{} - {:?}", "Testing Sdl".cyan(), Params::from(args.clone()));

    let (key_presses_tx, key_presses_rx) = flume::bounded::<Keycode>(10);
    let (key_presses_forward_tx, key_presses_forward_rx) = flume::bounded::<Keycode>(10);
    let (events_tx, events_rx) = flume::bounded::<Event>(1000);
    let (events_forward_tx, events_forward_rx) = flume::bounded::<Event>(1000);
    let (gol_done_tx, gol_done_rx) = flume::bounded::<()>(1);

    let gol = tokio::spawn({
        let args = args.clone();
        async move {
            gol::run(args, events_tx, key_presses_forward_rx).await.unwrap();
            gol_done_tx.send_async(()).await.unwrap();
            Ok(())
        }
    });
    let tester = tokio::spawn(
        Tester::start(args.clone(), key_presses_tx, events_forward_rx, gol_done_rx));
    let (gol, sdl, tester) = if args.headless {
        let sdl = sdl::run_headless(
            events_rx,
            key_presses_rx,
            events_forward_tx,
            key_presses_forward_tx
        );
        tokio::join!(gol, sdl, tester)
    } else {
        let sdl = sdl::run(
            args,
            "Gol GUI - Test Sdl",
            events_rx,
            key_presses_rx,
            events_forward_tx,
            key_presses_forward_tx
        );
        tokio::join!(gol, sdl, tester)
    };
    sdl.and(gol?).and(tester?).and(Ok(passed_tests))
}

struct Tester {
    args: Args,
    key_presses: Sender<Keycode>,
    events: Receiver<Event>,
    events_watcher: Receiver<Event>,
    turn: u32,
    world: Vec<Vec<CellValue>>,
    alive_map: HashMap<u32, u32>,
}

impl Tester {
    async fn start(
        args: Args,
        key_presses: Sender<Keycode>,
        events: Receiver<Event>,
        gol_done: Receiver<()>,
    ) -> Result<()> {
        let (watcher_tx, watcher_rx) = flume::unbounded::<Event>();
        let mut tester = Tester {
            args: args.clone(),
            key_presses,
            events,
            events_watcher: watcher_rx,
            turn: 0,
            world: vec![vec![CellValue::Dead; args.image_width]; args.image_height],
            alive_map: read_alive_counts(args.image_width as u32, args.image_height as u32)?,
        };

        tokio::spawn(tester.test_pause(Duration::from_secs(3)));
        tokio::spawn(tester.test_output(Duration::from_secs(12)));
        let quitting = tokio::spawn(tester.test_quitting(Duration::from_secs(16)));
        let deadline = deadline(
            Duration::from_secs(25),
            "Your program should complete this test within 20 seconds. Is your program deadlocked?"
        );

        let mut cell_flipped_received = false;
        let mut turn_complete_received = false;

        loop {
            select! {
                gol_event = tester.events.recv_async() => {
                    match gol_event {
                        Ok(Event::CellFlipped { completed_turns, cell }) => {
                            cell_flipped_received = true;
                            assert!(completed_turns == tester.turn || completed_turns == tester.turn + 1,
                                "Expected completed {} turns, got {} instead", tester.turn, completed_turns);
                            tester.world[cell.y][cell.x].flip();
                        },
                        Ok(Event::CellsFlipped { completed_turns, cells }) => {
                            cell_flipped_received = true;
                            assert!(completed_turns == tester.turn || completed_turns == tester.turn + 1,
                                "Expected completed {} turns, got {} instead", tester.turn, completed_turns);
                            cells.iter().for_each(|cell| tester.world[cell.y][cell.x].flip());
                        },
                        Ok(Event::TurnComplete { completed_turns }) => {
                            turn_complete_received = true;
                            tester.turn += 1;
                            assert_eq!(completed_turns, tester.turn,
                                "Expected completed {} turns, got {} instead", tester.turn, completed_turns);
                            tester.test_alive();
                            tester.test_gol();
                        },
                        e @ Ok(Event::ImageOutputComplete { .. }) => watcher_tx.send_async(e?).await?,
                        e @ Ok(Event::StateChange { .. }) => watcher_tx.send_async(e?).await?,
                        e @ Ok(Event::FinalTurnComplete { .. }) => watcher_tx.send_async(e?).await?,
                        Ok(_) => (),
                        Err(_) => {
                            if !cell_flipped_received {
                                panic!("No CellFlipped events received");
                            }
                            if !turn_complete_received {
                                panic!("No TurnComplete events received");
                            }
                            quitting.await?;
                            gol_done.recv_async().await?;
                            deadline.abort();
                            break
                        },
                    }
                },
            }
        }

        Ok(())
    }

    fn test_alive(&self) {
        let alive_count = self.world.iter()
            .flatten().filter(|&&cell| cell.is_alive()).count();
        let expected = if self.turn <= 10000 { *self.alive_map.get(&self.turn).unwrap() }
            else if self.turn % 2 == 0 { 5565 } else { 5567 };
        assert_eq!(
            alive_count, expected as usize,
            "At turn {} expected {} alive cells, got {} instead", self.turn, expected, alive_count
        );
    }

    fn test_gol(&self) {
        if self.turn == 0 || self.turn == 1 || self.turn == 100 {
            let path = format!(
                "check/images/{}x{}x{}.pgm",
                self.args.image_width,
                self.args.image_height,
                self.turn
            );
            let expected_alive = read_alive_cells(
                path,
                self.args.image_width,
                self.args.image_height
            ).unwrap();

            let alive_cells = self.world.iter().enumerate()
                .flat_map(|(y, row)|
                    row.iter().enumerate()
                        .filter(|&(_, &cell)| cell.is_alive())
                        .map(move |(x, _)| CellCoord::new(x, y)))
                .collect::<Vec<CellCoord>>();
            assert_eq_board(self.args.clone(), &alive_cells, &expected_alive);
        }
    }

    fn test_output(&self, delay: Duration) -> impl Future<Output = ()> {
        let key_presses = self.key_presses.clone();
        let event_watcher = self.events_watcher.clone();
        let (width, height) = (self.args.image_width, self.args.image_height);
        async move {
            tokio::time::sleep(delay).await;
            log::debug!(target: "Test", "{}", "Testing image output".cyan());
            event_watcher.drain();
            key_presses.send_async(Keycode::S).await.unwrap();
            tokio::time::timeout(Duration::from_secs(4), async {
                while let Ok(event) = event_watcher.recv_async().await {
                    if let Event::ImageOutputComplete { completed_turns, filename } = event {
                        assert_eq!(
                            filename.to_owned(),
                            format!("{}x{}x{}", width, height, completed_turns),
                            "Filename is not correct"
                        );
                        break;
                    }
                }
            }).await.expect("No ImageOutput events received in 4 seconds");
        }
    }

    fn test_pause(&self, delay: Duration) -> impl Future<Output = ()> {
        let key_presses = self.key_presses.clone();
        let event_watcher = self.events_watcher.clone();
        let test_output = self.test_output(Duration::from_secs(2));
        async move {
            tokio::time::sleep(delay).await;
            log::debug!(target: "Test", "{}", "Testing Pause key pressed".cyan());
            event_watcher.drain();
            key_presses.send_async(Keycode::P).await.unwrap();
            tokio::time::timeout(Duration::from_secs(2), async {
                while let Ok(event) = event_watcher.recv_async().await {
                    if let Event::StateChange { new_state: State::Pause, .. } = event { break }
                }
            }).await.expect("No Pause events received in 2 seconds");

            test_output.await;

            tokio::time::sleep(Duration::from_secs(2)).await;
            log::debug!(target: "Test", "{}", "Testing Pause key pressed again".cyan());
            event_watcher.drain();
            key_presses.send_async(Keycode::P).await.unwrap();
            tokio::time::timeout(Duration::from_secs(2), async {
                while let Ok(event) = event_watcher.recv_async().await {
                    if let Event::StateChange { new_state: State::Executing, .. } = event { break }
                }
            }).await.expect("No Executing events received in 2 seconds");
        }
    }

    fn test_quitting(&self, delay: Duration) -> impl Future<Output = ()> {
        let key_presses = self.key_presses.clone();
        let event_watcher = self.events_watcher.clone();
        async move {
            tokio::time::sleep(delay).await;
            log::debug!(target: "Test", "{}", "Testing Quit key pressed".cyan());
            event_watcher.drain();
            key_presses.send_async(Keycode::Q).await.unwrap();
            tokio::time::timeout(Duration::from_secs(2), async {
                while let Ok(event) = event_watcher.recv_async().await {
                    if let Event::FinalTurnComplete { .. } = event { break }
                }
            }).await.expect("No FinalTurnComplete events received in 2 seconds");

            tokio::time::timeout(Duration::from_secs(4), async {
                while let Ok(event) = event_watcher.recv_async().await {
                    if let Event::ImageOutputComplete { .. } = event { break }
                }
            }).await.expect("No ImageOutputComplete events received in 4 seconds");

            tokio::time::timeout(Duration::from_secs(2), async {
                while let Ok(event) = event_watcher.recv_async().await {
                    if let Event::StateChange { new_state: State::Quitting, .. } = event { break }
                }
            }).await.expect("No StateChange Quitting events received in 2 seconds");
        }
    }

}
