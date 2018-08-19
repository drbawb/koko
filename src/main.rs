#[macro_use] extern crate glium;

pub mod engine;
pub mod graphics;
pub mod input;
pub mod units;
pub mod util;

use engine::Engine;
use glium::glutin;

fn main() {
    println!("koko is starting up...");
    let context    = glutin::ContextBuilder::new();
    let mut events = glutin::EventsLoop::new();
    let window     = glutin::WindowBuilder::new()
        .with_title(String::from("koko gl"))
        .with_dimensions(glutin::dpi::LogicalSize::new(1280.0, 720.0));

    let display = glium::Display::new(window, context, &events)
        .expect("could not initialize display ...");


    println!("let me tell you a story...");
    let mut engine = Engine::new(display);
    engine.run(&mut events);
    println!("❤"); // TODO: emoji heart because I can?!
}
