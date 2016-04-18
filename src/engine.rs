use std::mem;
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
use units::V2;

pub static COLOR_BG:  Color = Color::RGB(0,0,0);
pub static COLOR_FPS: Color = Color::RGB(255,255,0);
pub static COLOR_PEN: Color = Color::RGB(125, 0, 175);

#[derive(Copy, Clone, Debug)]
enum BrushMode {
    Normal,
    Squareish,
    WowSoEdgy,
    Eraser,
}

struct Region {
    pub is_init:  bool,
    pub is_hot:   bool,
    pub texture:  Option<Texture>,
    pub rle_data: Vec<(u8,usize)>,
}

impl Region {
    pub fn new() -> Region {
        Region {
            is_init:  false,
            is_hot:   false,
            texture:  None,
            rle_data: vec![],
        }
    }

    pub fn swap_in(&mut self, io: &mut Display, pxbuf: &mut Vec<u8>) {
        assert!(self.is_init && !self.is_hot);
        let mut txbuf = io.get_texture(1280,720);

        unsafe { pxbuf.set_len(0); }
        for cbyte in self.rle_data.iter() {
            let (byte, count) = *cbyte;
            for _i in 0..count { pxbuf.push(byte) }
        }
       
        println!("unpack len: {}", pxbuf.len());

        txbuf.update(None, &pxbuf[..], 1280 * 4)
            .ok().expect("could not update texture");

        self.texture = Some(txbuf);
        self.is_hot  = true;
    }

    pub fn swap_out(&mut self, io: &mut Display) {
        if !self.is_init || !self.is_hot { return; }

        // grab vram into byte buffer
        let mut buf = None;
        Engine::with_texture(io, &mut self.texture, |io| {
            buf = Some(io.read_pixels());
        });

        // allocate rle buffer
        let buf = buf.take().unwrap(); // TODO: don't panic if we didn't get anything.
        let mut rle = vec![];
        assert!(buf.len() > 0);

        // perform quick RLE
        let mut run = (buf[0], 0);
        for &byte in buf.iter() {
            if byte == run.0 { run.1 += 1; }
            else { rle.push(run); run = (byte, 1); }
        }
        rle.push(run); // push the last one...

        println!("SWAP OUT: buf len: {}, rle len: {}", buf.len(), rle.len());
        self.texture  = None; // drop texture on the floor
        self.rle_data = rle;
        self.is_hot   = false;
    }
}

pub struct Engine {
    context:     sdl2::Sdl,
    controller:  Input,
    display:     Display,

    brush:   BrushMode,
    color:   (u8,u8,u8),
    cursor:  (i32,i32),
    scanbox: V2,
    swapbuf: Vec<u8>,
}

impl Engine {
    pub fn new(context: sdl2::Sdl) -> Engine {
        let video_renderer = Display::new(&context);

        Engine {
            context:    context,
            controller: Input::new(),
            display:    video_renderer,

            brush:   BrushMode::Squareish,
            color:   (125,0,175),
            cursor:  (0,0),
            scanbox: V2(0,0),
            swapbuf: vec![0; 1280 * 720 * 4],
        }
    }

    pub fn run(&mut self) {
        let target_fps_ms  = Duration::from_millis(1000 / 120); // TODO: const fn?

        let mut event_pump = self.context.event_pump().unwrap();
        let mut is_running = true;

        let mut frame_start_at;
        let mut elapsed_time = Duration::from_millis(0);

        // init 9x9
        let mut regions = vec![];
        for _ in 0..9 {
            regions.push(Region::new());
        }

        let mut mouse_clicked = false;
        let mut last_point = None;

        // init bitmap to good state
        //
            
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
                for region in regions.iter_mut() {
                    Engine::with_texture(&mut self.display, &mut region.texture, |io| {
                        io.clear_buffer();
                        io.fill_rect(Rect::new(0,0,1280,5),   Color::RGB(255,0,0));  // top
                        io.fill_rect(Rect::new(0,715,1280,5), Color::RGB(255,0,0));  // bottom
                        io.fill_rect(Rect::new(0,0,5,720),    Color::RGB(255,0,0) ); // left
                        io.fill_rect(Rect::new(1275,0,5,720), Color::RGB(255,0,0));  // right
                    });
                }
            }

            // switch brush
            if self.controller.was_key_released(Keycode::B) {
                self.brush = match self.brush { // TODO: better cycle?
                    BrushMode::Normal    => BrushMode::Squareish,
                    BrushMode::Squareish => BrushMode::WowSoEdgy,
                    BrushMode::WowSoEdgy => BrushMode::Eraser,
                    BrushMode::Eraser    => BrushMode::Normal,
                }
            }

            if self.controller.is_key_held(Keycode::I) {
                self.color.0 = self.color.0.wrapping_add(0x01);
            } else if self.controller.is_key_held(Keycode::O) {
                self.color.1 = self.color.1.wrapping_add(0x01);
            } else if self.controller.is_key_held(Keycode::P) {
                self.color.2 = self.color.2.wrapping_add(0x01);
            }

            if self.controller.is_key_held(Keycode::Up) {
                self.scanbox = self.scanbox - V2(0, 5);
            } else if self.controller.is_key_held(Keycode::Down) {
                self.scanbox = self.scanbox + V2(0, 5);
            } else if self.controller.is_key_held(Keycode::Left) {
                self.scanbox = self.scanbox - V2(5, 0);
            } else if self.controller.is_key_held(Keycode::Right) {
                self.scanbox = self.scanbox + V2(5, 0);
            }

            // blit current brush to appropriate region
            let brush_color = Color::RGB(self.color.0, self.color.1, self.color.2);
            if mouse_clicked {
                let (x2,y2) = self.cursor;

                if let Some((x1,y1)) = last_point {
                    let brush = self.brush;
                    let V2(ofs_x, ofs_y) = self.scanbox;

                    // cursor + scanbox origin => adjusted coordinate
                    let x1 = x1 + ofs_x as i32;
                    let x2 = x2 + ofs_x as i32;
                    let y1 = y1 + ofs_y as i32;
                    let y2 = y2 + ofs_y as i32;

                    // compute ridx of adjusted coordinate
                    let pitch = (regions.len() as f64).sqrt() as i32;
                    let col  = x1 / 1280;
                    let row  = y1 / 720;
                    let ridx = (row * pitch) + col; // row * 3rows/col + col

                    // blit in that region instead
                    println!("[{}], row: {}, col: {}", ridx, row, col);
                    println!("real ({},{})=>({},{})", x1, y1, x2, y2);
                    let (x1,x2) = if x1 > 1280 {
                        let x1 = x1 % (1280 * col+1);
                        let x2 = x2 % (1280 * col+1);
                        (x1,x2)
                    } else { (x1,x2) };

                    let (y1,y2) = if y1 > 720 {
                        let y1 = y1 % ( 720 * row+1);
                        let y2 = y2 % ( 720 * row+1);
                        (y1,y2)
                    } else { (y1,y2) };
                    println!("adj ({},{})=>({},{})", x1, y1, x2, y2);


                    if x1 + 5 < 1280 && x2 < 1280 && y1 < 720 && y2 + 10 < 720 {
                        Engine::with_texture(&mut self.display, &mut regions[ridx as usize].texture, |display| {
                            match brush {
                                BrushMode::WowSoEdgy => for i in 0..10 {
                                    display.draw_line(x1+i, y1, x2, y2+i, brush_color);
                                },

                                BrushMode::Squareish => for i in 0..5 {
                                    display.draw_line(x1+i, y1, x2+i, y2, brush_color);
                                },

                                BrushMode::Eraser => {
                                    display.fill_rect(Rect::new(x2,y2, 20, 20), Color::RGB(0,0,0));
                                },

                                BrushMode::Normal => {},
                            }
                        });
                    }

                    // let diffx = x2 - x1;
                    // let diffy = y2 - y1;
                    // println!("delta ({},{}), mag: {}", diffx, diffy, diffy-diffx);
                } // NOTE: doesn't draw point if mouse held for single frame

                last_point = Some((x2,y2));
           } else { last_point = None; }


            // handle draw calls
            self.display.clear_buffer(); // clear back-buffer
            self.draw_regions(&mut regions);
            self.draw_cursor(brush_color);

            let V2(rx, ry) = self.scanbox + V2(1280,720);
            let region_sqrt = (regions.len() as f64).sqrt();
            if (rx as f64 > region_sqrt * 1280.0) || (ry as f64 > region_sqrt *  720.0) {
                println!("need to regrow right!");
                let pitch = (region_sqrt + 1.0) as usize;
                let next_square = pitch * pitch;
               
                // swap in the newly regrown buffer
                let mut buf = Vec::with_capacity(next_square);
                mem::swap(&mut regions, &mut buf);
                let mut old_drain = buf.into_iter();

                // copy old / allocate new
                for row in 0..pitch {
                    for col in 0..pitch {
                        if (row >= pitch - 1) || (col >= pitch - 1) {
                            regions.push(Region::new());
                        } else {
                            regions.push(old_drain.next().expect("ran out of regions to copy during regrow!"));
                        }
                    }
                }
            }

            // TODO: can we collapse this with the previous case?
            // if they pan left, trick them into thinking had canvases there
            if (self.scanbox.0 < 0) || (self.scanbox.1 < 0) {
                println!("need to regrow left!");
                let pitch = (region_sqrt + 1.0) as usize;
                let next_square = pitch * pitch;
               
                // swap in the newly regrown buffer
                let mut buf = Vec::with_capacity(next_square);
                mem::swap(&mut regions, &mut buf);
                let mut old_drain = buf.into_iter();

                // copy old / allocate new
                for row in 0..pitch {
                    for col in 0..pitch {
                        if (row == 0) || (col == 0) {
                            regions.push(Region::new());
                        } else {
                            regions.push(old_drain.next().expect("ran out of regions to copy during regrow!"));
                        }
                    }
                }

                self.scanbox = V2(1280,720) + self.scanbox;
            }

            let num_regions_drawn = regions.iter()
                .filter(|region| region.is_hot)
                .count();

            self.draw_debug(elapsed_time, regions.len(), num_regions_drawn);
            self.display.switch_buffers();

            // sleep for <target> - <draw time> and floor to zero
            elapsed_time = frame_start_at.elapsed();
            let sleep_time = if elapsed_time > target_fps_ms {
                Duration::from_millis(0)
            } else { target_fps_ms - elapsed_time };

            thread::sleep(sleep_time);
        }
    }

    // HACK: pulls texture out of an option and gives you a closure in which
    // you can use renderer calls to blit to it ...
    //
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

    fn draw_debug(&mut self, time: Duration, num_regions: usize, num_drawn: usize) {
        let mut time_ms = time.as_secs() * 1000;        // -> millis
        time_ms += time.subsec_nanos() as u64 / (1000 * 1000); // /> micros /> millis
      
        let (hue_r, hue_g, hue_b) = self.color;
        let buf = format!("{}ms, # regions: {}, # drawn: {}, sb @ {:?}, \
                          e = erase all, b = brush ({:?}), hue(i,o,p) => ({:x},{:x},{:x})", 
                          time_ms, 
                          num_regions, num_drawn,
                          self.scanbox,
                          self.brush,
                          hue_r, hue_g, hue_b);
        self.display.blit_text(&buf[..], COLOR_FPS);
    }

    // TODO: runs out of vram after ~1000 regions
    // 1280 * 720 * 4bpp = ~3.6MiB
    // ~3.6MiB * 1000 regions = ~3600MiB
    //
    // we should swap out regions that aren't inside the scanbox
    //
    fn draw_regions(&mut self, regions: &mut Vec<Region>) {
        let V2(ofs_x, ofs_y) = self.scanbox;
        let top0   = ofs_y;
        let bot0   = ofs_y + 720;
        let left0  = ofs_x;
        let right0 = ofs_x + 1280;

        let pitch = (regions.len() as f64).sqrt() as usize;
        for row in 0..pitch {
            for col in 0..pitch {
                // (0,0), (0, 720)
                let x = (col * 1280) as i64;
                let y = (row *  720) as i64;
                let ridx = col + (row * pitch);

                let top1   = y;
                let bot1   = y + 720;
                let left1  = x;
                let right1 = x + 1280;

                let in_scanbox = right1 >= left0 && left1 < right0
                    && top1 < bot0 && bot1 >= top0;

                if in_scanbox {
                    if !regions[ridx].is_init {
                        let txbuf = self.display.get_texture(1280,720);
                        regions[ridx].is_init = true;
                        regions[ridx].is_hot  = true;
                        regions[ridx].texture = Some(txbuf);

                        Engine::with_texture(&mut self.display, &mut regions[ridx].texture, |io| {
                            io.clear_buffer();
                            io.fill_rect(Rect::new(0,0,1280,5),   Color::RGB(255,0,0));  // top
                            io.fill_rect(Rect::new(0,715,1280,5), Color::RGB(255,0,0));  // bottom
                            io.fill_rect(Rect::new(0,0,5,720),    Color::RGB(255,0,0) ); // left
                            io.fill_rect(Rect::new(1275,0,5,720), Color::RGB(255,0,0));  // right
                        });
                    }

                    if !regions[ridx].is_hot { regions[ridx].swap_in(&mut self.display, &mut self.swapbuf) }

                    self.display.copy_t(regions[ridx].texture.as_ref().unwrap(),
                        Rect::new(0, 0, 1280, 720),
                        Rect::new((x - ofs_x) as i32, (y - ofs_y) as i32, 1280, 720));
                } else {
                    regions[ridx].swap_out(&mut self.display);
                }
            }
        }
    }
}
