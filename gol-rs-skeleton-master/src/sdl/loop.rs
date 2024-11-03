use crate::args::Args;
use crate::gol::event::{Event, State};
use crate::sdl::window::Window;
use crate::util::avgturns::AvgTurns;
use anyhow::Result;
use flume::{Receiver, Sender};
use sdl2::keyboard::Keycode;
use sdl2::event::Event as SdlEvent;
use tokio::select;
use std::time::Duration;

pub async fn run(
    args: Args,
    events: Receiver<Event>,
    key_presses: Sender<Keycode>
) -> Result<()> {
    let mut sdl = Window::new(
        "Gol GUI",
        args.image_width as u32,
        args.image_height as u32,
    )?;

    let mut event_pump = sdl.take_event_pump()?;
    let mut dirty = false;
    let mut refresh_interval = tokio::time::interval(
        Duration::from_secs_f64(1_f64 / args.fps as f64)
    );
    let mut avg_turns = AvgTurns::new();

    'sdl: loop {
        select! {
            _ = refresh_interval.tick() => {
                match event_pump.poll_event() {
                    Some(SdlEvent::Quit { .. } | SdlEvent::KeyDown { keycode: Some(Keycode::Escape), ..}) =>
                        key_presses.send_async(Keycode::Q).await?,
                    Some(SdlEvent::KeyDown { keycode: Some(Keycode::P), .. }) =>
                        key_presses.send_async(Keycode::P).await?,
                    Some(SdlEvent::KeyDown { keycode: Some(Keycode::S), .. }) =>
                        key_presses.send_async(Keycode::S).await?,
                    Some(SdlEvent::KeyDown { keycode: Some(Keycode::Q), .. }) =>
                        key_presses.send_async(Keycode::Q).await?,
                    Some(SdlEvent::KeyDown { keycode: Some(Keycode::K), .. }) =>
                        key_presses.send_async(Keycode::K).await?,
                    _ => (),
                }
                if dirty {
                    sdl.render_frame()?;
                    dirty = false;
                }
            },
            gol_event = events.recv_async() => {
                match gol_event {
                    Ok(Event::CellFlipped { cell, .. }) =>
                        sdl.flip_pixel(cell.x as u32, cell.y as u32),
                    Ok(Event::CellsFlipped { cells, ..}) =>
                        cells.iter().for_each(|cell| sdl.flip_pixel(cell.x as u32, cell.y as u32)),
                    Ok(Event::TurnComplete { .. }) =>
                        dirty = true,
                    Ok(Event::AliveCellsCount { completed_turns, .. }) =>
                        log::info!(
                            target: "Event", "{} Avg{:>5} turns/s",
                            gol_event?,
                            avg_turns.get(completed_turns)
                        ),
                    Ok(Event::ImageOutputComplete { .. }) =>
                        log::info!(target: "Event", "{}", gol_event?),
                    Ok(Event::FinalTurnComplete { .. }) =>
                        log::info!(target: "Event", "{}", gol_event?),
                    Ok(Event::StateChange { new_state, .. }) => {
                        log::info!(target: "Event", "{}", gol_event?);
                        if let State::Quitting = new_state {
                            break 'sdl
                        }
                    },
                    Err(_) => break 'sdl,
                };
            }
        }
    }

    Ok(())
}

pub async fn run_headless(events: Receiver<Event>) -> Result<()> {
    let mut avg_turns = AvgTurns::new();
    loop {
        let gol_event = events.recv_async().await;
        match gol_event {
            Ok(Event::AliveCellsCount { completed_turns, .. }) =>
                log::info!(
                    target: "Event", "{} Avg{:>5} turns/s",
                    gol_event?,
                    avg_turns.get(completed_turns)
                ),
            Ok(Event::ImageOutputComplete { .. }) =>
                log::info!(target: "Event", "{}", gol_event?),
            Ok(Event::FinalTurnComplete { .. }) =>
                log::info!(target: "Event", "{}", gol_event?),
            Ok(Event::StateChange { new_state, .. }) => {
                log::info!(target: "Event", "{}", gol_event?);
                if let State::Quitting = new_state {
                    break
                }
            },
            Err(_) => break,
            _ => (),
        };
    }
    Ok(())
}
