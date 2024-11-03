use std::time::{Duration, Instant};

const BUF_SIZE: usize = 3;

pub struct AvgTurns {
    count: usize,
    last_completed_turns: u32,
    last_called: Instant,
    buf_turns: [u32; BUF_SIZE],
    buf_durations: [Duration; BUF_SIZE],
}

impl AvgTurns {
    pub fn new() -> Self {
        AvgTurns {
            count: 0,
            last_completed_turns: 0,
            last_called: Instant::now(),
            buf_turns: [Default::default(); BUF_SIZE],
            buf_durations: [Default::default(); BUF_SIZE],
        }
    }

    pub fn get(&mut self, completed_turns: u32) -> u32 {
        self.buf_turns[self.count % BUF_SIZE] = completed_turns - self.last_completed_turns;
        self.buf_durations[self.count % BUF_SIZE] = self.last_called.elapsed();
        self.last_called = Instant::now();
        self.last_completed_turns = completed_turns;
        (self.count, _) = self.count.overflowing_add(1);
        let turns = self.buf_turns.iter().sum::<u32>();
        let duration = self.buf_durations.iter().sum::<Duration>().as_secs_f32().round() as u32;
        turns / duration.clamp(1, u32::MAX)
    }
}
