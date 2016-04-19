use std::mem;
use std::thread;
use std::time::{Duration, Instant};

use glium::backend::glutin_backend::GlutinFacade;
use glium::glutin::{self, ElementState, Event, VirtualKeyCode as KeyCode};
use glium::{self,Surface};

use graphics_gl::Vert2;
use input::Input;
use units::V2;

const MAX_VERTS: usize = 128;

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

        let mut frame_start_at;
        let mut elapsed_time = Duration::from_millis(0);

        // simple program
        let v_shade = r#"
            #version 140
        
            in  vec2 pos;
            in  vec3 color;
            out vec4 px_color;
            uniform float t;
        
            void main() {
                px_color = vec4(color, 1.0);
                gl_Position = vec4(pos, 0.0, 1.0);
            }
        "#;
        
        let v_frag = r#"
            #version 140
       
            in  vec4 px_color;
            out vec4 color;
        
            void main() {
                color = px_color;
            }
        "#;

        let mut shape = [
            Vert2 { pos: [-0.5, -0.5], color: [1.0, 0.0, 0.0] },
            Vert2 { pos: [ 0.5, -0.5], color: [0.0, 1.0, 0.0] },
            Vert2 { pos: [ 0.5,  0.5], color: [0.0, 0.0, 1.0] },

            Vert2 { pos: [-0.5, -0.5], color: [1.0, 0.0, 0.0] },
            Vert2 { pos: [-0.5,  0.5], color: [0.0, 1.0, 0.0] },
            Vert2 { pos: [ 0.5,  0.5], color: [0.0, 0.0, 1.0] },
        ];

        let indices = glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList);
        let mut vbuf = glium::VertexBuffer::dynamic(&self.context, &shape)
            .ok().expect("could not alloc vbuf");

        let program = glium::Program::from_source(&self.context, v_shade, v_frag, None)
            .ok().expect("could not load shaders");


        let mut cursor_pts = vec![];
        let mut cursor_down  = false;
        let mut cursor_drawn = false;

        let mut cursor_x = 0;
        let mut cursor_y = 0;

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

                    Event::MouseInput(ElementState::Pressed,  _)  => cursor_down = true,
                    Event::MouseInput(ElementState::Released, _) => { 
                        cursor_down  = false;
                        cursor_drawn = false;
                    },

                    Event::MouseMoved(x,y) => { cursor_x = x; cursor_y = y },

                    _ => (),
                }
            }

            // let world_x = (cursor_x as f32 / 640.0) - 1.0;
            // let world_y = 1.0 - (cursor_y as f32 / 360.0);
            if cursor_down {
                cursor_pts.push(V2(cursor_x as i64, cursor_y as i64));
            }

            if self.controller.was_key_pressed(KeyCode::Escape) {
                self.is_running = false;
            }

            // composite frame
            let mut target = self.context.draw();
            target.clear_color(0.05, 0.05, 0.05, 1.0);

            let tri_params = glium::DrawParameters {
                line_width: Some(16.0),
                point_size: Some(5.0),
                .. Default::default()
            };

            target.draw(&vbuf, &indices, &program, &uniform! { t: 0.0f32 }, &tri_params)
                .ok().expect("could not blit triangle example");

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
}
