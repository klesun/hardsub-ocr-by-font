use crate::match_letter_to_font::match_letter_to_font;
use crate::pixel_utils::{get_surrounding, Color, Pixel, Point};
use crate::ppm_format;
use crate::ppm_format::PpmData;
use ab_glyph::{point, Font, FontRef, Glyph};
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::io::Read;

fn read_file(suffix: &str) -> PpmData {
    let frame_name = "frame15";
    let path = format!("out/change_frames/{}{}.ppm", frame_name, suffix);
    let metadata = fs::metadata(&path).unwrap();
    let mut file = File::open(path).unwrap();
    let file_size = metadata.len() as usize;
    let mut raw_bytes = vec![0 as u8; file_size];
    file.read(&mut raw_bytes).unwrap();

    return ppm_format::decode(raw_bytes);
}

struct SubsOcrFrame {
    full_ppm: PpmData,
    text_ppm: PpmData,
}

impl SubsOcrFrame {
    fn get_width(&self) -> usize {
        return self.full_ppm.width;
    }

    fn get_height(&self) -> usize {
        return self.full_ppm.height;
    }

    fn get_bitmap_length(&self) -> usize {
        // not true for non-255 color formats, but nah...
        return self.full_ppm.width * self.full_ppm.height * 3;
    }

    fn get_byte_index(&self, point: &Point) -> usize {
        return self.full_ppm.get_byte_index(&point);
    }

    fn get_pixel(&self, point: &Point) -> Color {
        return self.full_ppm.get_pixel(&point);
    }

    fn load() -> SubsOcrFrame {
        // TODO; wrong naming, it's not old/new, it's full/text-only
        let full_ppm = read_file("_old");
        let text_ppm = read_file("_new");

        let ocr_frame = SubsOcrFrame { full_ppm, text_ppm };
        if ocr_frame.get_bitmap_length() != ocr_frame.full_ppm.get_bitmap().len() {
            panic!(
                "Ololo pizdec unsupported pixel format {} -> {} != {}",
                ocr_frame.full_ppm.color_depth,
                ocr_frame.get_bitmap_length(),
                ocr_frame.full_ppm.get_bitmap().len()
            );
        }

        return ocr_frame;
    }
}

struct OcrProcess<'a> {
    ocr_frame: &'a SubsOcrFrame,
    /// x-to-y-to-bool
    checked_points: Vec<Vec<bool>>,
    matched_points: Vec<Point>,
    output_bitmap: Vec<u8>,
}

impl OcrProcess<'_> {
    fn init(ocr_frame: &SubsOcrFrame) -> OcrProcess {
        let checked_points = vec![vec![false; ocr_frame.get_height()]; ocr_frame.get_width()];
        let matched_points = Vec::new();
        let output_bitmap = vec![0; ocr_frame.full_ppm.get_bitmap().len()];

        return OcrProcess {
            ocr_frame,
            checked_points,
            matched_points,
            output_bitmap,
        };
    }

    fn keep_pixel(&mut self, point: Point) {
        self.matched_points.push(point);

        let byte_index = self.ocr_frame.get_byte_index(&point);
        let Color { r, g, b } = self.ocr_frame.get_pixel(&point);
        self.output_bitmap[byte_index + 0] = r;
        self.output_bitmap[byte_index + 1] = g;
        self.output_bitmap[byte_index + 2] = b;

        self.checked_points[point.x][point.y] = true; // redundant for call except for first one
    }

    fn check_surrounding(&mut self, base_point: Point) -> Vec<Point> {
        // let options: Vec<Point> = get_surrounding(&base_point, self.ocr_frame.get_width(), self.ocr_frame.get_height())
        //     .filter(move |p| !self.checked_points[p.x][p.y])
        //     .collect();
        let mut options = Vec::new();
        for p in get_surrounding(
            &base_point,
            self.ocr_frame.get_width(),
            self.ocr_frame.get_height(),
        ) {
            if !self.checked_points[p.x][p.y] {
                self.checked_points[p.x][p.y] = true;
                options.push(p);
            }
        }
        return options;
    }

    fn match_as_part_of_letter(&mut self, point: Point, pixel: &Color) -> &[Point] {
        if !pixel.is_nearly_white() {
            return &[];
        }
        let matched_points_start = self.matched_points.len();
        self.keep_pixel(point);
        let mut pick_points = Vec::with_capacity(64);
        pick_points.push(point);

        while pick_points.len() > 0 {
            let base_point = pick_points.pop().unwrap();
            for next_point in self.check_surrounding(base_point) {
                if self.ocr_frame.get_pixel(&next_point).is_somewhat_white() {
                    self.keep_pixel(next_point);
                    pick_points.push(next_point);
                }
            }
        }

        return &self.matched_points[matched_points_start..];
    }

    fn save_file(&self, name: &str) -> std::result::Result<(), std::io::Error> {
        let mut file = File::create(format!("out/change_frames/{}.ppm", name))?;
        let ppm_header = format!(
            "P6\n{} {}\n255\n",
            self.ocr_frame.get_width(),
            self.ocr_frame.get_height()
        );
        file.write_all(ppm_header.as_bytes())?;
        file.write_all(&self.output_bitmap)?;
        Ok(())
    }
}

fn get_font<'a>() -> FontRef<'a> {
    let font_bytes = include_bytes!("../arial.ttf");
    return FontRef::try_from_slice(font_bytes).unwrap();
}

/// run through every white-ish pixel in the image, find the borders of the
/// symbol it belongs to, (like magic stick in photoshop), then compare
/// resulting bitmap to every character in the Sans-serif font
pub fn ocr_out_from_image() {
    let ocr_frame = SubsOcrFrame::load();
    let mut process = OcrProcess::init(&ocr_frame);
    let font = get_font();

    for y in 0..ocr_frame.get_height() {
        for x in 0..ocr_frame.get_width() {
            let point = Point { x, y };
            let pixel = ocr_frame.text_ppm.get_pixel(&point);
            if pixel != Color::BLACK {
                // this pixel had a significant change in
                // the frame, likely a part of the hardsub
                let letter_pixels: Vec<Pixel> = process
                    .match_as_part_of_letter(point, &pixel)
                    .iter()
                    .map(|pt| Pixel {
                        color: ocr_frame.get_pixel(pt),
                        point: *pt,
                    })
                    .collect();
                if letter_pixels.len() > 0 {
                    match_letter_to_font(&letter_pixels, &font);
                    panic!("v pizdu");
                }
            }
        }
    }

    println!("points picked: {}", process.matched_points.len());
    process.save_file("frame15_white_only").unwrap();
}
