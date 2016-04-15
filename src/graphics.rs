use sdl2;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::Renderer;


pub struct Display {
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

        // strap it to graphics subsystem
        Display { screen: renderer }
    }

    pub fn switch_buffers(&mut self) {
        self.screen.present();
    }

    pub fn clear_buffer(&mut self) {
        let _ = self.screen.clear();
    }

    pub fn fill_rect(&mut self, dst: Rect, fill: Color) {
        let _ = self.screen.set_draw_color(fill);
        let _ = self.screen.fill_rect(dst);
        let _ = self.screen.set_draw_color(Color::RGBA(0,0,0,0)); // TODO: default???
    }

}
