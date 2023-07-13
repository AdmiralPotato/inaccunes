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
        let devices = system.get_devices();
        let DebugWindow { canvas, font, .. } = &mut self.window;
        let controllers = system.get_controllers();
        canvas.set_draw_color(OVERALL_BACKGROUND);
        canvas.clear();
        let y = 0;
        font.render_to_canvas(
            canvas,
            LEFT_MARGIN,
            TOP_MARGIN + y * font.get_glyph_height() as i32,
            &system.show_cpu_state(),
        );
        let y = y + 1;
        font.render_to_canvas(
            canvas,
            LEFT_MARGIN,
            TOP_MARGIN + y * font.get_glyph_height() as i32,
            &format!("Controllers: {:?}", controllers),
        );
        let y = y + 2;
        let data = devices.get_ppu()[0];
        font.render_to_canvas(
            canvas,
            LEFT_MARGIN,
            TOP_MARGIN + y * font.get_glyph_height() as i32,
            &format!(
                "PPUCTRL = ${data:02X}\t\tNMI {nmi}\t|\tPPU {master}\n\
                \tSprite patterns ${spritepat}xxx\t|\tSprite Size: {sprites}\n\
                \tBG patterns ${bgpat}xxx\t|\tVRAM addr+={vraminc}\t|\tnames $2{nametable:X}xx",
                nmi = if (data & 0x80) == 0 { "off" } else { "ON" },
                master = if (data & 0x40) == 0 {
                    "master"
                } else {
                    "slave"
                },
                sprites = if (data & 0x20) == 0 { "8x8" } else { "8x16" },
                bgpat = if (data & 0x10) == 0 { "0" } else { "1" },
                spritepat = if (data & 0x8) == 0 { "0" } else { "1" },
                vraminc = if (data & 0x4) == 0 { "1(X)" } else { "32(Y)" },
                nametable = (data & 3) << 2,
            ),
        );
        let y = y + 4;
        let data = devices.get_ppu()[1];
        font.render_to_canvas(
            canvas,
            LEFT_MARGIN,
            TOP_MARGIN + y * font.get_glyph_height() as i32,
            &format!(
                "PPUMASK = ${data:02X}\t\tEmphasis: {emphasis}\tShow: {show}\tClip: {clip}\t{color}
                ",
                emphasis = match data >> 5 {
                    0b000 => "---",
                    0b001 => "R--",
                    0b010 => "-G-",
                    0b100 => "--B",
                    0b011 => "RG-",
                    0b110 => "-GB",
                    0b101 => "R-B",
                    0b111 => "RGB",
                    _ => unreachable!(),
                },
                show = match (data >> 3) & 0b11 {
                    0b00 => "--,--",
                    0b01 => "--,BG",
                    0b10 => "SP,--",
                    0b11 => "SP,BG",
                    _ => unreachable!(),
                },
                clip = match (data >> 1) & 0b11 {
                    0b00 => "--,--",
                    0b01 => "--,BG",
                    0b10 => "SP,--",
                    0b11 => "SP,BG",
                    _ => unreachable!(),
                },
                color = if (data & 0b1) == 0 {
                    "color"
                } else {
                    "greyscale"
                }
            ),
        );
        let y = y + 2;
        let data = devices.get_ppu()[3];
        font.render_to_canvas(
            canvas,
            LEFT_MARGIN,
            TOP_MARGIN + y * font.get_glyph_height() as i32,
            &format!(
                "OAM ADDRESS = ${data:02X}",
            ),
        );
        let y = y + 2;
        canvas.present();
    }
}
