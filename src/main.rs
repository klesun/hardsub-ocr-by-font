mod detect_still_frames;
use detect_still_frames::detect_still_frames;

mod ocr_out_from_image;
mod ppm_format;
mod pixel_utils;

use ocr_out_from_image::ocr_out_from_image;

fn main() {
    //detect_still_frames().unwrap();
    ocr_out_from_image();
}