use super::*;
use sdl2::{pixels::Color, rect::Rect};

const OVERALL_BACKGROUND: Color = Color {
    r: 0,
    g: 0,
    b: 0,
    a: 0,
};
const EVEN_BACKGROUND: Color = Color {
    r: 0,
    g: 64,
    b: 64,
    a: 0,
};
const ODD_BACKGROUND: Color = Color {
    r: 64,
    g: 0,
    b: 64,
    a: 0,
};
const STACK_EVEN_BACKGROUND: Color = Color {
    r: 64,
    g: 64,
    b: 0,
    a: 0,
};
const STACK_ODD_BACKGROUND: Color = Color {
    r: 64,
    g: 41,
    b: 0,
    a: 0,
};

const LEFT_MARGIN: i32 = 3;
const TOP_MARGIN: i32 = 1;

pub struct DebugMemoryWindow {
    window: DebugWindow,
}

impl DebugMemoryWindow {
    pub fn new(video: &VideoSubsystem, font: Arc<FontData>) -> Box<Self> {
        let window = DebugWindow::new(
            "Work RAM Window",
            VISIBLE_MEMORY_COLUMNS * (font.get_glyph_width() + 1),
            VISIBLE_MEMORY_ROWS * (font.get_glyph_height() + 2),
            video,
            font,
        );
        Box::new(Self { window })
    }
}

impl DebugWindowThing for DebugMemoryWindow {
    fn draw(&mut self, system: &System) {
        let DebugWindow { canvas, font, .. } = &mut self.window;
        canvas.set_draw_color(OVERALL_BACKGROUND);
        canvas.clear();
        let cell_width = font.get_glyph_width() as i32 + 1;
        let cell_height = font.get_glyph_height() as i32 + 2;
        let left_margin = LEFT_MARGIN * cell_width;
        let top_margin = TOP_MARGIN * cell_height;
        for x in 0..4 {
            for (i, ch) in b"0123456789ABCDEF".iter().enumerate() {
                font.render_to_canvas(
                    canvas,
                    left_margin + (x * 16 as i32 + i as i32) * cell_width * 3,
                    1,
                    &format!(".{}", *ch as char),
                );
            }
        }
        for y in 0..NUM_MEMORY_ROWS {
            let target_address = y * BYTES_PER_MEMORY_ROW;
            if target_address >= 0x0100 && target_address <= 0x01FF {
                if y & 1 == 0 {
                    canvas.set_draw_color(STACK_EVEN_BACKGROUND);
                } else {
                    canvas.set_draw_color(STACK_ODD_BACKGROUND);
                }
            } else {
                if y & 1 == 0 {
                    canvas.set_draw_color(EVEN_BACKGROUND);
                } else {
                    canvas.set_draw_color(ODD_BACKGROUND);
                }
            }
            canvas
                .fill_rect(Rect::new(
                    left_margin - cell_width,
                    top_margin + y as i32 * cell_height,
                    BYTES_PER_MEMORY_ROW as u32 * cell_width as u32 * 3 + cell_width as u32,
                    cell_height as u32,
                ))
                .unwrap();
            font.render_to_canvas(
                canvas,
                0,
                top_margin + y as i32 * (cell_height) + 2,
                &format!("{:02X}", (target_address >> 4)),
            );
            for x in 0..BYTES_PER_MEMORY_ROW {
                let target_address = target_address + x;
                font.render_to_canvas(
                    canvas,
                    left_margin + (x as i32) * (cell_width) * 3,
                    top_margin + y as i32 * (cell_height) + 2,
                    &format!("{:02X}", system.get_work_memory_byte(target_address)),
                );
                if target_address == 0x74A {
                    // HACK!
                    font.render_to_canvas(
                        canvas,
                        left_margin + (x as i32) * (cell_width) * 3 + 1,
                        top_margin + y as i32 * (cell_height) + 2,
                        &format!("{:02X}", system.get_work_memory_byte(target_address)),
                    );
                }
            }
        }
        canvas.present();
    }
}
