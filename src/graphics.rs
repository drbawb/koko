//use glium::backend::glutin_backend::GlutinFacade;
use glium::draw_parameters::DrawParameters;
use glium::{self, backend::Facade, texture, Surface};
use glium::uniforms::{MinifySamplerFilter, MagnifySamplerFilter, SamplerWrapFunction};

use util;

static TEXT_VRT: &'static str = include_str!("shaders/text.v.glsl");
static TEXT_FRG: &'static str = include_str!("shaders/text.f.glsl");

#[derive(Copy, Clone, Debug)]
pub struct Vert2 {
    pub pos:   [f32; 3],
    pub color: [f32; 3],
}

implement_vertex!(Vert2, pos, color);

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
    pub fn new<F: Facade>(context: &mut F) -> Self {
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
            .expect("could not alloc vbuf");

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

            let row: Vec<u8> = (&image[start..end]).iter().cloned().collect();
            let fuck = texture::RawImage2d::from_raw_rgba(row, (256,16));
            sprite_rows.push(fuck);
        }

        let atlas_array = texture::texture2d_array::Texture2dArray::new(context,sprite_rows)
            .expect("could not uplaod texture array");

        TextBlitter {
            atlas_array: atlas_array,
            vbuf:    vbuf,
            program: program,
            indices: indices,
        }
    }

    pub fn draw(&self, text: &str, font_size: f32, ofs: (f32, f32), target: &mut glium::Frame) {
        let mapping: Vec<(f32,f32)> = text.chars()
            .map(TextBlitter::ascii_to_ofs)
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
        for &(char_x, char_y) in &mapping {
            let char_uni = uniform! {
                atlas_array: self.atlas_array.sampled()
                    .minify_filter(MinifySamplerFilter::Nearest)
                    .magnify_filter(MagnifySamplerFilter::Nearest)
                    .wrap_function(SamplerWrapFunction::Clamp),

                c_pos: [ofs_x, 0.0, 0.0f32],
                c_ofs: [char_x, char_y],
                w_ofs: [ofs.0, ofs.1, 0.0f32],
                scale: font_size,
            };

            ofs_x += 16.0 / 128.0; // move forward one character in textspace

            target.draw(&self.vbuf, &self.indices, &self.program, &char_uni, &DrawParameters {
                .. Default::default()
            }).expect("could not blit character");
        }
    }

    fn ascii_to_ofs(cp: char) -> (f32, f32) {
        assert!(cp.is_ascii());
        let sprite_ofs = match cp {
            'A'...'P' => (cp as u32 - 'A' as u32,      0),
            'Q'...'Z' => (cp as u32 - 'Q' as u32,      1),
            'a'...'f' => (cp as u32 - 'a' as u32 + 10, 1),
            'g'...'v' => (cp as u32 - 'g' as u32,      2),
            'w'...'z' => (cp as u32 - 'w' as u32,      3),

            '1'...'9'  => ((cp  as u32 - '1' as u32) + 4, 3),
            '0'        => (('9' as u32 - '0' as u32) + 4, 3),

            ' ' => ( 0, 7),
            '-' => (14, 3),
            ',' => ( 0, 4),
            '.' => ( 1, 4),
            '@' => ( 5, 4),
            '#' => ( 6, 4),
            '(' => (12, 4),
            ')' => (13, 4),
            '=' => (15, 4),
            ':' => ( 2, 5),
            '[' => ( 8, 5),
            ']' => ( 9, 5),
            '<' => (14, 5),
            '>' => (15, 5),

            token => panic!("unhandled character in spritemap: {}", token),
        };

        let char_x: f32 =  sprite_ofs.0 as f32 * (1.0 / 16.0); // index into the page
        let char_y: f32 =  sprite_ofs.1 as f32;                 // atlas page number

        (char_x, char_y)
    }
}
