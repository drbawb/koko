use std::io::Cursor;
use std::thread;
use std::time::{Duration, Instant};

use glium::backend::glutin_backend::GlutinFacade;
use glium::draw_parameters::DrawParameters;
use glium::glutin::{ElementState, Event, VirtualKeyCode as KeyCode};
use glium::{self, texture, Surface};

use graphics_gl::Vert2;
use input::Input;
use units::V2;

static BASIC_VRT: &'static str = include_str!("shaders/basic.v.glsl");
static BASIC_FRG: &'static str = include_str!("shaders/basic.f.glsl");

static TEXT_VRT: &'static str = include_str!("shaders/text.v.glsl");
static TEXT_FRG: &'static str = include_str!("shaders/text.f.glsl");

/// On GPU Text Blitting program
pub struct TextBlitter {
    atlas:   glium::Texture2d,
    vbuf:    glium::VertexBuffer<Vert2>,
    program: glium::Program,
    indices: glium::index::NoIndices,
}

impl TextBlitter {
    pub fn new(context: &mut GlutinFacade) -> Self {
        // simple square
        let indices = glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList);
        let shape = [
            Vert2 { pos: [-  1.0,   1.0,  0.0], color: [1.0, 1.0, 1.0] },
            Vert2 { pos: [-0.875,   1.0,  0.0], color: [1.0, 1.0, 1.0] },
            Vert2 { pos: [-  1.0,  0.75,  0.0], color: [1.0, 1.0, 1.0] },

            Vert2 { pos: [-0.875,  0.75,  0.0], color: [1.0, 1.0, 1.0] },
            Vert2 { pos: [-0.875,   1.0,  0.0], color: [1.0, 1.0, 1.0] },
            Vert2 { pos: [-  1.0,  0.75,  0.0], color: [1.0, 1.0, 1.0] },
        ];

        let vbuf = glium::VertexBuffer::dynamic(context, &shape)
            .ok().expect("could not alloc vbuf");

        let program = match glium::Program::from_source(context, TEXT_VRT, TEXT_FRG, None) {
            Ok(program) => program,
            Err(msg) => panic!("could not load shader: {}", msg),
        };

        let (image, dim) = load_image_tga("./simple-font.tga");
        let image = texture::RawImage2d::from_raw_rgba(image, (dim.0 as u32, dim.1 as u32));
        let texture = texture::Texture2d::new(context, image)
            .ok().expect("could not upload texture");

        TextBlitter {
            atlas:   texture,
            vbuf:    vbuf,
            program: program,
            indices: indices,
        }
    }

    pub fn draw(&self, text: &str, target: &mut glium::Frame) {
        let mapping: Vec<(f32,f32)> = text.chars()
            .map(|cp| TextBlitter::ascii_to_ofs(cp))
            .collect();

        let mut ofs_x = 0.0;
        for &(char_x, char_y) in mapping.iter() {
            let char_uni = uniform! {
                atlas: &self.atlas,
                w_ofs: [ofs_x, 0.0, 0.0f32],
                c_ofs: [char_x, -char_y],
                scale: 0.25f32,
            };

            // (one character in texture space) * (screen space)
            ofs_x += (16.0 / 256.0) * 0.5;

            target.draw(&self.vbuf, &self.indices, &self.program, &char_uni, &DrawParameters {
                .. Default::default()
            }).ok().expect("could not blit char example");
        }
    }

    fn ascii_to_ofs(cp: char) -> (f32, f32) {
        use std::ascii::AsciiExt;
        
        assert!(cp.is_ascii());
        let sprite_ofs = match cp {
            'A'...'P' => (cp as u8 - 'A' as u8,      0),
            'Q'...'Z' => (cp as u8 - 'Q' as u8,      1),
            'a'...'f' => (cp as u8 - 'a' as u8 + 10, 1),
            'g'...'v' => (cp as u8 - 'g' as u8,      2),
            'w'...'z' => (cp as u8 - 'w' as u8,      3),

            '1'...'9'  => ((cp as u8 - '1' as u8)  + 4, 3),
            '0'        => (('9' as u8 - '0' as u8) + 4, 3),

            ' ' => (0, 7),

            _ => panic!("unhandled character in spritemap"),
        };

        let char_x: f32 =  sprite_ofs.0 as f32 * (0.125 / 2.0);
        let char_y: f32 =  sprite_ofs.1 as f32 * (0.125 / 1.0);

        (char_x, char_y)
    }
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
        let shape = [
            // face 1
            Vert2 { pos: [ 1.0,  0.0, 0.0], color: [1.0, 0.0, 0.0] },
            Vert2 { pos: [ 0.0,  0.0, 0.0], color: [1.0, 0.0, 0.0] },
            Vert2 { pos: [ 0.0, -1.0, 0.0], color: [1.0, 0.0, 0.0] },
        ];

        let indices = glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList);
        let vbuf = glium::VertexBuffer::dynamic(&self.context, &shape)
            .ok().expect("could not alloc vbuf");

        let program = match glium::Program::from_source(&self.context, BASIC_VRT, BASIC_FRG, None) {
            Ok(program) => program,
            Err(msg) => panic!("could not load shader: {}", msg),
        };

        let mut cursor_down  = false;
        let mut cursor_pts   = vec![];

        let mut cursor_x = 0;
        let mut cursor_y = 0;

        let text_blitter = TextBlitter::new(&mut self.context);
        let mut text_count = 0;
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

            // cursor
            let cursor_uni = uniform! {
                ofs:   [wx as f32, wy as f32, 0.0f32], 
                scale: 0.15f32,
                timer: time_ms as f32 * 0.001,
            };


            frame_count += 1;
            if frame_count > 10 {
                frame_count = 0;
                text_count = (text_count + 1) % 0xFF;
            }

            let text_out = format!("hi 0x{:02X}", text_count);
            text_blitter.draw(&text_out[..], &mut target);

            target.draw(&vbuf, &indices, &program, &cursor_uni, &tri_params)
                .ok().expect("could not blit cursor example");

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
        .ok().expect(&format!("could not find image file @ {:?}", path)[..]);

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
    let width  = width as usize;
    let height = height as usize;
    let pitch  = (depth / 8) as usize;
    let size   = width * height * pitch;
    assert!(pitch == 4);

    let mut rgba = Vec::with_capacity(size);
    for row in 0..height {
        for col in 0..width {
            let px_ofs = (row * width * pitch) + (col * pitch);
            if buf[ofs + px_ofs + 3] == 0 {
                rgba.extend_from_slice(&[0x00, 0x00, 0x00, 0xFF]);
            } else {
                rgba.push(buf[ofs + px_ofs + 2]);
                rgba.push(buf[ofs + px_ofs + 1]);
                rgba.push(buf[ofs + px_ofs + 0]);
                rgba.push(buf[ofs + px_ofs + 3]);
            }
        }
    }

    assert!(rgba.len() == width * height * 4);
    return (rgba, (width,height));
}
