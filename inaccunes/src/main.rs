use log::*;
mod cartridge;
use cartridge::Cartridge;
fn main() {
    env_logger::init();
    let our_arguments: Vec<String> = std::env::args().collect();
    println!("our_arguments: {:?}", our_arguments);
    if our_arguments.len() != 2 {
        error!("Wrong nubmer of arguments. Please provide only the file path to ROM file.");
        error!("Usage: inaccunes path/to/game.nes");
        return
    }
    let cartridge = Cartridge::new(&our_arguments[1]);
}
