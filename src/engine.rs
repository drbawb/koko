use std::mem;
use std::thread;
use std::time::{Duration, Instant};

use glium::glutin::{DeviceEvent, WindowEvent, Event, EventsLoop};
use glium::glutin::{ElementState, VirtualKeyCode as KeyCode};
use glium::{self, Display, Surface, VertexBuffer};

use graphics::{TextBlitter, Vert2};
use input::Input;
use units::{Color, V2};

static BASIC_VRT: &'static str = include_str!("shaders/basic.v.glsl");
static BASIC_FRG: &'static str = include_str!("shaders/basic.f.glsl");
static FLAT_VRT:  &'static str = include_str!("shaders/flat.v.glsl");

static MAX_VERTS: usize = 256;

pub static COLOR_BG:  Color = Color::RGB(0,0,0);
pub static COLOR_FPS: Color = Color::RGB(255,255,0);
pub static COLOR_PEN: Color = Color::RGB(125, 0, 175);

#[allow(dead_code)]
#[derive(Copy, Clone, Debug)]
enum BrushMode {
    Normal,
    Squareish,
    WowSoEdgy,
    Eraser,
}

struct ControlPath {
    needs_render: bool,

    pub buffer:  VertexBuffer<Vert2>,
    pub samples: Vec<ControlPoint>,
    pub scale:   f32,
}

/// Represents a mouse-input sample from some brush
struct ControlPoint {
    screen_xy: V2,
}

impl ControlPath {
    pub fn new(context: &Display, scale: f32, scanbox: V2, points: Vec<ControlPoint>) -> ControlPath {
        let vbuf_path = glium::VertexBuffer::empty_dynamic(context, points.len() * 6)
            .expect("could not alloc vbuf");

        // NOTE: correct the cursor's position in the unit square to it's relative position
        //       by adding the current offset of the scanbox
        //
        // TODO: would be nice if the extremes of this path were stored in some sort
        //       of spatial data-structure so we can quickly query if the path is currently
        //       inside the scanbox -- this would enable some optimizations like skipping
        //       rendering and possibly removing out-of-bounds paths from VRAM.
        //
        let corrected_samples = points.iter().map(|point| {
            let adj_x = point.screen_xy.0 as f32 + (scanbox.0 as f32 / 2.0);
            let adj_y = point.screen_xy.1 as f32 - (scanbox.1 as f32 / 2.0);

            ControlPoint { screen_xy: V2(adj_x as i64, adj_y as i64) }
        }).collect();

        ControlPath {
            needs_render: true,

            buffer:  vbuf_path,
            samples: corrected_samples,
            scale: scale,
        }
    }

    // cleans up shop and prepares buffer for a draw call
    pub fn draw(&mut self, dim: (u32, u32)) {
        if !self.needs_render { return; }
        self.needs_render = false;

        self.buffer.invalidate();
        let mut writer = self.buffer.map_write();
        let fudge_x = 7.5 / 1280.0;
        let fudge_y = 7.5 /  720.0;
        let (win_x, win_y) = dim;
        
        let mut ofs = 0;
        for point in &self.samples {
            let (wx, wy) = {
                let adj_x = (point.screen_xy.0 as f32 / 360.0) * win_y as f32 / win_x as f32;
                let adj_y = (point.screen_xy.1 as f32 / 360.0) * 1.0;

                let inv_scale = 1.0 / self.scale;

                ( ((adj_x - 1.0) * inv_scale), -((adj_y - 1.0) * inv_scale) )
            };

            writer.set(ofs + 0, Vert2 { pos: [ wx-fudge_x, wy+fudge_y,  0.0], color: [0.75, 0.0, 0.5] });
            writer.set(ofs + 1, Vert2 { pos: [ wx+fudge_x, wy+fudge_y,  0.0], color: [0.75, 0.0, 0.5] });
            writer.set(ofs + 2, Vert2 { pos: [ wx-fudge_x, wy-fudge_y,  0.0], color: [0.75, 0.0, 0.5] });
            writer.set(ofs + 3, Vert2 { pos: [ wx-fudge_x, wy-fudge_y,  0.0], color: [0.75, 0.0, 0.5] });
            writer.set(ofs + 4, Vert2 { pos: [ wx+fudge_x, wy-fudge_y,  0.0], color: [0.75, 0.0, 0.5] });
            writer.set(ofs + 5, Vert2 { pos: [ wx+fudge_x, wy+fudge_y,  0.0], color: [0.75, 0.0, 0.5] });
            
            ofs += 6;
        }

        println!("final init offset: {}", ofs);
    }
}

pub struct Engine {
    is_running: bool,
    window_dim: (u32, u32),

    context:    Display,
    controller: Input,

    indices_tris: glium::index::NoIndices,
    _indices_pts: glium::index::NoIndices, // NOTE: unused; but ocasionally useful for debugging
    program:      glium::Program,
    path_program: glium::Program,

    brush:   BrushMode,
    color:   (u8, u8, u8),
    scale:   f32,
    scanbox: V2,
}

impl Engine {
    pub fn new(gl_ctx: Display) -> Engine {

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
            window_dim: (1280, 720), // TODO: init to actual window size

            context:    gl_ctx,
            controller: Input::new(),

            indices_tris: indices,
            _indices_pts: indices_pts,
            program:      basic_shader,
            path_program: flat_shader,

            brush:   BrushMode::Squareish,
            color:   (125, 0, 175),
            scale:   1.0,
            scanbox: V2(0,0),
        }
    }

    pub fn run(&mut self, events: &mut EventsLoop) {
        let target_fps_ms = Duration::from_millis(1000 / 120); // TODO: const fn?

        let mut frame_start_at;
        let mut elapsed_time;

        // draw a basic shape using standard shader
        let shape = vec![
            // face 1
            Vert2 { pos: [ 1.0,  0.0, 0.0], color: [1.0, 0.0, 0.0] },
            Vert2 { pos: [ 0.0,  0.0, 0.0], color: [1.0, 0.0, 0.0] },
            Vert2 { pos: [ 0.0, -1.0, 0.0], color: [1.0, 0.0, 0.0] },
        ];

        
        let vbuf_cursor = glium::VertexBuffer::new(&self.context, &shape[..])
            .expect("could not alloc vbuf");
        
        let mut vbuf_points = glium::VertexBuffer::empty_dynamic(&self.context, MAX_VERTS)
            .expect("could not alloc vbuf");

        // current cursor state
        let mut cursor_x = 0;
        let mut cursor_y = 0;
        let mut cursor_commit = true;
        let mut cursor_down   = false;
        
        // control point buffers
        let mut input_buffers: Vec<ControlPath>  = vec![];
        let mut input_samples: Vec<ControlPoint> = Vec::with_capacity(MAX_VERTS);
        let mut verts = 0;

        // text renedring
        let text_blitter = TextBlitter::new(&mut self.context);

        while self.is_running {
            // cut new frame
            frame_start_at = Instant::now();
            self.controller.begin_new_frame();

            // TODO: ick, callback based API
            // process platform events 
            events.poll_events(|evt| {
                match evt {
                    Event::WindowEvent { event, .. } => match event {
                        WindowEvent::CloseRequested => self.is_running = false,
                        WindowEvent::Resized(new_size) => {
                            self.window_dim = (
                                new_size.width  as u32,
                                new_size.height as u32,
                            );
                        },

                        WindowEvent::CursorMoved { position, .. } => {
                            // TODO: nasty casts ...
                            cursor_x = position.x as i64;
                            cursor_y = position.y as i64; 
                        },

                        // TODO: actually check which button is being pushed
                        WindowEvent::MouseInput { state: ElementState::Pressed, .. } => {
                            cursor_down = true;
                            cursor_commit = false;
                        },

                        WindowEvent::MouseInput { state: ElementState::Released, .. } => {
                            cursor_down = false;
                        },

                        WindowEvent::KeyboardInput { input, .. } => {
                            match (input.state, input.virtual_keycode) {
                                (ElementState::Pressed, Some(key)) => {
                                    self.controller.key_down_event(key);
                                },

                                (ElementState::Released, Some(key)) => {
                                    self.controller.key_up_event(key);
                                },

                                _ => {},
                            };
                        },


                        _ => {},
                    },


                    _ => {},
                }
            });

            // for evt in self.events.poll_events() { 
            //     match evt {
            //         Event::Closed       => self.is_running = false,
            //         Event::Resized(w,h) => self.window_dim = (w,h),

            //         Event::KeyboardInput(ElementState::Pressed, _, Some(key)) => {
            //             self.controller.key_down_event(key);
            //         },

            //         Event::KeyboardInput(ElementState::Released, _, Some(key)) => {
            //             self.controller.key_up_event(key);
            //         },

            //         Event::MouseInput(ElementState::Pressed,  _)  => {
            //             cursor_down = true;
            //             cursor_commit = false;
            //         },

            //         Event::MouseInput(ElementState::Released, _)  => cursor_down = false,

            //         Event::MouseMoved(x,y) => { cursor_x = x; cursor_y = y },

            //         _ => (),
            //     }
            // }

            // handle user keyboard input
            if self.controller.was_key_pressed(KeyCode::Escape) {
                self.is_running = false;
            }

            if self.controller.is_key_held(KeyCode::Equals) {
                self.scale += 0.005;

            } else if self.controller.is_key_held(KeyCode::Minus) {
                self.scale -= 0.005;
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


            // handle cursor input
            // store the user input into screen-relative control points
            // and then offset them based on the current scanbox.
            //
            if cursor_down {
                input_samples.push(ControlPoint {
                    screen_xy: V2(cursor_x as i64, cursor_y as i64),
                });
            } else if !cursor_down && !cursor_commit {
                // swap the input buffer with a fresh one
                let mut input_buf = Vec::with_capacity(MAX_VERTS);
                mem::swap(&mut input_samples, &mut input_buf);

                verts += input_buf.len() * 6;
                let pathbuf = ControlPath::new(&self.context, self.scale, self.scanbox, input_buf);
                input_buffers.push(pathbuf);
                cursor_commit = true;
            }
            
            // composite frame
            let mut target = self.context.draw();
            target.clear_color(0.05, 0.05, 0.05, 1.0);

            let tri_params = glium::DrawParameters {
                .. Default::default()
            };

            // draw cursor
            let (wx, wy) = Engine::world_to_unit(cursor_x as f64, cursor_y as f64);
            
            let cursor_uni = uniform! {
                dim:   [self.window_dim.0 as f32, self.window_dim.1 as f32],
                ofs:   [wx as f32, wy as f32, 0.0f32], 
                scale: 0.15f32,
            };

            target.draw(&vbuf_cursor, &self.indices_tris, &self.program, &cursor_uni, &tri_params)
                .expect("could not blit cursor example");

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
            for point in &input_samples {
                let (wx, wy) = Engine::world_to_unit(point.screen_xy.0 as f64,
                                                     point.screen_xy.1 as f64);

                let path_uni = uniform! {
                    ofs:   [wx as f32, wy as f32, 0.0f32], 
                    scale: 0.015f32,
                };

                target.draw(&vbuf_points, &self.indices_tris, &self.program, &path_uni, &tri_params)
                    .expect("could not blit cursor example");
            }

            // show frame time
            let mut time_ms = 0;
            let time = Instant::now().duration_since(frame_start_at);
            time_ms += time.as_secs() * 1000;
            time_ms += time.subsec_nanos() as u64 / 1000 / 1000;


            // TODO: helper for this
            // strlen =>  (char width * text length) * scale
            let (hue_r, hue_g, hue_b) = self.color;
            let buf_1 = format!("{}ms [# paths: {}]  [# verts: {}] [sb @ {:?}] [scale @ {:?}]",
                              time_ms, input_buffers.len(), verts, self.scanbox, self.scale);

            let buf_2 = format!("e = erase all, b = brush ({:?}), hue(i,o,p) => ({:02x},{:02x},{:02x})",
                               self.brush, hue_r, hue_g, hue_b);

            // the text size is
            // (why /128 and not /256 ???)
            // char width: 16 * (aspect correction) / 128
            // * num chars
            // * scale of text
            //
            let text_scale = 0.25;
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

            self.draw_regions(&mut input_buffers[..], &mut target);

            target.finish()
                .expect("could not render frame");

            // sleep for a bit if we made our deadline
            elapsed_time = frame_start_at.elapsed();
            let sleep_time = if elapsed_time > target_fps_ms {
                Duration::from_millis(0)
            } else { target_fps_ms - elapsed_time };

            thread::sleep(sleep_time);
        }
    }

    fn draw_regions(&mut self, paths: &mut [ControlPath], target: &mut glium::Frame) {
        let V2(ofs_x, ofs_y) = self.scanbox;

        let unit_ofs_x = ofs_x as f32 / 1280.0; // offset of the scanbox converted to the screen space unit square
        let unit_ofs_y = ofs_y as f32 /  720.0; // offset of the scanbox converted to the screen space unit square
        
        for path in paths {
            let path_uni = uniform! {
                ofs:   [-unit_ofs_x, -unit_ofs_y, 0.0f32],
                scale: self.scale,
            };
            
            // inflate each control point to six verts
            path.draw(self.window_dim);
            target.draw(&path.buffer, &self.indices_tris, &self.path_program, &path_uni, &Default::default())
                .expect("could not blit cursor example");
        }
    }

    fn world_to_unit(x: f64, y: f64) -> (f64, f64) {
        let adj_x = x / 640.0;
        let adj_y = y / 360.0;
        ( (adj_x - 1.0), -(adj_y - 1.0) )
    }
}
