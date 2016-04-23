use std::fs::File;
use std::io::Read;
use std::path::Path;


// TODO: -> Result<>
pub fn load_image_tga(path_text: &str) -> (Vec<u8>, (usize,usize)) {
    let path = Path::new(path_text);
    let mut file = File::open(path)
        .expect(&format!("could not find image file @ {:?}", path)[..]);

    // read file into byte buffer
    let mut buf = vec![];
    let mut ofs = 0;
    file.read_to_end(&mut buf)
        .expect("i/o error reading text sprite sheet");

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
    (rgba, (width,height))
}
