mod detect_still_frames;
use detect_still_frames::detect_still_frames;

mod match_letter_to_font;
mod ocr_out_from_image;
mod pixel_utils;
mod ppm_format;

use ocr_out_from_image::ocr_out_from_image;

fn main() {
    //detect_still_frames().unwrap();
    ocr_out_from_image();
}
