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
        let ppu = devices.get_ppu();
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
        let data = ppu.register_control;
        font.render_to_canvas(
            canvas,
            LEFT_MARGIN,
            TOP_MARGIN + y * font.get_glyph_height() as i32,
            &format!(
                "PPUCTRL = ${data:02X}\t\tNMI {nmi}\t|\tPPU {master}\n\
                \tSprite patterns ${spritepat}xxx\t|\tSprite Size: {sprites}\n\
                \tBG patterns ${bgpat}xxx\t|\tVRAM addr+={vraminc}\t|\tnames $2{nametable:X}xx",
                nmi = if ppu.is_nmi_on() { "ON" } else { "off" },
                master = if ppu.is_master() { "master" } else { "slave" },
                sprites = if ppu.is_sprite_size_8x16() {
                    "8x16"
                } else {
                    "8x8"
                },
                bgpat = if ppu.are_bg_tiles_in_upper_half() {
                    "1"
                } else {
                    "0"
                },
                spritepat = if ppu.are_sprite_tiles_in_upper_half() {
                    "1"
                } else {
                    "0"
                },
                vraminc = if ppu.is_vram_incrementing_by_y() {
                    "32(Y)"
                } else {
                    "1(X)"
                },
                nametable = ppu.which_nametable_is_upper_left() << 2,
            ),
        );
        let y = y + 4;
        let data = ppu.register_mask;
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
        font.render_to_canvas(
            canvas,
            LEFT_MARGIN,
            TOP_MARGIN + y * font.get_glyph_height() as i32,
            &format!(
                "OAM ADDRESS = ${oam:02X}\t\tPPU ADDRESS = ${ppudata:04X}",
                oam = ppu.register_oam_address,
                ppudata = ppu.register_ppudata_address,
            ),
        );
        let y = y + 2;

        let shift_x = ppu.register_control & 1;
        let shift_y = (ppu.register_control & 2) >> 1;
        font.render_to_canvas(
            canvas,
            LEFT_MARGIN,
            TOP_MARGIN + y * font.get_glyph_height() as i32,
            &format!(
                "x = ${x:04X}/{x_extra}\t\ty = ${y:04X}/{y_extra}",
                x = ppu.register_scroll_x,
                y = ppu.register_scroll_y,
                x_extra = ppu.register_scroll_x as u16 + (256 * shift_x as u16),
                y_extra = ppu.register_scroll_y as u16 + (240 * shift_y as u16),
            ),
        );
        let y = y + 2;
        canvas.present();
    }
}
