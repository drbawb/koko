extern crate sdl2;
extern crate sdl2_ttf;

pub mod engine;
pub mod graphics;
pub mod input;
pub mod units;

use engine::Engine;

fn main() {
	println!("initalizing sdl ...");
	let sdl_context = sdl2::init().expect("could not init sdl2!?");

	println!("let me tell you a story ...");
    let mut engine = Engine::new(sdl_context);
    engine.run();

    println!("<3"); // TODO: emoji heart because I can?!
}
