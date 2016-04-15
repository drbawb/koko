use std::thread;
use std::time::Duration;

use sdl2;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;

use sdl2::pixels::Color;
use sdl2::rect::Rect; // TODO: abstract my own drawtypes?

use graphics::Display;
use input::Input;

pub struct Engine {
	context:     sdl2::Sdl,
	controller:  Input,
	display:     Display,

    cursor: (i32,i32),
}

impl Engine {
    pub fn new(context: sdl2::Sdl) -> Engine {
        let video_renderer = Display::new(&context);

        Engine {
            context:    context,
            controller: Input::new(),
            display:    video_renderer,

            cursor: (0,0),
        }
    }

    pub fn run(&mut self) {
		let mut event_pump = self.context.event_pump().unwrap();
        let mut is_running = true;

        while is_running {
			// drain input event queue once per frame
			self.controller.begin_new_frame();
			for event in event_pump.poll_iter() {
				match event {
					Event::KeyDown { keycode, .. } => {
						self.controller.key_down_event(keycode.unwrap());
					},

					Event::KeyUp { keycode, .. } => {
						self.controller.key_up_event(keycode.unwrap());
					},

                    Event::MouseMotion { x, y, .. } => self.cursor = (x,y),

					_ => {},
				}
			}

            // handle exit game
			if self.controller.was_key_released(Keycode::Escape) { is_running = false; }

            // handle draw calls
			self.display.clear_buffer(); // clear back-buffer
            self.display.fill_rect(Rect::new(self.cursor.0, self.cursor.1, 10, 10), Color::RGB(128, 0, 175));
			self.display.switch_buffers();

            // TODO: at least *pretend* we actually care about hitting 60FPS
            thread::sleep(Duration::from_millis(1000 / 60));
        }
    }
}
