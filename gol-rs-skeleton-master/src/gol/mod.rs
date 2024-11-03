use crate::args::Args;
use crate::gol::distributor::{DistributorChannels, distributor};
use crate::gol::event::Event;
use crate::gol::io::{start_io, IoChannels};
use anyhow::Result;
use flume::{Receiver, Sender};
use log::Cell;
use io::IoCommand;
use sdl2::keyboard::Keycode;
use crate::util::cell::CellValue;

pub mod distributor;
pub mod event;
pub mod io;

/// `Params` provides the details of how to run the Game of Life and which image to load.
#[derive(Clone, Debug)]
pub struct Params {
    pub turns: usize,
    pub threads: usize,
    pub image_width: usize,
    pub image_height: usize,
}

pub async fn run<P: Into<Params>>(
    params: P,
    events: Sender<Event>,
    key_presses: Receiver<Keycode>,
) -> Result<()> {
    let params: Params = params.into();
    // TODO: Put the missing channels in here.

    let (io_command_tx, io_command_rx) = flume::unbounded::<IoCommand>();
    let (io_idle_tx, io_idle_rx) = flume::unbounded::<bool>();
    // Add missing channels (filename, output, input)
    let (io_filename_tx,io_filename_rx) = flume::unbounded::<String>();
    let (io_input_tx, io_input_rx) = flume::unbounded::<CellValue>();
    let (io_output_tx, io_output_rx) = flume::unbounded::<CellValue>();

    let io_channels = IoChannels {
        command: Some(io_command_rx),
        idle: Some(io_idle_tx),
        // Set missing Channels
        filename: Some(io_filename_rx),
        input: Some(io_input_tx),
        output: Some(io_output_rx),
    };

    tokio::spawn(start_io(params.clone(), io_channels));

    let distributor_channels = DistributorChannels {
        events: Some(events),
        key_presses: Some(key_presses),
        io_command: Some(io_command_tx),
        io_idle: Some(io_idle_rx),
        // Set missing Channels
        io_filename: Some(io_filename_tx),
        io_input: Some(io_input_rx),
        io_output: Some(io_output_tx),
    };

    tokio::task::spawn_blocking(move ||
        distributor(params, distributor_channels)).await??;

    Ok(())
}

impl From<Args> for Params {
    fn from(args: Args) -> Self {
        Params {
            turns: args.turns,
            threads: args.threads,
            image_width: args.image_width,
            image_height: args.image_height,
        }
    }
}
