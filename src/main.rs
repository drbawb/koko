#[macro_use] extern crate glium;

pub mod engine_gl;
pub mod graphics_gl;
pub mod input;
pub mod units;

use engine_gl::Engine;
use glium::DisplayBuild;

fn main() {
    println!("koko is starting up...");
    let display = glium::glutin::WindowBuilder::new()
        .with_title(String::from("koko gl"))
        .with_dimensions(1280, 720)
        .build_glium();

    let display = match display {
        Ok(gl_ctx) => gl_ctx,
        Err(msg) => {
            println!("glium init error: {}", msg);
            panic!("koko could not initialize the graphics subsystem");
        },
    };

    println!("let me tell you a story...");
    let mut engine = Engine::new(display);
    engine.run();
    
    println!("<3"); // TODO: emoji heart because I can?!
}
