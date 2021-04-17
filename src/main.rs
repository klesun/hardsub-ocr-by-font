mod detect_still_frames;
use detect_still_frames::detect_still_frames;

mod match_letter_to_font;
mod ocr_out_from_image;
mod pixel_utils;
mod ppm_format;

use ocr_out_from_image::ocr_out_from_image;

use ab_glyph::{point, Font, FontRef, Glyph};

fn main() {
    //detect_still_frames().unwrap();
    ocr_out_from_image();

    // let font_bytes = include_bytes!("../arial.ttf");
    // let font = FontRef::try_from_slice(font_bytes).unwrap();
    // let q_glyph: Glyph = font
    //     .glyph_id('T')
    //     .with_scale_and_position(21.0, point(0.0, 0.0));
    //
    // if let Some(q) = font.outline_glyph(q_glyph) {
    //     q.draw(|x, y, c| println!("{} {} {}", x, y, c));
    // }
}
