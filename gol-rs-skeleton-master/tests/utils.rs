#[allow(dead_code)]
pub mod io {
    use std::{path::Path, collections::HashMap, fs::File};
    use gol_rs::util::cell::{CellCoord, CellValue};
    use image::ImageReader;
    use anyhow::Result;
    use serde::Deserialize;

    pub fn read_alive_cells<P: AsRef<Path>>(
        path: P,
        width: usize,
        height: usize
    ) -> Result<Vec<CellCoord>> {
        let pgm = ImageReader::open(path)?.decode()?;
        assert_eq!(
            pgm.width(),
            width as u32,
            "Incorrect width"
        );
        assert_eq!(
            pgm.height(),
            height as u32,
            "Incorrect height"
        );

        Ok(pgm.into_bytes().chunks(width).enumerate()
            .flat_map(|(y, row)|
                row.iter().enumerate()
                    .filter(|&(_, &cell)| CellValue::from(cell).is_alive())
                    .map(move |(x, _)| CellCoord::new(x, y)))
            .collect()
        )
    }

    #[derive(Debug, Deserialize)]
    struct Check {
        completed_turns: u32,
        alive_cells: u32,
    }

    pub fn read_alive_counts(
        width: u32,
        height: u32
    ) -> Result<HashMap<u32, u32>> {
        let path = format!("check/alive/{}x{}.csv", width, height);
        let mut csv_reader = csv::Reader::from_reader(File::open(path)?);
        let result = csv_reader.deserialize().map(|line| {
            let check: Check = line.unwrap();
            (check.completed_turns, check.alive_cells)
        }).collect();
        Ok(result)
    }
}

#[allow(dead_code)]
pub mod visualise {
    use gol_rs::{args::Args, gol::Params, util::cell::{CellCoord, CellValue}};

    pub fn assert_eq_board(
        args: Args,
        input_cells: &[CellCoord],
        expected_cells: &[CellCoord]
    ) {
        let all_match =
            input_cells.len() == expected_cells.len()
                && expected_cells.iter().all(|cell| input_cells.contains(cell));

        if all_match {
            return
        }

        if args.image_width == 16 && args.image_height == 16 {
            let mut input_matrix = vec![vec![CellValue::Dead; args.image_width]; args.image_height];
            let mut expected_matrix = input_matrix.clone();
            input_cells.iter().for_each(|cell| input_matrix[cell.y][cell.x] = CellValue::Alive);
            expected_cells.iter().for_each(|cell| expected_matrix[cell.y][cell.x] = CellValue::Alive);
            let mut input = matrix_to_strings(&input_matrix);
            let mut expected = matrix_to_strings(&expected_matrix);
            input.insert(0, get_centered_banner(39, "Your result", ' '));
            expected.insert(0, get_centered_banner(39, "Expected result", ' '));
            let output = fold_strings(&[&input, &expected]);
            log::info!(target: "Test", "{}", output);
        }
        panic!("Test Failed - {:?}", Params::from(args));
    }

    fn get_centered_banner(
        len: usize,
        str: &str,
        filling_char: char
    ) -> String {
        assert!(len > str.len(), "string should not be longer than banner");
        let filling = (0..(len - str.len()) / 2).map(|_| filling_char).collect::<String>();
        format!("{}{}{}", filling, str, filling)
    }

    fn fold_strings(items: &[&[String]]) -> String {
        assert!(items.len() > 0, "nothing to fold");
        assert!(
            items.iter().all(|item| item.len() == items[0].len()),
            "items for folding should have same length"
        );
        (0..items[0].len()).fold(String::new(), |output, i| {
            format!(
                "{}\n{}",
                output,
                items.iter().fold(String::new(), |line, item| line + &item[i])
            )
        })
    }

    fn matrix_to_strings(cells: &Vec<Vec<CellValue>>) -> Vec<String> {
        assert!(cells.len() > 0);
        let width = cells[0].len();
        let mut output: Vec<String> = vec![];
        output.push(format!("   ┌{}┐  ", (0..width*2).map(|_| "─").collect::<String>()));
        output.append(&mut cells.iter().enumerate()
            .map(|(y, row)|
                format!("{:2} │{}│  ", y + 1,
                        row.iter().map(|&cell|
                            if cell.is_dead() { "  " } else { "██" }).collect::<String>()))
            .collect());
        output.push(format!("   └{}┘  ", (0..width*2).map(|_| "─").collect::<String>()));
        output
    }
}

#[allow(dead_code)]
pub mod sdl {
    use std::time::Duration;
    use anyhow::Result;
    use flume::{Receiver, Sender};
    use sdl2::keyboard::Keycode;
    use gol_rs::{args::Args, gol::event::{Event, State}, sdl::window::Window, util::avgturns::AvgTurns};
    use tokio::select;

    pub async fn run<T: AsRef<str>>(
        args: Args,
        title: T,
        events: Receiver<Event>,
        key_presses: Receiver<Keycode>,
        events_forward: Sender<Event>,
        key_presses_forward: Sender<Keycode>,
    ) -> Result<()> {
        let mut sdl = Window::new(
            title,
            args.image_width as u32,
            args.image_height as u32,
        )?;
        let fps = 60;
        let mut event_pump = sdl.take_event_pump()?;
        let mut dirty = false;
        let mut refresh_interval = tokio::time::interval(
            Duration::from_secs_f64(1_f64 / fps as f64)
        );
        let mut avg_turns = AvgTurns::new();

        'sdl: loop {
            select! {
                _ = refresh_interval.tick() => {
                    event_pump.poll_event();
                    if dirty {
                        sdl.render_frame()?;
                        dirty = false;
                    }
                },
                key = key_presses.recv_async() => {
                    if let Ok(key) = key {
                        key_presses_forward.send_async(key).await?;
                    }
                },
                gol_event = events.recv_async() => {
                    if let Ok(e) = &gol_event {
                        events_forward.send_async(e.clone()).await?;
                    }
                    match gol_event {
                        Ok(Event::CellFlipped { cell, .. }) =>
                            sdl.flip_pixel(cell.x as u32, cell.y as u32),
                        Ok(Event::CellsFlipped { cells, ..}) =>
                            cells.iter().for_each(|cell| sdl.flip_pixel(cell.x as u32, cell.y as u32)),
                        Ok(Event::TurnComplete { .. }) =>
                            dirty = true,
                        Ok(Event::AliveCellsCount { completed_turns, .. }) =>
                            log::info!(target: "Test", "{} Avg{:>5} turns/s", gol_event?, avg_turns.get(completed_turns)),
                        Ok(Event::ImageOutputComplete { .. }) =>
                            log::info!(target: "Test", "{}", gol_event?),
                        Ok(Event::FinalTurnComplete { .. }) =>
                            log::info!(target: "Test", "{}", gol_event?),
                        Ok(Event::StateChange { new_state, .. }) => {
                            log::info!(target: "Test", "{}", gol_event?);
                            if let State::Quitting = new_state {
                                break 'sdl
                            }
                        },
                        Err(_) => break 'sdl,
                    }
                },
            }
        }
        Ok(())
    }

    pub async fn run_headless(
        events: Receiver<Event>,
        key_presses: Receiver<Keycode>,
        events_forward: Sender<Event>,
        key_presses_forward: Sender<Keycode>,
    ) -> Result<()> {
        let mut avg_turns = AvgTurns::new();
        'sdl: loop {
            select! {
                key_presses = key_presses.recv_async() => {
                    if let Ok(key) = key_presses {
                        key_presses_forward.send_async(key).await?;
                    }
                },
                gol_event = events.recv_async() => {
                    if let Ok(e) = &gol_event {
                        events_forward.send_async(e.clone()).await?;
                    }
                    match gol_event {
                        Ok(Event::AliveCellsCount { completed_turns, .. }) =>
                            log::info!(target: "Test", "{} Avg{:>5} turns/s", gol_event?, avg_turns.get(completed_turns)),
                        Ok(Event::ImageOutputComplete { .. }) =>
                            log::info!(target: "Test", "{}", gol_event?),
                        Ok(Event::FinalTurnComplete { .. }) =>
                            log::info!(target: "Test", "{}", gol_event?),
                        Ok(Event::StateChange { new_state, .. }) => {
                            log::info!(target: "Test", "{}", gol_event?);
                            if let State::Quitting = new_state {
                                break 'sdl
                            }
                        },
                        Err(_) => break 'sdl,
                        _ => (),
                    }
                },
            }
        }
        Ok(())
    }
}

#[allow(dead_code)]
pub mod common {
    use std::{time::Duration, fmt::Display};
    use tokio::task::JoinHandle;

    pub fn deadline<T>(ddl: Duration, msg: T) -> JoinHandle<()>
    where
        T: AsRef<str> + Display + Send + 'static
    {
        tokio::spawn(async move {
            tokio::time::sleep(ddl).await;
            panic!("{}", msg);
        })
    }
}
