use std::path::Path;

#[derive(Copy, Clone)]
pub struct Vert2 {
    pub pos: [f32; 2],
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
