use crate::gol::event::{Event, State};
use crate::gol::Params;
use crate::gol::io::IoCommand;
use crate::util::cell::{CellCoord, CellValue};
use anyhow::{Context, Result};
use flume::{Receiver, Sender};
use sdl2::keyboard::Keycode;
use std::sync::{Arc, Mutex};
use std::thread;

pub struct DistributorChannels {
    pub events: Option<Sender<Event>>,
    pub key_presses: Option<Receiver<Keycode>>,
    pub io_command: Option<Sender<IoCommand>>,
    pub io_idle: Option<Receiver<bool>>,
    pub io_filename: Option<Sender<String>>,
    pub io_input: Option<Receiver<CellValue>>,
    pub io_output: Option<Sender<CellValue>>,
}

pub fn distributor(params: Params, mut channels: DistributorChannels) -> Result<()> {
    let events = channels.events.take().unwrap();
    let io_command = channels.io_command.take().unwrap();
    let io_idle = channels.io_idle.take().unwrap();

    let width = params.image_width;
    let height = params.image_height;
    let total_cells = width * height;
    let world = Arc::new(Mutex::new(vec![CellValue::Dead; total_cells]));

    io_command.send(IoCommand::IoInput)?;
    for y in 0..height {
        for x in 0..width {
            let cell = channels
                .io_input
                .as_ref()
                .context("The input channel is None")?
                .recv()?;
            let index = y * width + x;
            world.lock().unwrap()[index] = cell;
        }
    }

    let turns = params.turns as u32;
    let threads = params.threads;
    let height_usize = height;

    let rows_per_thread = height_usize / threads;
    let extra_rows = height_usize % threads;

    let initial_turn: u32 = 0;
    events.send(Event::StateChange {
        completed_turns: initial_turn,
        new_state: State::Executing,
    })?;

    for turn in 1..=turns {
        let mut handles = Vec::new();
        let next_world = Arc::new(Mutex::new(vec![CellValue::Dead; total_cells]));

        for thread_id in 0..threads {
            let world_clone = Arc::clone(&world);
            let next_world_clone = Arc::clone(&next_world);
            let width = width;
            let height = height_usize;

            let start_row = thread_id * rows_per_thread + usize::min(thread_id, extra_rows);
            let mut end_row = start_row + rows_per_thread;
            if thread_id < extra_rows {
                end_row += 1;
            }
            if end_row > height {
                end_row = height;
            }

            let handle = thread::spawn(move || {
                for y in start_row..end_row {
                    for x in 0..width {
                        let index = y * width + x;
                        let cell = world_clone.lock().unwrap()[index];
                        let alive_neighbors =
                            count_alive_neighbors(&world_clone, width, height, x, y);
                        let new_cell = match cell {
                            CellValue::Alive => {
                                if alive_neighbors == 2 || alive_neighbors == 3 {
                                    CellValue::Alive
                                } else {
                                    CellValue::Dead
                                }
                            }
                            CellValue::Dead => {
                                if alive_neighbors == 3 {
                                    CellValue::Alive
                                } else {
                                    CellValue::Dead
                                }
                            }
                        };
                        next_world_clone.lock().unwrap()[index] = new_cell;
                    }
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        let mut flipped_cells = Vec::new();
        {
            let mut current_world = world.lock().unwrap();
            let new_world = next_world.lock().unwrap();
            for y in 0..height_usize {
                for x in 0..width {
                    let index = y * width + x;
                    if current_world[index] != new_world[index] {
                        flipped_cells.push(CellCoord { x, y });
                        current_world[index] = new_world[index];
                    }
                }
            }
        }

        if !flipped_cells.is_empty() {
            events.send(Event::CellsFlipped {
                completed_turns: turn,
                cells: flipped_cells,
            })?;
        }

        events.send(Event::TurnComplete { completed_turns: turn })?;
    }

    let alive_cells = {
        let current_world = world.lock().unwrap();
        current_world
            .iter()
            .enumerate()
            .filter_map(|(idx, &cell)| {
                if cell == CellValue::Alive {
                    Some(CellCoord {
                        x: (idx % width) as usize,
                        y: (idx / width) as usize,
                    })
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
    };

    events.send(Event::FinalTurnComplete {
        completed_turns: turns,
        alive: alive_cells,
    })?;

    io_command.send(IoCommand::IoCheckIdle)?;
    io_idle.recv()?;

    events.send(Event::StateChange {
        completed_turns: turns,
        new_state: State::Quitting,
    })?;

    Ok(())
}

fn count_alive_neighbors(
    world: &Arc<Mutex<Vec<CellValue>>>,
    width: usize,
    height: usize,
    x: usize,
    y: usize,
) -> u32 {
    let mut count = 0;
    for dy in [-1isize, 0, 1].iter().cloned() {
        for dx in [-1isize, 0, 1].iter().cloned() {
            if dx == 0 && dy == 0 {
                continue;
            }
            let nx = x as isize + dx;
            let ny = y as isize + dy;
            if nx >= 0 && nx < width as isize && ny >= 0 && ny < height as isize {
                let index = ny as usize * width + nx as usize;
                if world.lock().unwrap()[index] == CellValue::Alive {
                    count += 1;
                }
            }
        }
    }
    count
}
