use std::io::Cursor;
use std::thread;
use std::time::{Duration, Instant};

use glium::backend::glutin_backend::GlutinFacade;
use glium::glutin::{ElementState, Event, VirtualKeyCode as KeyCode};
use glium::{self, texture, Surface};

use graphics_gl::Vert2;
use input::Input;
use units::V2;

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

        // simple program
        let v_shade = r#"
            #version 140

            in  vec3 pos;
            in  vec3 color;
            out vec4 px_color;
            out vec2 texcoord;
            out float fade_factor;
           
            uniform vec3      ofs;
            uniform vec2 char_pos;
            uniform float   scale;
            uniform float   timer;
        
            void main() {
                mat4 projection = mat4(
                    vec4(720.0/1280.0, 0.0, 0.0, 0.0),
                    vec4(         0.0, 1.0, 0.0, 0.0),
                    vec4(         0.0, 0.0, 0.5, 0.5),
                    vec4(         0.0, 0.0, 0.0, 1.0)
                );

                mat4 rotation = mat4(
                    vec4(1.0,         0.0,         0.0, 0.0),
                    vec4(0.0,  cos(timer), -sin(timer), 0.0),
                    vec4(0.0,  sin(timer),  cos(timer), 0.0),
                    vec4(0.0,         0.0,         0.0, 1.0)
                );

                mat4 translate = mat4(
                    vec4(  1.0,   0.0,  0.0,  0.0),
                    vec4(  0.0,   1.0,  0.0,  0.0),
                    vec4(  0.0,   0.0,  1.0,  0.0),
                    vec4(ofs.x, ofs.y,  0.0,  1.0)
                );

                mat4 scale = mat4(
                    vec4(scale,   0.0,   0.0,  0.0),
                    vec4(  0.0, scale,   0.0,  0.0),
                    vec4(  0.0,   0.0, scale,  0.0),
                    vec4(  0.0,   0.0,   0.0,  1.0)
                );

                vec4 pos3d     = vec4(pos, 1.0);
                vec4 proj_pos  = translate * projection * scale * pos3d;
                float perspective_factor = proj_pos.z * 0.5 + 1.0;
                
                texcoord = char_pos + (scale * pos3d).xy;

                gl_Position = proj_pos/perspective_factor;
                px_color    = vec4(color, 1.0);
                fade_factor = sin(timer) * 0.5 + 0.5;
            }
        "#;
        
        let v_frag = r#"
            #version 140
       
            in  vec4 px_color;
            in  vec2 texcoord;
            out vec4 color;

            uniform sampler2D tex;
        
            void main() {
                color = texture(tex, texcoord);
            }
        "#;

        let shape = [
            // face 1
            Vert2 { pos: [ 0.0,  0.0, 0.0], color: [0.0, 1.0, 0.0] },
            Vert2 { pos: [ 0.0, -0.5625, 0.0], color: [0.0, 0.0, 1.0] },
            Vert2 { pos: [ 1.0,  0.0, 0.0], color: [1.0, 0.0, 0.0] },

            Vert2 { pos: [ 0.0, -0.5625, 0.0], color: [0.0, 0.0, 1.0] },
            Vert2 { pos: [ 1.0, -0.5625, 0.0], color: [0.0, 1.0, 0.0] },
            Vert2 { pos: [ 1.0,  0.0, 0.0], color: [1.0, 0.0, 0.0] },
        ];

        let indices = glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList);
        let vbuf = glium::VertexBuffer::dynamic(&self.context, &shape)
            .ok().expect("could not alloc vbuf");

        let program = match glium::Program::from_source(&self.context, v_shade, v_frag, None) {
            Ok(program) => program,
            Err(msg) => panic!("could not load shader: {}", msg),
        };

        let (image, dim) = load_image_tga("./simple-font.tga");
        let image = texture::RawImage2d::from_raw_rgba_reversed(image, (dim.0 as u32, dim.1 as u32));
        let texture = texture::Texture2d::new(&self.context, image)
            .ok().expect("could not upload texture");

        let mut cursor_down  = false;
        let mut cursor_pts   = vec![];

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
                    Event::MouseInput(ElementState::Released, _)  => cursor_down = false,

                    Event::MouseMoved(x,y) => { cursor_x = x; cursor_y = y },

                    _ => (),
                }
            }


            let (wx, wy) = Engine::world_to_unit(cursor_x as f64, cursor_y as f64);
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
                .. Default::default()
            };

            let mut time_ms = 0.0;
            let time = Instant::now().duration_since(game_start_at);
            time_ms += time.as_secs() as f64 * 1000.0;
            time_ms += time.subsec_nanos() as f64 * 0.001 * 0.001;

            // F
            let tex_x = 0.125 * 6.0;
            let tex_y = 1.0 - (64.0 / 192.0);
            let cursor_uni = uniform! {
                ofs:   [wx as f32, wy as f32, 0.0f32], 
                scale: 0.15f32,
                char_pos: [tex_x as f32, tex_y as f32], // the letter A
                timer: time_ms as f32 * 0.001,
            };

            target.draw(&vbuf, &indices, &program, &cursor_uni, &tri_params)
                .ok().expect("could not blit triangle example");

            // O
            let tex_x = 0.125 * 7.0;
            let tex_y = 1.0 - (80.0 / 192.0);
            let cursor_uni = uniform! {
                ofs:   [wx as f32 + 0.25, wy as f32, 0.0f32], 
                scale: 0.15f32,
                char_pos: [tex_x as f32, tex_y as f32], // the letter A
                timer: time_ms as f32 * 0.001,
            };

            target.draw(&vbuf, &indices, &program, &cursor_uni, &tri_params)
                .ok().expect("could not blit triangle example");



            // O
            let tex_x = 0.125 * 7.0;
            let tex_y = 1.0 - (80.0 / 192.0);
            let cursor_uni = uniform! {
                ofs:   [wx as f32 + 0.5, wy as f32, 0.0f32], 
                scale: 0.15f32,
                char_pos: [tex_x as f32, tex_y as f32], // the letter A
                timer: time_ms as f32 * 0.001,
            };

            target.draw(&vbuf, &indices, &program, &cursor_uni, &tri_params)
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

    fn world_to_unit(x: f64, y: f64) -> (f64, f64) {
        let adj_x = x / 640.0;
        let adj_y = y / 360.0;
        ( (adj_x - 1.0), -(adj_y - 1.0) )
    }
}

// TODO: move to util module
// TODO: -> Result<>
//
fn load_image_tga(path_text: &str) -> (Vec<u8>, (usize,usize)) {
    use std::fs::File;
    use std::io::Read;
    use std::path::Path;

    let path = Path::new(path_text);
    let mut file = File::open(path)
        .ok().expect("could not find image file ...");

    // read file into byte buffer
    let mut buf = vec![];
    let mut ofs = 0;
    file.read_to_end(&mut buf);

    assert!(buf[0] == 0); // no id field
    assert!(buf[1] == 0); // no color map
    assert!(buf[2] == 2); // uncompressed true color
    ofs += 3; ofs += 5;   // skip header & color map
    
    let x_origin = (buf[ofs + 0] as u16) << 8 | buf[ofs + 1] as u16; ofs += 2;
    let y_origin = (buf[ofs + 0] as u16) << 8 | buf[ofs + 1] as u16; ofs += 2;

    let width  = (buf[ofs + 1] as u16) << 8 | buf[ofs + 0] as u16; ofs += 2;
    let height = (buf[ofs + 1] as u16) << 8 | buf[ofs + 0] as u16; ofs += 2;
    let depth  = buf[ofs]; ofs += 1;
    let descriptor = buf[ofs]; ofs += 1;

    println!("x origin: {}, y origin: {}", x_origin, y_origin);
    println!("bpp: {}, width: {}, height: {}", depth, width, height);
    println!("descriptor: {:08b}", descriptor);

    println!("reading image data");
    let width = width as usize;
    let height = height as usize;
    let mut size = width * height * 4;

    let mut rgba = Vec::with_capacity(size);
    for row in (0..height).rev() {
        for col in 0..width {
            let px_ofs = (row * width * 3) + col;

            rgba.push(buf[ofs + px_ofs + 0]);
            rgba.push(buf[ofs + px_ofs + 1]);
            rgba.push(buf[ofs + px_ofs + 2]);
            rgba.push(0xFF);
        }
    }

    assert!(rgba.len() == width * height * 4);
    return (rgba, (width,height));
}
