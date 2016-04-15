use std::path::Path;
use std::time::Duration;

use sdl2;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::Rect;
use sdl2::render::{self, Renderer, Texture, TextureAccess};
use sdl2_ttf::{self, Font, Sdl2TtfContext};

use engine;

pub struct Display {
    text:   Sdl2TtfContext, 
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

        let renderer = window_context.renderer()
            .build()
            .ok()
            .expect("could not initialize sdl2 rendering context");

        // NOTE: must hide cursor _after_ window is built otherwise it doesn't work.
        context.mouse().show_cursor(false);
        println!("is cursor showing? {}", context.mouse().is_cursor_showing());

        let textmode = sdl2_ttf::init()
            .ok().expect("could not open ttf font renderer");

        let opensans = textmode.load_font(Path::new("./OpenSans-Regular.ttf"), 18)
            .ok().expect("could not load OpenSans-Regular.ttf from workingdir");

        // strap it to graphics subsystem
        Display {
            text: textmode,
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

    // TODO: debug only
    pub fn blit_fps(&mut self, time: Duration) {
        let mut time_ms = time.as_secs() * 1000;        // -> millis
        time_ms += time.subsec_nanos() as u64 / (1000 * 1000); // /> micros /> millis
        
        let buf = format!("{}ms", time_ms);
        let surface = self.font.render(&buf[..])
            .solid(Color::RGB(255,255,0))
            .ok().expect("could not render fps");

        let bounds = surface.rect();
        let texture = self.screen.create_texture_from_surface(surface)
            .ok().expect("could blit font to texture");

        self.screen.copy(&texture, None, Some(Rect::new(10, 10, bounds.width(), bounds.height())));

    }

    pub fn copy(&mut self, texture: &Texture) {
        self.screen.copy(texture, None, None);
    }

    pub fn get_texture(&mut self, width: u32, height: u32) -> Texture {
        self.screen.create_texture(PixelFormatEnum::ARGB8888,
                                   TextureAccess::Streaming,
                                   width, height)
            .ok().expect("could not open streaming texture")
    }

    pub fn fill_rect(&mut self, dst: Rect, fill: Color) {
        let _ = self.screen.set_draw_color(fill);
        let _ = self.screen.fill_rect(dst);
        let _ = self.screen.set_draw_color(Color::RGBA(0,0,0,0)); // TODO: default???
    }

}
