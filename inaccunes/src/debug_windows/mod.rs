use crate::*;
pub mod devices;
pub mod memory;
use sdl2::{render::WindowCanvas, VideoSubsystem};

struct DebugWindow {
    font: FontInstance,
    canvas: WindowCanvas,
}

impl DebugWindow {
    fn new(
        name: &str,
        width: u32,
        height: u32,
        video: &VideoSubsystem,
        font: Arc<FontData>,
    ) -> DebugWindow {
        let window = video
            .window(name, width, height)
            .build()
            .expect("Couldn't make an SDL window?!!");
        let mut canvas = window.into_canvas().build().unwrap();
        canvas.set_draw_color(sdl2::pixels::Color::RGB(0, 255, 255));
        canvas.clear();
        canvas.present();
        let font = FontInstance::new(font, &canvas.texture_creator());
        DebugWindow { font, canvas }
    }
}

pub trait DebugWindowThing {
    fn draw(&mut self, system: &System);
}
