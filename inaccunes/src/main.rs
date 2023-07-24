use std::sync::Arc;

use log::*;
use sdl2::{pixels::PixelFormatEnum, render::TextureAccess};

mod cartridge;
use cartridge::Cartridge;
mod system;
use system::System;
mod font;
use font::*;
mod debug_windows;
use debug_windows::*;

const WORK_RAM_SIZE: usize = 2048;
const NES_WIDTH: usize = 256;
const NES_HEIGHT: usize = 240;
const NES_PITCH: usize = std::mem::size_of::<u32>() * NES_WIDTH;
const NES_PIXEL_COUNT: usize = NES_WIDTH * NES_HEIGHT;
const BYTES_PER_MEMORY_ROW: u16 = 64;
const NUM_MEMORY_ROWS: u16 =
    (WORK_RAM_SIZE as u16 + (BYTES_PER_MEMORY_ROW - 1)) / BYTES_PER_MEMORY_ROW;
const VISIBLE_MEMORY_COLUMNS: u32 = 3 + (BYTES_PER_MEMORY_ROW as u32) * 3; // 64 columns plus a heading on the left
const VISIBLE_MEMORY_ROWS: u32 = 1 + 32; // 32 rows plus a header

fn main() {
    env_logger::init();
    let our_arguments: Vec<String> = std::env::args().collect();
    println!("our_arguments: {:?}", our_arguments);
    if our_arguments.len() != 2 {
        error!("Wrong nubmer of arguments. Please provide only the file path to ROM file.");
        error!("Usage: inaccunes path/to/game.nes");
        return;
    }
    let cartridge = Cartridge::new(&our_arguments[1]);
    let mut system = System::new(cartridge);

    let monaco =
        load_monaco().expect("Could not load Monaco, the best [bitmapped] monospace font evar");
    let monaco = Arc::new(monaco);

    let sdl = sdl2::init().expect("Unable to initialize SDL (like, at all)");
    let video = sdl.video().expect("Unable to initialize SDL video");
    let mut event_pump = sdl.event_pump().expect("Couldn't get an event pump?!");
    // TV window
    let tv_window = video
        .window("inaccunes", 512, 480)
        .resizable()
        .allow_highdpi() // thanks apple you started the lie that caused the resolution war
        .build()
        .expect("Couldn't make an SDL window?!!");
    let mut tv_canvas = tv_window.into_canvas().present_vsync().build().unwrap();
    tv_canvas.set_draw_color(sdl2::pixels::Color::RGB(0, 255, 255));
    tv_canvas.clear();
    tv_canvas.present();
    let tv_texture_creator = tv_canvas.texture_creator();
    let mut tv_texture = tv_texture_creator
        .create_texture(
            PixelFormatEnum::ARGB8888,
            TextureAccess::Streaming,
            NES_WIDTH as u32,
            NES_HEIGHT as u32,
        )
        .expect("Could not create a native size texture.");
    let monaco_for_tv = FontInstance::new(monaco.clone(), &tv_texture_creator);
    // Memory window
    let mut debug_windows: Vec<Box<dyn DebugWindowThing>> = vec![
        debug_windows::memory::DebugMemoryWindow::new(&video, monaco.clone()),
        debug_windows::devices::DebugDevicesWindow::new(&video, monaco.clone()),
    ];
    'running: loop {
        ///////////////////////////////////////////////////////////////////////
        // Draw the TV
        ///////////////////////////////////////////////////////////////////////
        let pixels = system.render();
        // transmute is *unsafe*, in that the compiler can't help us if we make
        // a mistake. Unsafe justification: we are passing the u32s to the
        // graphics API, and it's just using &[u8] because it wants a bunch of
        // bytes, not because it *needs* it to *actually be* an array of
        // individual, meaningful byte values.
        let pixels_as_u8: &[u8] = unsafe { std::mem::transmute(&pixels[..]) };
        tv_texture
            .update(None, pixels_as_u8, NES_PITCH)
            .expect("Could not update the native texture with raw pixel data");
        tv_canvas
            .copy(&tv_texture, None, None)
            .expect("could not copy native texture to window texture");
        // HACK
        for chunk in system.get_devices().get_ppu().oam.chunks_exact(4) {
            let (y, tile, attributes, x) = (chunk[0], chunk[1], chunk[2], chunk[3]);
            monaco_for_tv.render_to_canvas(
                &mut tv_canvas,
                x as i32 * 2,
                y as i32 * 2,
                &format!("{tile:02X}\n{attributes:02X}"),
            );
        }
        tv_canvas.present();
        ///////////////////////////////////////////////////////////////////////
        // Draw debug windows
        ///////////////////////////////////////////////////////////////////////
        for debug_window in debug_windows.iter_mut() {
            debug_window.draw(&system);
        }
        ///////////////////////////////////////////////////////////////////////
        // All done drawing, do user input
        ///////////////////////////////////////////////////////////////////////
        for event in event_pump.poll_iter() {
            use sdl2::{event::Event, keyboard::Keycode};
            match event {
                Event::Quit { .. } => break 'running,
                Event::KeyDown {
                    keycode: Some(keycode),
                    ..
                } => match keycode {
                    Keycode::Escape => break 'running,
                    Keycode::Up => system.get_controllers_mut()[0].button_up = true,
                    Keycode::Down => system.get_controllers_mut()[0].button_down = true,
                    Keycode::Left => system.get_controllers_mut()[0].button_left = true,
                    Keycode::Right => system.get_controllers_mut()[0].button_right = true,
                    Keycode::Space => system.get_controllers_mut()[0].button_a = true,
                    Keycode::LShift => system.get_controllers_mut()[0].button_b = true,
                    Keycode::Return => system.get_controllers_mut()[0].button_start = true,
                    Keycode::Tab => system.get_controllers_mut()[0].button_select = true,
                    _ => info!("Key I don't care about: {keycode}"),
                },
                Event::KeyUp {
                    keycode: Some(keycode),
                    ..
                } => match keycode {
                    Keycode::Up => system.get_controllers_mut()[0].button_up = false,
                    Keycode::Down => system.get_controllers_mut()[0].button_down = false,
                    Keycode::Left => system.get_controllers_mut()[0].button_left = false,
                    Keycode::Right => system.get_controllers_mut()[0].button_right = false,
                    Keycode::Space => system.get_controllers_mut()[0].button_a = false,
                    Keycode::LShift => system.get_controllers_mut()[0].button_b = false,
                    Keycode::Return => system.get_controllers_mut()[0].button_start = false,
                    Keycode::Tab => system.get_controllers_mut()[0].button_select = false,
                    _ => (),
                },
                _ => {}
            }
        }
    }
}
