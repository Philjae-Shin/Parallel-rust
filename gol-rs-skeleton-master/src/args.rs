use clap::{ArgAction, Parser};

#[derive(Clone, Debug, Parser)]
#[clap(disable_help_flag = true)]
pub struct Args {
    #[arg(
        short = 't',
        long,
        default_value_t = 8,
        help = "Specify the number of worker threads to use."
    )]
    pub threads: usize,

    #[arg(
        short = 'w',
        long = "width",
        default_value_t = 512,
        help = "Specify the width of the image."
    )]
    pub image_width: usize,

    #[arg(
        short = 'h',
        long = "height",
        default_value_t = 512,
        help = "Specify the height of the image."
    )]
    pub image_height: usize,

    #[arg(
        short = 'f',
        long,
        default_value_t = 60,
        help = "Specify the FPS of the SDL window."
    )]
    pub fps: usize,

    #[arg(
        long,
        default_value_t = 10000000,
        help = "Specify the number of turns to process."
    )]
    pub turns: usize,

    #[arg(
        long,
        default_value_t = false,
        help = "Disable the SDL window for running in a headless environment."
    )]
    pub headless: bool,

    #[arg(
        long,
        action = ArgAction::HelpLong
    )]
    help: Option<bool>,
}

impl Default for Args {
    fn default() -> Self {
        Args::parse_from([""])
    }
}

impl Args {
    pub fn threads(mut self, threads: usize) -> Self {
        self.threads = threads;
        self
    }

    pub fn image_width(mut self, image_width: usize) -> Self {
        self.image_width = image_width;
        self
    }

    pub fn image_height(mut self, image_height: usize) -> Self {
        self.image_height = image_height;
        self
    }

    pub fn fps(mut self, fps: usize) -> Self {
        self.fps = fps;
        self
    }

    pub fn turns(mut self, turns: usize) -> Self {
        self.turns = turns;
        self
    }

    pub fn headless(mut self, headless: bool) -> Self {
        self.headless = headless;
        self
    }
}
