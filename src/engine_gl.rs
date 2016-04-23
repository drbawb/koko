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
use units::V2;

static BASIC_VRT: &'static str = include_str!("shaders/basic.v.glsl");
static BASIC_FRG: &'static str = include_str!("shaders/basic.f.glsl");
static FLAT_VRT:  &'static str = include_str!("shaders/flat.v.glsl");

static MAX_VERTS: usize = 256;

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

pub struct Engine {
    is_running: bool,

    context:    GlutinFacade,
    controller: Input,
}

impl Engine {
    pub fn new(gl_ctx: GlutinFacade) -> Engine {
        Engine {
            is_running: true,

            context:    gl_ctx,
            controller: Input::new(),
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

        let indices = glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList);
        let indices_pts = glium::index::NoIndices(glium::index::PrimitiveType::Points);
        
        let mut vbuf_cursor = glium::VertexBuffer::new(&self.context, &shape[..])
            .ok().expect("could not alloc vbuf");
        
        let mut vbuf_points = glium::VertexBuffer::empty_dynamic(&self.context, MAX_VERTS)
            .ok().expect("could not alloc vbuf");


        let program = match glium::Program::from_source(&self.context, BASIC_VRT, BASIC_FRG, None) {
            Ok(program) => program,
            Err(msg) => panic!("could not load shader: {}", msg),
        };

        let path_program = match glium::Program::from_source(&self.context, FLAT_VRT, BASIC_FRG, None) {
            Ok(program) => program,
            Err(msg) => panic!("could not load shader: {}", msg),
        };

        // current cursor state
        let mut cursor_x = 0;
        let mut cursor_y = 0;

        // control point buffers
        let mut input_buffers = vec![];
        let mut input_samples = Vec::with_capacity(MAX_VERTS);
        let mut cursor_commit = true;
        let mut cursor_down   = false;

        // text renedring
        let text_blitter = TextBlitter::new(&mut self.context);
        let mut text_count  = 0;
        let mut frame_count = 0;
        let mut text_scale  = 0.0;

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

            // TODO: (?) deduplicate input against last input?
            // store user input into control point buffers
            let (wx, wy) = Engine::world_to_unit(cursor_x as f64, cursor_y as f64);
            if cursor_down {
                input_samples.push(ControlPoint {
                    screen_xy: V2(cursor_x as i64, cursor_y as i64),
                });
            } else if !cursor_down && !cursor_commit {
                // swap the input buffer with a fresh one
                let mut input_buf = Vec::with_capacity(MAX_VERTS);
                mem::swap(&mut input_samples, &mut input_buf);

                // allocate gpu memory for them
                // TODO: swap these out
                println!("added {} points", input_buf.len());
                let pathbuf = ControlPath::new(&self.context, input_buf);

                // commit the dirty one to heap
                input_buffers.push(pathbuf);
                cursor_commit = true;
            }

            if self.controller.was_key_pressed(KeyCode::Escape) {
                self.is_running = false;
            }

            if self.controller.was_key_pressed(KeyCode::Up) {
                text_scale += 0.05;
            } else if self.controller.was_key_pressed(KeyCode::Down) {
                text_scale -= 0.05;
            }

            // composite frame
            let mut target = self.context.draw();
            target.clear_color(0.05, 0.05, 0.05, 1.0);

            let tri_params = glium::DrawParameters {
                .. Default::default()
            };

            // draw cursor
            let cursor_uni = uniform! {
                ofs:   [wx as f32, wy as f32, 0.0f32], 
                scale: 0.15f32,
            };

            target.draw(&vbuf_cursor, &indices, &program, &cursor_uni, &tri_params)
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

                target.draw(&vbuf_points, &indices, &program, &path_uni, &tri_params)
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
                target.draw(&path.buffer, &indices, &path_program, &path_uni, &tri_params)
                    .ok().expect("could not blit cursor example");
            }

            // show frame time
            let mut time_ms = 0;
            let time = Instant::now().duration_since(frame_start_at);
            time_ms += time.as_secs() * 1000;
            time_ms += time.subsec_nanos() as u64 / 1000 / 1000;


            // TODO: helper for this
            // strlen =>  (char width * text length) * scale
            let text_out = format!("frame time {}ms", time_ms);

            // the text size is
            // (why /128 and not /256 ???)
            // char width: 16 * (aspect correction) / 128
            // * num chars
            // * scale of text
            //
            let strlen =
                ((16.0 * (720.0 / 1280.0)) / 128.0)
                * text_out.len() as f32
                * text_scale;

                // ((16.0 / 128.0) * text_out.len() as f32) * text_scale;
            text_blitter.draw(&text_out[..], text_scale, (1.0 - strlen, 1.0), &mut target);

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

    fn world_to_unit(x: f64, y: f64) -> (f64, f64) {
        let adj_x = x / 640.0;
        let adj_y = y / 360.0;
        ( (adj_x - 1.0), -(adj_y - 1.0) )
    }
}
