
//! PPM is very basic format for uncompressed images: it starts from few plain text
//! metadata lines separated by '\n', and right after the last one go bitmap bytes
//! see https://en.wikipedia.org/wiki/Netpbm
//!
//! this module provides functions that read/save the file binary representation

use crate::pixel_utils::{Color, Point};

pub struct PpmData {
    pub version: String,
    pub width: usize,
    pub height: usize,
    pub color_depth: usize,
    /** you don't need this, it's stored just to keep `bitmap` slice alive */
    bitmap_start: usize,
    raw_bytes: Vec<u8>,
}

impl PpmData {
    pub fn get_bitmap(&self) -> &[u8] {
        return &self.raw_bytes[self.bitmap_start ..];
    }

    pub fn get_byte_index(&self, point: &Point) -> usize {
        let pixel_index = point.y * self.width + point.x;
        return pixel_index * 3;
    }

    pub fn get_pixel(&self, point: &Point) -> Color {
        let byte_index = self.get_byte_index(point);
        let r = self.get_bitmap()[byte_index + 0];
        let g = self.get_bitmap()[byte_index + 1];
        let b = self.get_bitmap()[byte_index + 2];

        return Color { r, g, b };
    }
}

pub fn decode(raw_bytes: Vec<u8>) -> PpmData {
    let mut version = String::from("");
    let mut width = 0;
    let mut height = 0;

    let mut line_breaks_found = 0;
    let mut header_line_buffer = String::from("");
    for i in 0..raw_bytes.len() {
        if raw_bytes[i] == '\n' as u8 {
            line_breaks_found += 1;
            if line_breaks_found == 1 {
                version = header_line_buffer;
            } else if line_breaks_found == 2 {
                let mut split = header_line_buffer
                    .split(" ")
                    .map(|part| part.parse::<usize>().unwrap());
                width = split.next().unwrap();
                height = split.next().unwrap();
            } else if line_breaks_found == 3 {
                let color_depth = header_line_buffer.parse::<usize>().unwrap();
                let bitmap_start = i + 1;
                let bitmap = &raw_bytes[i + 1 ..];
                return PpmData { version, width, height, color_depth, bitmap_start, raw_bytes };
            }
            header_line_buffer = String::from("");
        } else {
            header_line_buffer.push(raw_bytes[i] as char);
        }
    }
    panic!("Ebanij vrot, missing \\n in ppm file");
}