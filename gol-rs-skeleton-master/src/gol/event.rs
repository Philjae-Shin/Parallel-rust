use crate::util::cell::CellCoord;
use std::fmt::Display;

/// State represents a change in the state of execution.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum State {
    #[default]
    Executing,
    Pause,
    Quitting,
}

impl Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// `Event` represents any Game of Life event that needs to be communicated to the user.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum Event {
    /// `AliveCellsCount` is an Event notifying the user about the number of currently alive cells.
    /// This Event should be sent every 2s.
    AliveCellsCount { completed_turns: u32, cells_count: u32 },

    /// `ImageOutputComplete` is an Event notifying the user about the completion of output.
    /// This Event should be sent every time an image has been saved.
    ImageOutputComplete { completed_turns: u32, filename: String },

    /// `StateChange` is an Event notifying the user about the change of state of execution.
    /// This Event should be sent every time the execution is paused, resumed or quit.
    StateChange { completed_turns: u32, new_state: State },

    /// `CellFlipped` is an Event notifying the GUI about a change of state of a single cell.
    /// This event should be sent every time a cell changes state.
    /// Make sure to send this event for all cells that are alive when the image is loaded in.
    CellFlipped { completed_turns: u32, cell: CellCoord },

    /// `CellsFlipped` is an Event notifying the GUI about a change of state of many cells.
    /// You can collect many flipped cells and send `CellsFlipped` at a time instead of sending `CellFlipped` for every flipped cell.
    /// You can send many times of `CellsFlipped` event in a turn, i.e., each worker could send `CellsFlipped`.
    /// **Please be careful not to send `CellFlipped` and `CellsFlipped` at the same time, as they may conflict.**
    /// Choose one of them.
    CellsFlipped { completed_turns: u32, cells: Vec<CellCoord> },

    /// `TurnComplete` is an Event notifying the GUI about turn completion.
    /// SDL will render a frame when this event is sent.
    /// All `CellFlipped` or `CellsFlipped` events must be sent *before* `TurnComplete`.
    TurnComplete { completed_turns: u32 },

    /// `FinalTurnComplete` is an Event notifying the testing framework about the new world state after execution finished.
    /// The data included with this Event is used directly by the tests.
    /// SDL closes the window when this Event is sent.
    FinalTurnComplete { completed_turns: u32, alive: Vec<CellCoord> },
}

impl Display for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Event::AliveCellsCount { completed_turns, cells_count  } =>
                write!(f, "Complete Turns {:<8} Alive Cells {:<8}", completed_turns, cells_count),
            Event::ImageOutputComplete { completed_turns, filename } =>
                write!(f, "Complete Turns {:<8} File {} Output Done", completed_turns, filename),
            Event::FinalTurnComplete { completed_turns, .. } =>
                write!(f, "Complete Turns {:<8} Final Turn Complete", completed_turns),
            Event::StateChange { completed_turns, new_state } =>
                write!(f, "Complete Turns {:<8} {}", completed_turns, new_state),
            _ => Ok(()),
        }
    }
}

impl Event {
    pub fn get_completed_turns(&self) -> u32 {
        match self {
            Event::AliveCellsCount { completed_turns, .. }
            | Event::ImageOutputComplete { completed_turns, .. }
            | Event::StateChange { completed_turns, .. }
            | Event::CellFlipped { completed_turns, .. }
            | Event::TurnComplete { completed_turns, .. }
            | Event::FinalTurnComplete { completed_turns, .. }
            | Event::CellsFlipped { completed_turns, .. } => *completed_turns,
        }
    }
}
