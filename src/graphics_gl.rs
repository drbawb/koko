use glium::backend::glutin_backend::GlutinFacade;
use glium::draw_parameters::DrawParameters;
use glium::{self, texture, Surface};

use util;

static TEXT_VRT: &'static str = include_str!("shaders/text.v.glsl");
static TEXT_FRG: &'static str = include_str!("shaders/text.f.glsl");

#[derive(Copy, Clone)]
pub struct Vert2 {
    pub pos:   [f32; 3],
    pub color: [f32; 3],
}

implement_vertex!(Vert2, pos, color);

pub struct Display {}

impl Display {
    /*
    pub fn switch_buffers(&mut self) {}
    pub fn clear_buffer(&mut self) {}
    pub fn blit_text(&mut self, buf: &str, color: Color) {}
    pub fn copy(&mut self, texture: &Texture) {}
    pub fn copy_t(&mut self, texture: &Texture, src: Rect, dst: Rect) {}
    pub fn retarget(&mut self) -> RenderTarget {}
    pub fn draw_line(&mut self, x1: i32, y1: i32, x2: i32, y2: i32, color: Color)  {}
    pub fn fill_rect(&mut self, dst: Rect, fill: Color) {}
    pub fn read_pixels(&mut self) -> Vec<u8> {}
    */
}

/// On GPU Text Blitting program
pub struct TextBlitter {
    atlas_array: texture::texture2d_array::Texture2dArray,
    vbuf:    glium::VertexBuffer<Vert2>,
    program: glium::Program,
    indices: glium::index::NoIndices,
}

impl TextBlitter {
    /// Borrows an OpenGL Context to upload a font-atlas and text rendering program
    /// into GPU memory.
    ///
    /// This then returns a text-blitting helper which can be used to quickly draw
    /// strings of ASCII characters to the screen.
    ///
    /// NOTE: currently only prints a subset of ASCII
    /// NOTE: requires `simple-font.tga` in working directory
    /// NOTE: will totally explode if you swap out other fonts
    pub fn new(context: &mut GlutinFacade) -> Self {
        // simple square
        let indices = glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList);
        let shape = [
            Vert2 { pos: [-  1.0,  1.0,  0.0], color: [1.0, 1.0, 1.0] },
            Vert2 { pos: [-0.875,  1.0,  0.0], color: [1.0, 1.0, 1.0] },
            Vert2 { pos: [-  1.0, -1.0,  0.0], color: [1.0, 1.0, 1.0] },
 
            Vert2 { pos: [-  1.0, -1.0,  0.0], color: [1.0, 1.0, 1.0] },
            Vert2 { pos: [-0.875, -1.0,  0.0], color: [1.0, 1.0, 1.0] },
            Vert2 { pos: [-0.875,  1.0,  0.0], color: [1.0, 1.0, 1.0] },
        ];

        let vbuf = glium::VertexBuffer::new(context, &shape)
            .ok().expect("could not alloc vbuf");

        let program = match glium::Program::from_source(context, TEXT_VRT, TEXT_FRG, None) {
            Ok(program) => program,
            Err(msg) => panic!("could not load shader: {}", msg),
        };

        // TODO: read image dimension and dynamically size the atlas instead
        //       of hardcoding it to the 8 page font sheet.
        let (image, _dim) = util::load_image_tga("./simple-font.tga");
        
        let mut sprite_rows = vec![];
        for i in (0..8).rev() {
            let stride = 256 * 4 * 16;
            let start  = i * stride;
            let end    = start + stride;

            println!("{} => {}", start,end);

            let row: Vec<u8> = (&image[start..end]).iter().map(|byte| *byte).collect();
            let fuck = texture::RawImage2d::from_raw_rgba(row, (256,16));
            sprite_rows.push(fuck);
        }

        let atlas_array = texture::texture2d_array::Texture2dArray::new(context,sprite_rows)
            .ok().expect("could not uplaod texture array");

        TextBlitter {
            atlas_array: atlas_array,
            vbuf:    vbuf,
            program: program,
            indices: indices,
        }
    }

    pub fn draw(&self, text: &str, font_size: f32, ofs: (f32, f32), target: &mut glium::Frame) {
        let mapping: Vec<(f32,f32)> = text.chars()
            .map(|cp| TextBlitter::ascii_to_ofs(cp))
            .collect();

        // NOTE: there are two translation steps
        //
        // c_pos is applied before we scale the text so that it's computation
        // can be done w/o taking current text scaling into account
        //
        // w_pos is applied after we scale the thing so we can just move
        // it around as a single unit of text
        //
        // so essentially a character is:
        //   1. translated so the upper left corner of the character is at the origin
        //      (instead of the origin being the center of the sprite sheet)
        //
        //   2. translated over some number of characters (character offset * (16.0 / 128.0))
        //   3. scaled to the user's preferred text size
        //   4. translated to where the user wanted it on the screen (by upper left corner)
        //


        let mut ofs_x = 0.0;
        for &(char_x, char_y) in mapping.iter() {
            let char_uni = uniform! {
                atlas_array: self.atlas_array.sampled(),
                c_pos: [ofs_x, 0.0, 0.0f32],
                c_ofs: [char_x, char_y],
                w_ofs: [ofs.0, ofs.1, 0.0f32],
                scale: font_size,
            };

            ofs_x += 16.0 / 128.0; // move forward one character in textspace

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

        let char_x: f32 =  sprite_ofs.0 as f32 * (1.0 / 16.0); // index into the page
        let char_y: f32 =  sprite_ofs.1 as f32;                 // atlas page number

        (char_x, char_y)
    }
}
