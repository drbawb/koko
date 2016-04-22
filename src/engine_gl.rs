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

static MAX_VERTS: usize = 256;

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
        
        let mut vbuf_cursor = glium::VertexBuffer::new(&self.context, &shape[..])
            .ok().expect("could not alloc vbuf");
        
        let mut vbuf_points = glium::VertexBuffer::empty_dynamic(&self.context, MAX_VERTS)
            .ok().expect("could not alloc vbuf");


        let program = match glium::Program::from_source(&self.context, BASIC_VRT, BASIC_FRG, None) {
            Ok(program) => program,
            Err(msg) => panic!("could not load shader: {}", msg),
        };

        // current cursor state
        let mut cursor_x = 0;
        let mut cursor_y = 0;

        // control point buffers
        let mut input_buffers = vec![];
        let mut input_samples = LinkedList::new();
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
                input_samples.push_front(ControlPoint {
                    screen_xy: V2(cursor_x as i64, cursor_y as i64),
                });
            } else if !cursor_down && !cursor_commit {
                // swap the input buffer with a fresh one
                let mut input_buf = LinkedList::new();
                mem::swap(&mut input_samples, &mut input_buf);

                // commit the dirty one to heap
                input_buffers.push(input_buf);
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

            let mut time_ms = 0.0;
            let time = Instant::now().duration_since(game_start_at);
            time_ms += time.as_secs() as f64 * 1000.0;
            time_ms += time.subsec_nanos() as f64 * 0.001 * 0.001;

            frame_count += 1;
            if frame_count > 10 {
                frame_count = 0;
                text_count = (text_count + 1) % 0xFF;
            }

            // TODO: helper for this
            // strlen =>  (char width * text length) * scale
            let text_out = format!("debug mode 0x{:02X}", text_count);

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

            // draw cursor
            let cursor_uni = uniform! {
                ofs:   [wx as f32, wy as f32, 0.0f32], 
                scale: 0.15f32,
                timer: time_ms as f32 * 0.001,
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
                    scale: 0.05f32,
                    timer: time_ms as f32 * 0.001,
                };

                target.draw(&vbuf_points, &indices, &program, &path_uni, &tri_params)
                    .ok().expect("could not blit cursor example");
            }

            // for each path draw control point there
            for path in input_buffers.iter() {
                // inflate each control point to six verts
                for point in path.iter() {
                    let (wx, wy) = Engine::world_to_unit(point.screen_xy.0 as f64,
                                                         point.screen_xy.1 as f64);

                    let path_uni = uniform! {
                        ofs:   [wx as f32, wy as f32, 0.0f32], 
                        scale: 0.05f32,
                        timer: time_ms as f32 * 0.001,
                    };

                    target.draw(&vbuf_points, &indices, &program, &path_uni, &tri_params)
                        .ok().expect("could not blit cursor example");
                }
            }
            
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
