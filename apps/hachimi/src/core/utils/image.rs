//! Minimal PNG loading limited to RGBA8 images.

use std::{fs::File, path::Path};

// Intentionally dumb png loader implementation that only loads RGBA8 images
pub fn load_rgba_png<R: std::io::Read>(r: R) -> Option<(Vec<u8>, png::OutputInfo)> {
    let mut reader = png::Decoder::new(r).read_info().ok()?;
    let mut img_data = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut img_data).ok()?;
    if info.color_type != png::ColorType::Rgba || info.bit_depth != png::BitDepth::Eight {
        return None;
    }
    Some((img_data, info))
}

pub fn load_rgba_png_file<P: AsRef<Path>>(path: P) -> Option<(Vec<u8>, png::OutputInfo)> {
    load_rgba_png(File::open(path).ok()?)
}
