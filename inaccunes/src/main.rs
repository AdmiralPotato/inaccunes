use log::*;
use sdl2::{pixels::PixelFormatEnum, render::TextureAccess};

mod cartridge;
use cartridge::Cartridge;
mod system;
use system::System;

const NES_WIDTH: usize = 256;
const NES_HEIGHT: usize = 240;
const NES_PITCH: usize = std::mem::size_of::<u32>() * NES_WIDTH;
const NES_PIXEL_COUNT: usize = NES_WIDTH * NES_HEIGHT;

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

    let sdl = sdl2::init().expect("Unable to initialize SDL (like, at all)");
    let video = sdl.video().expect("Unable to initialize SDL video");
    let mut event_pump = sdl.event_pump().expect("Couldn't get an event pump?!");
    let window = video
        .window("inaccunes", 512, 480)
        .resizable()
        .allow_highdpi() // thanks apple you started the lie that caused the resolution war
        .build()
        .expect("Couldn't make an SDL window?!!");
    let mut canvas = window.into_canvas().build().unwrap();

    canvas.set_draw_color(sdl2::pixels::Color::RGB(0, 255, 255));
    canvas.clear();
    canvas.present();
    let texture_creator = canvas.texture_creator();
    let mut texture = texture_creator
        .create_texture(
            PixelFormatEnum::ARGB8888,
            TextureAccess::Streaming,
            NES_WIDTH as u32,
            NES_HEIGHT as u32,
        )
        .expect("Could not create a native size texture.");

    'running: loop {
        let pixels = system.render();
        // transmute is *unsafe*, in that the compiler can't help us if we make
        // a mistake. Unsafe justification: we are passing the u32s to the
        // graphics API, and it's just using &[u8] because it wants a bunch of
        // bytes, not because it *needs* it to *actually be* an array of
        // individual, meaningful byte values.
        let pixels_as_u8: &[u8] = unsafe { std::mem::transmute(&pixels[..]) };
        texture
            .update(None, pixels_as_u8, NES_PITCH)
            .expect("Could not update the native texture with raw pixel data");
        canvas
            .copy(&texture, None, None)
            .expect("could not copy native texture to window texture");
        canvas.present();
        for event in event_pump.poll_iter() {
            use sdl2::{event::Event, keyboard::Keycode};
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                _ => {}
            }
        }
    }
}
