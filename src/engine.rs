use std::thread;
use std::time::{Duration, Instant};

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
        let target_fps_ms  = Duration::from_millis(1000 / 120); // TODO: const fn?

		let mut event_pump = self.context.event_pump().unwrap();
        let mut is_running = true;

        let mut frame_start_at;
        let mut elapsed_time = Duration::from_millis(0);

        // drawing state
        let mut bitmap = self.display.get_texture(1280,720);
        let mut mouse_clicked = false;
        
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

            let (x,y) = self.cursor;
            if mouse_clicked {
                // origin x,y
                bitmap.with_lock(None, |buf,pitch| {
                    for ofs_x in 0..5 {
                        for ofs_y in 0..5 {
                            let x = (x + ofs_x) as usize;
                            let y = (y + ofs_y) as usize;

                            let row = y * 1280 * 4 as usize; // 1280 * 4bpp
                            let col = x * 4;              // 4bpp
                            buf[row + col + 0] = 175;
                            buf[row + col + 1] = 0;
                            buf[row + col + 2] = 125;
                            buf[row + col + 3] = 0;
                        }
                    }
                });
            }

            // handle draw calls
			self.display.clear_buffer(); // clear back-buffer
            self.display.copy(&bitmap);
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
        self.display.fill_rect(Rect::new(self.cursor.0, self.cursor.1, 10, 10), Color::RGB(128, 0, 175));
    }

    fn draw_debug(&mut self, elapsed_time: Duration) {
        self.display.blit_fps(elapsed_time);
    }
}
