use crate::gol::event::{Event, State};
use crate::gol::Params;
use crate::gol::io::IoCommand;
use crate::util::cell::CellValue;
use anyhow::Result;
use flume::{Receiver, Sender};
use sdl2::keyboard::Keycode;

pub struct DistributorChannels {
    pub events: Option<Sender<Event>>,
    pub key_presses: Option<Receiver<Keycode>>,
    pub io_command: Option<Sender<IoCommand>>,
    pub io_idle: Option<Receiver<bool>>,
    pub io_filename: Option<Sender<String>>,
    pub io_input: Option<Receiver<CellValue>>,
    pub io_output: Option<Sender<CellValue>>,
}

pub fn distributor(
    params: Params,
    mut channels: DistributorChannels
) -> Result<()> {
    let events = channels.events.take().unwrap();
    let key_presses = channels.key_presses.take().unwrap();
    let io_command = channels.io_command.take().unwrap();
    let io_idle = channels.io_idle.take().unwrap();

    // TODO: Create a 2D vector to store the world.

    let turn = 0;
    events.send(
        Event::StateChange { completed_turns: turn, new_state: State::Executing })?;

    // TODO: Execute all turns of the Game of Life.

    // TODO: Report the final state using FinalTurnCompleteEvent.


    // Make sure that the Io has finished any output before exiting.
    io_command.send(IoCommand::IoCheckIdle)?;
    io_idle.recv()?;

    events.send(
        Event::StateChange { completed_turns: turn, new_state: State::Quitting })?;
    Ok(())
}
