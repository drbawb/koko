use std::thread;
use std::time::{Duration, Instant};

use sdl2;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;

use sdl2::pixels::Color;
use sdl2::rect::Rect; // TODO: abstract my own drawtypes?

use graphics::Display;
use input::Input;

pub static COLOR_BG: Color  = Color::RGB(0,0,0);
pub static COLOR_FPS: Color = Color::RGB(255,255,0);
pub static COLOR_PEN: Color = Color::RGB(125, 0, 175);

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
        let target_fps_ms  = Duration::from_millis(1000 / 120); // TODO: const fn?

		let mut event_pump = self.context.event_pump().unwrap();
        let mut is_running = true;

        let mut frame_start_at;
        let mut elapsed_time = Duration::from_millis(0);

        // drawing state
        let mut bitmap = Some(self.display.get_texture(1280,720));
        let mut mouse_clicked = false;
        let mut last_point = None;

        // init bitmap to good state
        let mut target = 
            self.display.retarget().set(bitmap.take().unwrap());

        self.display.clear_buffer();

        bitmap = self.display.retarget().reset()
            .ok().expect("did not get target back");

        while is_running {
            frame_start_at  = Instant::now();

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

                    Event::MouseButtonDown { .. } => mouse_clicked = true,
                    Event::MouseButtonUp   { .. } => mouse_clicked = false,
                    
					_ => {},
				}
			}

            // handle exit game
			if self.controller.was_key_released(Keycode::Escape) { is_running = false; }

            if mouse_clicked {
                let (x2,y2) = self.cursor;

                if let Some((x1,y1)) = last_point {
                    let mut target = 
                        self.display.retarget().set(bitmap.take().unwrap());

                    for i in 0..10 {
                        self.display.draw_line(x1+i,y1, x2+i,y2, COLOR_PEN);
                    }

                    bitmap = self.display.retarget().reset()
                        .ok().expect("did not get target back");
                } 

                last_point = Some((x2,y2));
           } else { last_point = None; }


            // handle draw calls
			self.display.clear_buffer(); // clear back-buffer
            self.display.copy(bitmap.as_ref().unwrap());
            self.draw_cursor();
            self.draw_debug(elapsed_time);
			self.display.switch_buffers();

            // sleep for <target> - <draw time> and floor to zero
            elapsed_time = frame_start_at.elapsed();
            let sleep_time = if elapsed_time > target_fps_ms {
                Duration::from_millis(0)
            } else { target_fps_ms - elapsed_time };

            thread::sleep(sleep_time);
        }
    }

    fn draw_cursor(&mut self) {
        self.display.fill_rect(Rect::new(self.cursor.0, self.cursor.1, 10, 10), COLOR_PEN);
    }

    fn draw_debug(&mut self, elapsed_time: Duration) {
        self.display.blit_fps(elapsed_time, COLOR_FPS);
    }
}
