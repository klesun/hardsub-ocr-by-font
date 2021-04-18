use crate::match_letter_to_font::{match_letter_to_font, CharMatch, make_rel_bitmap_from_image, Bounds, RelMatrix};
use crate::pixel_utils::{get_surrounding, Color, Pixel, Point};
use crate::ppm_format;
use crate::ppm_format::PpmData;
use ab_glyph::FontRef;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::io::Read;
use std::collections::BinaryHeap;
use std::cmp::{Ordering, max, min};

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

    fn set_output_pixel(&mut self, point: Point, color: Color) {
        let byte_index = self.ocr_frame.get_byte_index(&point);
        let Color { r, g, b } = color;
        self.output_bitmap[byte_index + 0] = r;
        self.output_bitmap[byte_index + 1] = g;
        self.output_bitmap[byte_index + 2] = b;
    }

    fn keep_pixel(&mut self, point: Point) {
        self.matched_points.push(point);
        self.set_output_pixel(point, self.ocr_frame.get_pixel(&point));
        self.checked_points[point.x as usize][point.y as usize] = true; // redundant for call except for first one
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
            if !self.checked_points[p.x as usize][p.y as usize] {
                self.checked_points[p.x as usize][p.y as usize] = true;
                options.push(p);
            }
        }
        return options;
    }

    fn match_as_part_of_letter(&mut self, point: Point, pixel: &Color) -> &[Point] {
        if !pixel.is_nearly_white() || self.checked_points[point.x as usize][point.y as usize] {
            return &[];
        }
        let matched_points_start = self.matched_points.len();
        self.keep_pixel(point);
        let mut pick_points = Vec::with_capacity(64);
        pick_points.push(point);

        let mut non_black_border = false;
        while pick_points.len() > 0 {
            let base_point = pick_points.pop().unwrap();
            for next_point in self.check_surrounding(base_point) {
                let pixel = self.ocr_frame.get_pixel(&next_point);
                if pixel.is_closely_black() {
                    // black outline of the letters
                } else if pixel.is_somewhat_white() || pixel.is_greyish() {
                    self.keep_pixel(next_point);
                    pick_points.push(next_point);
                } else {
                    non_black_border = true;
                }
            }
        }
        if non_black_border {
            // borders are nt black, not subs, abort
            let wrong_points = self.matched_points[matched_points_start..].to_vec();
            for wrong_point in &wrong_points {
                self.set_output_pixel(*wrong_point, Color::BLACK);
            }
            return &[];
        }

        return &self.matched_points[matched_points_start..];
    }

    fn save_file(&self, name: &str) -> std::result::Result<(), std::io::Error> {
        let mut file = File::create(format!("out/change_frames/{}.ppm", name))?;
        let ppm_header = ppm_format::make_header(
            self.ocr_frame.get_width(),
            self.ocr_frame.get_height(),
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

struct OcredChar {
    bounds: Bounds,
    char_matches: Vec<CharMatch>,
}

fn group_chars_by_line(ocred_chars: Vec<OcredChar>) -> Vec<Vec<OcredChar>> {
    let mut lines: Vec<Vec<OcredChar>> = Vec::new();
    for ocred_char in ocred_chars {
        let mut target_line: Option<usize> = None;
        for i in 0..lines.len() {
            let start_y = lines[i][0].bounds.start.y;
            if ocred_char.bounds.start.y < start_y + 20 {
                target_line = Some(i);
                break;
            }
        }
        let line_number: usize = match target_line {
            Some(i) => i,
            None => {
                let mut new_line = Vec::new();
                lines.push(new_line);
                lines.len() - 1
            },
        };
        lines[line_number].push(ocred_char);
    }
    for i in 0..lines.len() {
        lines[i].sort_by(|a, b| a.bounds.start.x.cmp(&b.bounds.start.x));
    }
    return lines;
}

const SAME_LINE_Y_THRESHOLD: u32 = 15;

fn cmp_letters_order(a: &RelMatrix, b: &RelMatrix) -> Ordering {
    return if a.bounds.start.y > b.bounds.start.y + SAME_LINE_Y_THRESHOLD as i64 {
        Ordering::Greater // a is below b
    } else if b.bounds.start.y > a.bounds.start.y + SAME_LINE_Y_THRESHOLD as i64 {
        Ordering::Less // a is above b
    } else {
        a.bounds.start.x.cmp(&b.bounds.start.x)
    }
}

fn are_parts_of_same_char(prev_item: &RelMatrix, current_item: &RelMatrix) -> bool {
    let x_overlap_start = max(prev_item.bounds.start.x, current_item.bounds.start.x);
    let x_overlap_end = min(prev_item.bounds.end.x, current_item.bounds.end.x);
    let width = min(
        prev_item.bounds.end.x - prev_item.bounds.start.x,
        current_item.bounds.end.x - current_item.bounds.start.x,
    );
    let x_overlap_px = x_overlap_end - x_overlap_start;
    // may be negative
    let x_overlap_rel = x_overlap_px as f32 / width as f32;

    let y_start_offset = (prev_item.bounds.start.y - current_item.bounds.start.y).abs();

    return x_overlap_rel > 0.5 && y_start_offset < SAME_LINE_Y_THRESHOLD as i64;
}

fn merge_char_parts(prev_item: &RelMatrix, current_item: &RelMatrix) -> RelMatrix {
    let bounds = Bounds {
        start: Point {
            x: min(
                prev_item.bounds.start.x,
                current_item.bounds.start.x,
            ),
            y: min(
                prev_item.bounds.start.y,
                current_item.bounds.start.y,
            ),
        },
        end: Point {
            x: max(
                prev_item.bounds.end.x,
                current_item.bounds.end.x,
            ),
            y: max(
                prev_item.bounds.end.y,
                current_item.bounds.end.y,
            ),
        },
    };
    let mut bitmap = vec![
        vec![0.0; bounds.get_height()];
        bounds.get_width()
    ];
    for matrix in [prev_item, current_item].iter() {
        let base = Point {
            x: matrix.bounds.start.x - bounds.start.x,
            y: matrix.bounds.start.y - bounds.start.y,
        };
        for (x, cols) in matrix.bitmap.iter().enumerate() {
            for (y, coverage) in cols.iter().enumerate() {
                bitmap[base.x as usize + x as usize][base.y as usize + y as usize] = *coverage;
            }
        }
    }
    return RelMatrix { bounds, bitmap };
}

/// dots on "i"s are extracted as separate characters, but they can be easily
/// deducted as their x position clashes with position of the stick part
fn dot_the_is(mut rel_bitmaps: Vec<RelMatrix>) -> Vec<RelMatrix> {
    let mut dotted = Vec::new();
    let mut current_item_opt = rel_bitmaps.pop();
    while current_item_opt.is_some() {
        let current_item = current_item_opt.unwrap();
        let prev_item_opt = rel_bitmaps.pop();
        if prev_item_opt.is_some() {
            let prev_item = prev_item_opt.unwrap();
            if are_parts_of_same_char(&prev_item, &current_item) {
                current_item_opt = Some(
                    merge_char_parts(&prev_item, &current_item)
                );
            } else {
                dotted.push(current_item);
                current_item_opt = Some(prev_item);
            }
        } else {
            dotted.push(current_item);
            current_item_opt = None;
        }
    }
    dotted.reverse();
    return dotted;
}

/// run through every white-ish pixel in the image, find the borders of the
/// symbol it belongs to, (like magic stick in photoshop), then compare
/// resulting bitmap to every character in the Sans-serif font
pub fn ocr_out_from_image<'a>() {
    let ocr_frame = SubsOcrFrame::load();
    let mut process = OcrProcess::init(&ocr_frame);
    let font = get_font();

    let mut rel_bitmaps: Vec<RelMatrix> = Vec::new();
    for y in 0..ocr_frame.get_height() as i64 {
        for x in 0..ocr_frame.get_width() as i64 {
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
                    let rel_bitmap = make_rel_bitmap_from_image(&letter_pixels);
                    rel_bitmaps.push(rel_bitmap);
                }
            }
        }
    }

    println!("points picked: {}", process.matched_points.len());
    process.save_file("frame15_white_only").unwrap();

    rel_bitmaps.sort_by(cmp_letters_order);
    rel_bitmaps = dot_the_is(rel_bitmaps);
    let mut ocred_chars: Vec<OcredChar> = Vec::new();
    for rel_bitmap in rel_bitmaps {
        let char_matches = match_letter_to_font(&rel_bitmap.bitmap, &font, ocred_chars.len());

        let next_best = char_matches[0];
        let comment = if next_best.match_score < 8000000 { "huj" } else { "" };
        println!("actual match #{}: {:?} {}", ocred_chars.len(), next_best, comment);

        let ocred_char: OcredChar = OcredChar {
            bounds: rel_bitmap.bounds,
            char_matches,
        };
        ocred_chars.push(ocred_char);
    }

    let lines = group_chars_by_line(ocred_chars);
    for line in lines {
        let mut end_x = line[0].bounds.end.x;
        for ocred_char in line {
            if ocred_char.bounds.start.x as i32 - end_x as i32 > 5 {
                print!(" ");
            }
            end_x = ocred_char.bounds.end.x;
            print!("{}", ocred_char.char_matches[0].char);
        }
        println!();
    }
}
