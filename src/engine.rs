use std::thread;
use std::time::{Duration, Instant};

use sdl2;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;

use sdl2::pixels::Color;
use sdl2::rect::Rect; // TODO: abstract my own drawtypes?
use sdl2::render::Texture;

use graphics::Display;
use input::Input;

pub static COLOR_BG: Color  = Color::RGB(0,0,0);
pub static COLOR_FPS: Color = Color::RGB(255,255,0);
pub static COLOR_PEN: Color = Color::RGB(125, 0, 175);

#[derive(Copy, Clone, Debug)]
enum BrushMode {
    Normal,
    Squareish,
    WowSoEdgy,
}

pub struct Engine {
	context:     sdl2::Sdl,
	controller:  Input,
	display:     Display,

    brush:  BrushMode,
    color:  (u8,u8,u8),
    cursor: (i32,i32),
}

impl Engine {
    pub fn new(context: sdl2::Sdl) -> Engine {
        let video_renderer = Display::new(&context);

        Engine {
            context:    context,
            controller: Input::new(),
            display:    video_renderer,

            brush:  BrushMode::Normal,
            color:  (125,0,175),
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
        let mut regions = vec![];
        for i in 0..9 {
            regions.push(Some(self.display.get_texture(1280,720)));
        }

        let mut mouse_clicked = false;
        let mut last_point = None;

        // init bitmap to good state
        Engine::with_texture(&mut self.display, &mut regions[0], |io| { io.clear_buffer() });
            
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

            // erase canvas
            if self.controller.was_key_released(Keycode::E) {
                Engine::with_texture(&mut self.display, &mut regions[0], |io| { io.clear_buffer() });
            }

            // switch brush
            if self.controller.was_key_released(Keycode::B) {
                self.brush = match self.brush { // TODO: better cycle?
                    BrushMode::Normal    => BrushMode::Squareish,
                    BrushMode::Squareish => BrushMode::WowSoEdgy,
                    BrushMode::WowSoEdgy => BrushMode::Squareish,
                }
            }

            if self.controller.is_key_held(Keycode::I) {
                self.color.0 = self.color.0.wrapping_add(0x01);
            } else if self.controller.is_key_held(Keycode::O) {
                self.color.1 = self.color.1.wrapping_add(0x01);
            } else if self.controller.is_key_held(Keycode::P) {
                self.color.2 = self.color.2.wrapping_add(0x01);
            }

            let brush_color = Color::RGB(self.color.0, self.color.1, self.color.2);
            if mouse_clicked {
                let (x2,y2) = self.cursor;

                if let Some((x1,y1)) = last_point {

                    let brush = self.brush;
                    Engine::with_texture(&mut self.display, &mut regions[0], |display| {
                        match brush {
                            BrushMode::WowSoEdgy => for i in 0..10 {
                                display.draw_line(x1+i, y1, x2, y2+i, brush_color);
                            },

                            BrushMode::Squareish => for i in 0..5 {
                                display.draw_line(x1+i, y1, x2+i, y2, brush_color);
                            },

                            BrushMode::Normal => {},
                        }
                    });

                    // let diffx = x2 - x1;
                    // let diffy = y2 - y1;
                    // println!("delta ({},{}), mag: {}", diffx, diffy, diffy-diffx);
                } // NOTE: doesn't draw point if mouse held for single frame

                last_point = Some((x2,y2));
           } else { last_point = None; }


            // handle draw calls
			self.display.clear_buffer(); // clear back-buffer
            self.display.copy(regions[0].as_ref().unwrap());
            self.draw_cursor(brush_color);
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

    fn with_texture<F>(io: &mut Display, target: &mut Option<Texture>, mut mutator: F)
        where F: FnMut(&mut Display) -> () {


        let _last = io.retarget().set(target.take().unwrap());
        mutator(io);
        *target = io.retarget().reset()
                        .ok().expect("did not get target back");
    }

    fn draw_cursor(&mut self, brush_color: Color) {
        self.display.fill_rect(Rect::new(self.cursor.0, self.cursor.1, 10, 10), brush_color);
    }

    fn draw_debug(&mut self, time: Duration) {
        let mut time_ms = time.as_secs() * 1000;        // -> millis
        time_ms += time.subsec_nanos() as u64 / (1000 * 1000); // /> micros /> millis
      
        let (hue_r, hue_g, hue_b) = self.color;
        let buf = format!("{}ms, e = erase all, b = brush ({:?}), hue(i,o,p) => ({:x},{:x},{:x})", 
                          time_ms, 
                          self.brush,
                          hue_r, hue_g, hue_b);
        self.display.blit_text(&buf[..], COLOR_FPS);
    }
}
