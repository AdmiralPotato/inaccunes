use std::{
    io::Read,
    ops::{Deref, RangeInclusive},
    sync::Arc,
};

use anyhow::{anyhow, Context};
use log::*;
use sdl2::{
    pixels::Color,
    rect::Rect,
    render::{TextureCreator, WindowCanvas},
    video::WindowContext,
};

const TAB_WIDTH: i32 = 8;

/// The raw, plain-ole-data properties of a font.
pub struct FontData {
    glyph_width: u32,
    glyph_height: u32,
    first_glyph: u8,
    num_glyphs: u8,
    glyphs_per_row: u8,
    glyph_data: Vec<u8>,
    num_rows: u8,
}

impl FontData {
    /// Load a font with the given properties from the given PNG data.
    pub fn load_from_png<R: Read>(
        png_data_reader: R,
        glyph_width: u32,
        glyph_height: u32,
        first_glyph: u8,
        num_glyphs: u8,
        glyphs_per_row: u8,
    ) -> Result<FontData, anyhow::Error> {
        //-> anyhow::Result<FontData> {
        // integer divide with round up
        let num_rows = (num_glyphs as u32 + (glyphs_per_row - 1) as u32) / glyphs_per_row as u32;
        // some day, when #88581 is done, we can do this instead:
        // let num_rows = (num_glyphs as u32).div_ceil(glyphs_per_row as u32);
        let decoder = png::Decoder::new(png_data_reader);
        let mut reader = decoder
            .read_info()
            .context("Unable to read PNG info chunk")?;
        // exactly, literally, actually the same as:
        /*
        let mut reader = match decoder.read_info() {
            Ok(x) => x,
            Err(x) => return Err(x),
        }
        */
        // Read the next frame. An APNG might contain multiple frames.
        let mut glyph_data = vec![0; reader.output_buffer_size()];
        let info = reader
            .next_frame(&mut glyph_data)
            .context("Unable to read PNG frame data")?;
        // Sanity check the width and height.
        if glyph_width * glyphs_per_row as u32 != info.width {
            return Err(anyhow!("Input PNG had wrong width"));
        }
        if glyph_height * num_rows != info.height {
            return Err(anyhow!("Input PNG had wrong height"));
        }

        // Allocate the output buffer.
        debug!("What is info? {info:?}");
        // Grab the bytes of the image.
        glyph_data.truncate(info.buffer_size());
        // Inspect more details of the last read frame.
        return Ok(FontData {
            glyph_width,
            glyph_height,
            first_glyph,
            num_glyphs,
            glyphs_per_row,
            glyph_data,
            num_rows: num_rows as u8,
        });
    }
    pub fn get_valid_glyph_range(&self) -> RangeInclusive<u8> {
        self.first_glyph..=self.first_glyph + (self.num_glyphs - 1)
    }

    pub fn get_glyph_height(&self) -> u32 {
        self.glyph_height
    }

    pub fn get_glyph_width(&self) -> u32 {
        self.glyph_width
    }
}

/// An instance of a font, ready to render to a particular window.
///
/// This is required because SDL textures cannot (simply) be shared between
/// different windows.
pub struct FontInstance {
    font_data: Arc<FontData>,
    texture: sdl2::render::Texture,
}
impl FontInstance {
    pub fn new(
        font_data: Arc<FontData>,
        texture_creator: &TextureCreator<WindowContext>,
    ) -> FontInstance {
        let width: u32 = font_data.glyph_width as u32 * font_data.glyphs_per_row as u32;
        let height: u32 = font_data.glyph_height as u32 * font_data.num_rows as u32;
        let mut texture = texture_creator
            .create_texture_static(sdl2::pixels::PixelFormatEnum::ABGR8888, width, height)
            .expect("Could not create FontInstance texture");
        texture
            .update(None, &font_data.glyph_data, width as usize * 4)
            .expect("Failed to populate texture with font data");
        texture.set_blend_mode(sdl2::render::BlendMode::Blend);
        FontInstance { font_data, texture }
    }

    pub fn render_to_canvas(
        &self,
        canvas: &mut sdl2::render::WindowCanvas,
        x: i32,
        y: i32,
        text: &str,
    ) {
        let FontData {
            glyph_width,
            glyph_height,
            glyphs_per_row,
            .. // I don't care about the rest of the fields
        } = *self.font_data;
        let mut current_x = x;
        let mut current_y = y;
        for char in text.chars().into_iter() {
            match char {
                '\n' => {
                    current_x = x;
                    current_y += glyph_height as i32;
                }
                '\t' => {
                    let tab_width = glyph_width as i32 * TAB_WIDTH;
                    current_x += tab_width - (current_x - x) % tab_width;
                }
                ' ' => {
                    current_x += glyph_width as i32;
                }
                char => {
                    let char_index: u8 = char.try_into().expect("UNICODE! NONICODE!");
                    let glyph_index =
                        if !self.font_data.get_valid_glyph_range().contains(&char_index) {
                            b'?' - self.font_data.first_glyph
                        } else {
                            char_index - self.font_data.first_glyph
                        };
                    let glyph_x: i32 = ((glyph_index % glyphs_per_row) as i32) * glyph_width as i32;
                    let glyph_y: i32 =
                        ((glyph_index / glyphs_per_row) as i32) * glyph_height as i32;
                    let source_rect = Rect::new(glyph_x, glyph_y, glyph_width, glyph_height);
                    let dest_rect = Rect::new(current_x, current_y, glyph_width, glyph_height);
                    // canvas.set_draw_color(Color::RGB(127, 0, 0));
                    // canvas.fill_rect(dest_rect).expect("Could not fill rect");
                    // // canvas.set_draw_color(Color::RGB(255, 255, 255));
                    canvas
                        .copy(&self.texture, source_rect, dest_rect)
                        .expect("Could not render text to canvas");
                    current_x += glyph_width as i32;
                }
            }
        }
    }
}

impl Deref for FontInstance {
    type Target = FontData;
    fn deref(&self) -> &FontData {
        self.font_data.deref()
    }
}

pub fn load_monaco() -> anyhow::Result<FontData> {
    FontData::load_from_png(&include_bytes!("monaco.png")[..], 6, 12, b' ', 96, 32)
}
