use clap::Parser;
use flume::Sender;
use log::Level;
use sdl2::keyboard::Keycode;
use tokio::try_join;
use gol_rs::args::Args;
use gol_rs::gol::{self, event::Event};
use gol_rs::sdl;
use gol_rs::util::logger;

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    let args = Args::parse();
    logger::init(Level::Info, false);

    log::info!(target: "Main", "{:<10} {}", "Threads", args.threads);
    log::info!(target: "Main", "{:<10} {}", "Width", args.image_width);
    log::info!(target: "Main", "{:<10} {}", "Height", args.image_height);
    log::info!(target: "Main", "{:<10} {}", "Turns", args.turns);

    let (key_presses_tx, key_presses_rx) = flume::bounded::<Keycode>(10);
    let (events_tx, events_rx) = flume::bounded::<Event>(1000);

    tokio::spawn(sigint(key_presses_tx.clone()));

    if !args.headless {
        try_join!(
            gol::run(args.clone(), events_tx, key_presses_rx),
            sdl::r#loop::run(args, events_rx, key_presses_tx)
        ).unwrap();
    } else {
        try_join!(
            gol::run(args, events_tx, key_presses_rx),
            sdl::r#loop::run_headless(events_rx)
        ).unwrap();
    }
}

async fn sigint(key_presses_tx: Sender<Keycode>) {
    tokio::signal::ctrl_c().await.unwrap();
    key_presses_tx.send_async(Keycode::Q).await.unwrap();
}
