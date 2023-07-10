use super::*;
use sdl2::pixels::Color;

const OVERALL_BACKGROUND: Color = Color {
    r: 0,
    g: 0,
    b: 0,
    a: 0,
};

const LEFT_MARGIN: i32 = 3;
const TOP_MARGIN: i32 = 1;
pub struct DebugDevicesWindow {
    window: DebugWindow,
}

impl DebugDevicesWindow {
    pub fn new(video: &VideoSubsystem, font: Arc<FontData>) -> Box<Self> {
        let window = DebugWindow::new("Devices Window", 512, 384, video, font);
        Box::new(Self { window })
    }
}

impl DebugWindowThing for DebugDevicesWindow {
    fn draw(&mut self, system: &System) {
        let DebugWindow { canvas, font, .. } = &mut self.window;
        let controllers = system.get_controllers();
        canvas.set_draw_color(OVERALL_BACKGROUND);
        canvas.clear();
        font.render_to_canvas(canvas, LEFT_MARGIN, TOP_MARGIN, &system.show_cpu_state());
        font.render_to_canvas(
            canvas,
            LEFT_MARGIN,
            TOP_MARGIN + font.get_glyph_height() as i32,
            &format!("Controllers: {:?}", controllers),
        );
        canvas.present();
    }
}
