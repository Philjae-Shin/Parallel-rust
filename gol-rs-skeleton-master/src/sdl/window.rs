use anyhow::{anyhow, Result, Context};
use sdl2::EventPump;
use sdl2::pixels::{PixelFormatEnum, Color};
use sdl2::render::{Texture, Canvas};
use sdl2::video::Window as SdlWindow;

pub struct Window {
    width: u32,
    height: u32,
    pitch: u32,
    canvas: Canvas<SdlWindow>,
    texture: Option<Texture>,
    pump: Option<EventPump>,
    pixels: Vec<u8>,
}

#[allow(dead_code)]
impl Window {
    pub fn new<T: AsRef<str>>(
        title: T,
        width: u32,
        height: u32
    ) -> Result<Self> {
        let context = sdl2::init().map_err(|e| anyhow!(e))?;
        let video = context.video().map_err(|e| anyhow!(e))?;
        let window = video
            .window(title.as_ref(), width, height)
            .position_centered()
            .opengl()
            .allow_highdpi()
            .build()?;
        let pump = context.event_pump().map_err(|e| anyhow!(e))?;
        let canvas = window.into_canvas().build()?;
        let texture = canvas.texture_creator().create_texture_streaming(
            PixelFormatEnum::ARGB8888,
            width,
            height,
        )?;
        let pitch = width * 4 * std::mem::size_of::<u8>() as u32;
        let pixels = vec![0_u8; (height * pitch) as usize];

        Ok(Window {
            width,
            height,
            pitch,
            canvas,
            texture: Some(texture),
            pump: Some(pump),
            pixels,
        })
    }

    pub fn take_event_pump(&mut self) -> Result<EventPump> {
        self.pump.take().context("Cannot take event pump twice!")
    }

    pub fn render_frame(&mut self) -> Result<()> {
        let texture = self.texture.as_mut().context("Missing texture")?;
        texture.update(None, &self.pixels, self.pitch as usize)?;
        self.canvas.clear();
        self.canvas.copy(&texture, None, None).map_err(|e| anyhow!(e))?;
        self.canvas.present();
        Ok(())
    }

    pub fn set_pixel(&mut self, x: u32, y: u32, color: Color) {
        self.pixels[4 * (y * self.width + x) as usize + 0] = color.a;
        self.pixels[4 * (y * self.width + x) as usize + 1] = color.r;
        self.pixels[4 * (y * self.width + x) as usize + 2] = color.g;
        self.pixels[4 * (y * self.width + x) as usize + 3] = color.b;
    }

    pub fn flip_pixel(&mut self, x: u32, y: u32) {
        assert!(
            x < self.width && y < self.height,
            "Cell flipped at ({}, {}) is outside the bounds of the window.",
            x, y
        );
        self.pixels[4 * (y * self.width + x) as usize + 0] =
            !self.pixels[4 * (y * self.width + x) as usize + 0];
        self.pixels[4 * (y * self.width + x) as usize + 1] =
            !self.pixels[4 * (y * self.width + x) as usize + 1];
        self.pixels[4 * (y * self.width + x) as usize + 2] =
            !self.pixels[4 * (y * self.width + x) as usize + 2];
        self.pixels[4 * (y * self.width + x) as usize + 3] =
            !self.pixels[4 * (y * self.width + x) as usize + 3];
    }

    pub fn count_pixels(&self) -> u32 {
        self.pixels
            .chunks(4)
            .filter(|&chunk|
                chunk.get(0).map_or(false, |&x| x == 0xFF_u8))
            .count() as u32
    }

}

impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            self.texture.take().unwrap().destroy();
        }
    }
}
