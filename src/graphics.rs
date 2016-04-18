use std::path::Path;

use sdl2;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::{Point, Rect};
use sdl2::render::{Renderer, RenderTarget, Texture, TextureAccess};
use sdl2_ttf::{self, Font, Sdl2TtfContext};

pub struct Display {
    _text:   Sdl2TtfContext, 
    font:   Font,  
    screen: Renderer<'static>,
}

impl Display {

    pub fn new(context: &sdl2::Sdl) -> Display {
        // boot the renderer
        let (w,h) = (1280,720);
       
        let video            = context.video().unwrap();
        let mut window_proto = video.window("koko", w as u32, h as u32);

        let current_mode     = window_proto.position_centered()
                                           .input_grabbed()
                                           .build();

        let window_context = match current_mode {
            Ok(ctx)  => ctx,
            Err(msg) => panic!(msg),
        };

        let renderer = window_context.renderer().build()
            .ok().expect("could not initialize sdl2 rendering context");

        // NOTE: must hide cursor _after_ window is built otherwise it doesn't work.
        context.mouse().show_cursor(false);
        println!("is cursor showing? {}", context.mouse().is_cursor_showing());

        // set up the font stuff
        let textmode = sdl2_ttf::init()
            .ok().expect("could not open ttf font renderer");

        let opensans = textmode.load_font(Path::new("./OpenSans-Regular.ttf"), 18)
            .ok().expect("could not load OpenSans-Regular.ttf from workingdir");

        // strap it all to the graphics subsystem
        Display {
            _text: textmode,
            font: opensans,
            screen: renderer,
        }
    }

    pub fn switch_buffers(&mut self) {
        self.screen.present();
    }

    pub fn clear_buffer(&mut self) {
        let _ = self.screen.clear();
    }

    pub fn blit_text(&mut self, buf: &str, color: Color) {
        let surface = self.font.render(&buf[..])
            .solid(color)
            .ok().expect("could not render text");

        let bounds = surface.rect();
        let texture = self.screen.create_texture_from_surface(surface)
            .ok().expect("could blit font to texture");

        let cursor_block = Rect::new(10,10, bounds.width(),bounds.height());
        self.screen.copy(&texture, None, Some(cursor_block));

    }

    pub fn copy(&mut self, texture: &Texture) {
        self.screen.copy(texture, None, None);
    }

    pub fn copy_t(&mut self, texture: &Texture, src: Rect, dst: Rect) {
        self.screen.copy(texture, Some(src), Some(dst));
    }

    pub fn get_texture(&mut self, width: u32, height: u32) -> Texture {
        self.screen.create_texture(PixelFormatEnum::ARGB8888,
                                   TextureAccess::Target,
                                   width, height)
            .ok().expect("could not open texture")
    }

    pub fn retarget(&mut self) -> RenderTarget {
        self.screen.render_target()
            .expect("renderer does not support arbitrary targets")
    }

    pub fn draw_line(&mut self, x1: i32, y1: i32, x2: i32, y2: i32, color: Color)  {
        let previous = self.screen.draw_color();
        self.screen.set_draw_color(color);

        self.screen.draw_line(Point::new(x1,y1), Point::new(x2,y2))
            .ok().expect("could not draw line");

        self.screen.set_draw_color(previous);
    }

    pub fn fill_rect(&mut self, dst: Rect, fill: Color) {
        let previous = self.screen.draw_color();
        let _ = self.screen.set_draw_color(fill);
        let _ = self.screen.fill_rect(dst);
        let _ = self.screen.set_draw_color(previous);
    }

}
