use std::collections::LinkedList;
use std::mem;
use std::thread;
use std::time::{Duration, Instant};

use glium::backend::glutin_backend::GlutinFacade;
use glium::buffer::Content;
use glium::glutin::{ElementState, Event, VirtualKeyCode as KeyCode};
use glium::{self, Surface, VertexBuffer};

use graphics_gl::{TextBlitter, Vert2};
use input::Input;
use units::{Color, V2};

static BASIC_VRT: &'static str = include_str!("shaders/basic.v.glsl");
static BASIC_FRG: &'static str = include_str!("shaders/basic.f.glsl");
static FLAT_VRT:  &'static str = include_str!("shaders/flat.v.glsl");

static MAX_VERTS: usize = 256;

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

struct ControlPath {
    needs_render: bool,

    pub buffer: VertexBuffer<Vert2>,
    pub samples: Vec<ControlPoint>,
}

impl ControlPath {
    pub fn new(context: &GlutinFacade, points: Vec<ControlPoint>) -> ControlPath {
        let mut vbuf_path = glium::VertexBuffer::empty_dynamic(context, points.len() * 6)
            .ok().expect("could not alloc vbuf");


        ControlPath {
            needs_render: true,

            buffer:  vbuf_path,
            samples: points,
        }
    }

    // cleans up shop and prepares buffer for a draw call
    pub fn draw(&mut self) {
        if !self.needs_render { return; }
        self.needs_render = false;

        self.buffer.invalidate();
        let mut writer = self.buffer.map_write();
        let fudge = 10.0 / 720.0;
        let mut ofs = 0;
        for point in &self.samples {
            let (wx, wy) = {
                let adj_x = (point.screen_xy.0 as f32 / 360.0) * 720.0 / 1280.0;
                let adj_y = (point.screen_xy.1 as f32 / 360.0) * 1.0;
                ( (adj_x - 1.0), -(adj_y - 1.0) )
            };

            writer.set(ofs + 0, Vert2 { pos: [        wx,        wy,   0.0], color: [0.75, 0.0, 0.5] });
            writer.set(ofs + 1, Vert2 { pos: [   wx+fudge,       wy,   0.0], color: [0.75, 0.0, 0.5] });
            writer.set(ofs + 2, Vert2 { pos: [         wx,  wy-fudge,  0.0], color: [0.75, 0.0, 0.5] });
            writer.set(ofs + 3, Vert2 { pos: [         wx,  wy-fudge,  0.0], color: [0.75, 0.0, 0.5] });
            writer.set(ofs + 4, Vert2 { pos: [   wx+fudge,  wy-fudge,  0.0], color: [0.75, 0.0, 0.5] });
            writer.set(ofs + 5, Vert2 { pos: [   wx+fudge,        wy,   0.0], color: [0.75, 0.0, 0.5] });
            
            ofs += 6;
        }

        println!("final init offset: {}", ofs);
    }
}

/// Represents a mouse-input sample from some brush
struct ControlPoint {
    screen_xy: V2,
}

struct Region {
    pub paths: Vec<ControlPath>, // TODO: paths can span regions
}

impl Region {
    pub fn new() -> Region {
        Region { paths: vec![] }
    }
}

pub struct Engine {
    is_running: bool,

    context:    GlutinFacade,
    controller: Input,

    indices_tris: glium::index::NoIndices,
    indices_pts:  glium::index::NoIndices,
    program:      glium::Program,
    path_program: glium::Program,

    brush:   BrushMode,
    color:   (u8, u8, u8),
    cursor:  V2,
    scanbox: V2,
}

impl Engine {
    pub fn new(gl_ctx: GlutinFacade) -> Engine {

        let indices = glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList);
        let indices_pts = glium::index::NoIndices(glium::index::PrimitiveType::Points);

        let basic_shader = match glium::Program::from_source(&gl_ctx, BASIC_VRT, BASIC_FRG, None) {
            Ok(program) => program,
            Err(msg) => panic!("could not load shader: {}", msg),
        };

        let flat_shader = match glium::Program::from_source(&gl_ctx, FLAT_VRT, BASIC_FRG, None) {
            Ok(program) => program,
            Err(msg) => panic!("could not load shader: {}", msg),
        };

        Engine {
            is_running: true,

            context:    gl_ctx,
            controller: Input::new(),

            indices_tris: indices,
            indices_pts:  indices_pts,
            program:      basic_shader,
            path_program: flat_shader,

            brush:   BrushMode::Squareish,
            color:   (125, 0, 175),
            cursor:  V2(0, 0),
            scanbox: V2(0,0),
        }
    }

    pub fn run(&mut self) {
        let target_fps_ms = Duration::from_millis(1000 / 120); // TODO: const fn?
        let game_start_at = Instant::now();

        let mut frame_start_at;
        let mut elapsed_time;

        // draw a basic shape using standard shader
        let shape = vec![
            // face 1
            Vert2 { pos: [ 1.0,  0.0, 0.0], color: [1.0, 0.0, 0.0] },
            Vert2 { pos: [ 0.0,  0.0, 0.0], color: [1.0, 0.0, 0.0] },
            Vert2 { pos: [ 0.0, -1.0, 0.0], color: [1.0, 0.0, 0.0] },
        ];

        
        let mut vbuf_cursor = glium::VertexBuffer::new(&self.context, &shape[..])
            .ok().expect("could not alloc vbuf");
        
        let mut vbuf_points = glium::VertexBuffer::empty_dynamic(&self.context, MAX_VERTS)
            .ok().expect("could not alloc vbuf");

        // current cursor state
        let mut cursor_x = 0;
        let mut cursor_y = 0;

        // control point buffers
        let mut regions = vec![Region::new()];
        let mut input_buffers: Vec<ControlPath>  = vec![];
        let mut input_samples: Vec<ControlPoint> = Vec::with_capacity(MAX_VERTS);
        let mut cursor_commit = true;
        let mut cursor_down   = false;

        // text renedring
        let text_blitter = TextBlitter::new(&mut self.context);
        let mut text_count  = 0;
        let mut frame_count = 0;

        while self.is_running {
            // cut new frame
            frame_start_at = Instant::now();
            self.controller.begin_new_frame();

            // process platform events 
            for evt in self.context.poll_events() { 
                match evt {
                    Event::Closed => self.is_running = false,
                    Event::KeyboardInput(ElementState::Pressed, _, Some(key)) => {
                        self.controller.key_down_event(key);
                    },

                    Event::KeyboardInput(ElementState::Released, _, Some(key)) => {
                        self.controller.key_up_event(key);
                    },

                    Event::MouseInput(ElementState::Pressed,  _)  => {
                        cursor_down = true;
                        cursor_commit = false;
                    },

                    Event::MouseInput(ElementState::Released, _)  => cursor_down = false,

                    Event::MouseMoved(x,y) => { cursor_x = x; cursor_y = y },

                    _ => (),
                }
            }

            // handle user keyboard input
            if self.controller.was_key_pressed(KeyCode::Escape) {
                self.is_running = false;
            }
            
            if self.controller.is_key_held(KeyCode::I) {
                self.color.0 = self.color.0.wrapping_add(0x01);
            } else if self.controller.is_key_held(KeyCode::O) {
                self.color.1 = self.color.1.wrapping_add(0x01);
            } else if self.controller.is_key_held(KeyCode::P) {
                self.color.2 = self.color.2.wrapping_add(0x01);
            }

            if self.controller.is_key_held(KeyCode::Up) {
                self.scanbox = self.scanbox + V2(0, 5);
            } else if self.controller.is_key_held(KeyCode::Down) {
                self.scanbox = self.scanbox - V2(0, 5);
            } else if self.controller.is_key_held(KeyCode::Left) {
                self.scanbox = self.scanbox - V2(5, 0);
            } else if self.controller.is_key_held(KeyCode::Right) {
                self.scanbox = self.scanbox + V2(5, 0);
            }


            // alloc new region
            let V2(rx,ry) = self.scanbox + V2(1280, 720);
            let region_sqrt = (regions.len() as f64).sqrt();
            if rx as f64 > (region_sqrt * 1280.0) || ry as f64 > (region_sqrt * 720.0){
                println!("growing up/right");
                let pitch = (region_sqrt + 1.0) as usize;
                let next_square = pitch * pitch;

                let mut buf = Vec::with_capacity(next_square);
                mem::swap(&mut regions, &mut buf);
                let mut old_drain = buf.into_iter();

                for row in 0..pitch {
                    for col in 0..pitch {
                        if (row >= pitch - 1) || (col >= pitch - 1) {
                            regions.push(Region::new());
                        } else {
                            regions.push(old_drain.next().expect("ran out of regions to copy during regrow !!!"));
                        }
                    }
                }
            }

            if (self.scanbox.0 < 0) || (self.scanbox.1 < 0) {
                println!("need to regrow left/down");
                let pitch = (region_sqrt + 1.0) as usize;
                let next_square = pitch * pitch;

                let mut buf = Vec::with_capacity(next_square);
                mem::swap(&mut regions, &mut buf);
                let mut old_drain = buf.into_iter();

                let mut idx = 0;
                for row in 0..pitch {
                    for col in 0..pitch {
                        if (row == 0) || (col == 0) {
                            println!("[{},{}] => new", row, col);
                            regions.push(Region::new());
                        } else {
                            println!("[{},{}] => {}", row, col, idx);
                            idx += 1;
                            regions.push(old_drain.next().expect("ran out of regions to copy during regrow !!!"));
                        }
                    }
                }

                self.scanbox = V2(1280, 720) + self.scanbox;
            }



            // // copy old / allocate new
            // for row in 0..pitch {
            //     for col in 0..pitch {
            //         if (row >= pitch - 1) || (col >= pitch - 1) {
            //             regions.push(Region::new());
            //         } else {
            //             regions.push(old_drain.next().expect("ran out of regions to copy during regrow!"));
            //         }
            //     }
            // }

            // handle cursor input
            //
            // we convert the control point to it's unit-position
            // inside a particular region of the scanbox
            //
            let brush = self.brush;
            let V2(ofs_x, ofs_y) = self.scanbox;

            let x1 = cursor_x as i64 + ofs_x;
            let y1 = cursor_y as i64 + ofs_y;

            let pitch = (regions.len() as f64).sqrt() as i64;
            let col  = x1 / 1280;
            let row  = y1 /  720;
            let ridx = (row * pitch) + col; // row * 3rows/col + col

            // blit in that region instead
            //println!("[{}], row: {}, col: {}", ridx, row, col);
            //println!("real ({},{})", x1, y1);
            let x1 = match x1 > 1280 {
                true  => x1 % (1280 * col+1),
                false => x1,
            };

            let y1 = match y1 > 720 {
                true  => y1 % ( 720 * row+1 ),
                false => y1,
            };
            //println!("adj ({},{})", x1, y1);


            // store user input into control point buffer for the computed region
            let (wx, wy) = Engine::world_to_unit(x1 as f64, y1 as f64);
            if cursor_down {
                input_samples.push(ControlPoint {
                    screen_xy: V2(x1 as i64, y1 as i64),
                });
            } else if !cursor_down && !cursor_commit {
                // swap the input buffer with a fresh one
                let mut input_buf = Vec::with_capacity(MAX_VERTS);
                mem::swap(&mut input_samples, &mut input_buf);

                // compute starting ridx
                let pitch = (regions.len() as f64).sqrt() as i64;
                let col  = input_buf[0].screen_xy.0 / 1280;
                let row  = input_buf[0].screen_xy.1 /  720;
                let ridx = (row * pitch) + col; // row * 3rows/col + col

                // TODO: coordinate needs to be adjusted if it crosses regions as well
                // commit dirty one to heap
                println!("added {} points", input_buf.len());
                let pathbuf = ControlPath::new(&self.context, input_buf);
                regions[ridx as usize].paths.push(pathbuf);
                cursor_commit = true;
            }
            
            // if x1 + 5 < 1280 && x2 < 1280 && y1 < 720 && y2 + 10 < 720 {
            //     // draw control point
            // }

            // let diffx = x2 - x1;
            // let diffy = y2 - y1;
            // println!("delta ({},{}), mag: {}", diffx, diffy, diffy-diffx);


            // composite frame
            let mut target = self.context.draw();
            target.clear_color(0.05, 0.05, 0.05, 1.0);

            let tri_params = glium::DrawParameters {
                .. Default::default()
            };

            // draw cursor
            let (wx, wy) = Engine::world_to_unit(cursor_x as f64, cursor_y as f64);
            
            let cursor_uni = uniform! {
                ofs:   [wx as f32, wy as f32, 0.0f32], 
                scale: 0.15f32,
            };

            target.draw(&vbuf_cursor, &self.indices_tris, &self.program, &cursor_uni, &tri_params)
                .ok().expect("could not blit cursor example");

            // draw control points
            {
                // vbuf.invalidate();
                let mut writer = vbuf_points.map_write();
                writer.set(0, Vert2 { pos: [-1.0,  1.0,  0.0], color: [1.0, 0.0, 1.0] });
                writer.set(1, Vert2 { pos: [-1.0, -1.0,  0.0], color: [1.0, 0.0, 1.0] });
                writer.set(2, Vert2 { pos: [ 1.0,  1.0,  0.0], color: [1.0, 0.0, 1.0] });

                writer.set(3, Vert2 { pos: [ 1.0,  1.0,  0.0], color: [1.0, 0.0, 1.0] });
                writer.set(4, Vert2 { pos: [ 1.0, -1.0,  0.0], color: [1.0, 0.0, 1.0] });
                writer.set(5, Vert2 { pos: [-1.0, -1.0,  0.0], color: [1.0, 0.0, 1.0] });
            }

            // inflate each control point to six verts
            for point in input_samples.iter() {
                let (wx, wy) = Engine::world_to_unit(point.screen_xy.0 as f64,
                                                     point.screen_xy.1 as f64);

                let path_uni = uniform! {
                    ofs:   [wx as f32, wy as f32, 0.0f32], 
                    scale: 0.015f32,
                };

                target.draw(&vbuf_points, &self.indices_tris, &self.program, &path_uni, &tri_params)
                    .ok().expect("could not blit cursor example");
            }

            // for each path draw control point there
            for path in &mut input_buffers {
                let path_uni = uniform! {
                    ofs:   [0.0, 0.0, 0.0f32], 
                    scale: 1.0f32,
                };

                // inflate each control point to six verts
                path.draw();
                target.draw(&path.buffer, &self.indices_tris, &self.path_program, &path_uni, &tri_params)
                    .ok().expect("could not blit cursor example");
            }

            // show frame time
            let mut time_ms = 0;
            let time = Instant::now().duration_since(frame_start_at);
            time_ms += time.as_secs() * 1000;
            time_ms += time.subsec_nanos() as u64 / 1000 / 1000;


            // TODO: helper for this
            // strlen =>  (char width * text length) * scale
            let (hue_r, hue_g, hue_b) = self.color;
            let buf_1 = format!("{}ms, # regions: {}, # drawn: {}, sb @ {:?}",
                              time_ms, regions.len(), 0, self.scanbox);

            let buf_2 = format!("e = erase all, b = brush ({:?}), hue(i,o,p) => ({:02x},{:02x},{:02x})",
                               self.brush, hue_r, hue_g, hue_b);

            // the text size is
            // (why /128 and not /256 ???)
            // char width: 16 * (aspect correction) / 128
            // * num chars
            // * scale of text
            //
            let text_scale = 0.30;
            let strlen1 =
                ((16.0 * (720.0 / 1280.0)) / 128.0)
                * buf_1.len() as f32
                * text_scale;

            let strlen2 =
                ((16.0 * (720.0 / 1280.0)) / 128.0)
                * buf_2.len() as f32
                * text_scale;
            
            let strheight = (16.0 / 128.0) * text_scale;

                // ((16.0 / 128.0) * text_out.len() as f32) * text_scale;
            text_blitter.draw(&buf_1[..], text_scale, (1.0 - strlen1, 1.0), &mut target);
            text_blitter.draw(&buf_2[..], text_scale, (1.0 - strlen2, 1.0 - strheight), &mut target);

            self.draw_regions(&mut regions, &mut target);

            target.finish()
                .ok().expect("could not render frame");

            // sleep for a bit if we made our deadline
            elapsed_time = frame_start_at.elapsed();
            let sleep_time = if elapsed_time > target_fps_ms {
                Duration::from_millis(0)
            } else { target_fps_ms - elapsed_time };

            thread::sleep(sleep_time);
        }
    }

    fn draw_regions(&mut self, regions: &mut Vec<Region>, target: &mut glium::Frame) {
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
                println!("ridx: {}", ridx);

                let top1   = y;
                let bot1   = y + 720;
                let left1  = x;
                let right1 = x + 1280;

                let in_scanbox = right1 >= left0 && left1 < right0
                    && top1 < bot0 && bot1 >= top0;

                if true {
                    let wofs_x = (ofs_x as f32 / 1280.0);
                    let wofs_y = (ofs_y as f32 /  720.0);

                    let bofs_x = (col as f32 * 1280.0) / 1280.0;
                    let bofs_y = (row as f32 *  720.0) /  720.0;
                    println!("ofs: ({},{}), wofs: ({},{})", bofs_x, bofs_y, wofs_x, wofs_y);

                    for path in &mut regions[ridx].paths {
                        let path_uni = uniform! {
                            ofs:   [-bofs_x + wofs_x, bofs_y - wofs_y, 0.0f32], // TODO: check why X needs to be inverted
                            scale: 1.0f32,
                        };

                        // inflate each control point to six verts
                        path.draw();
                        target.draw(&path.buffer, &self.indices_tris, &self.path_program, &path_uni, &Default::default())
                            .ok().expect("could not blit cursor example");
                    }
                }
            }
        }
    }

    fn world_to_unit(x: f64, y: f64) -> (f64, f64) {
        let adj_x = x / 640.0;
        let adj_y = y / 360.0;
        ( (adj_x - 1.0), -(adj_y - 1.0) )
    }
}
