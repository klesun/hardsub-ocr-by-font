//! this module takes an array of present pixels (bitmap with alpha conceptually), it then
//! matches the image to every english letter in Sans-serif font and returns the best match
//! an OCR if you will

use crate::pixel_utils::{Color, Pixel, Point};
use crate::ppm_format;
use ab_glyph::{point, Font, FontRef, Glyph};
use std::cmp::{max, min, Ordering};
use std::collections::BinaryHeap;
use std::fs::File;
use std::io::Write;
use crate::rel_matrix::{make_rel_bitmap, RelMatrix, PixelCoverage};
use crate::font_data::{CHAR_OPTIONS, FontData};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CharMatch {
    pub char: String,
    pub match_score: i64,
    pub font_area_score: i64,
    pub font_shift_index: usize,
}

impl Ord for CharMatch {
    fn cmp(&self, other: &Self) -> Ordering {
        return self.match_score.cmp(&other.match_score);
    }
}

impl PartialOrd for CharMatch {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        return Some(self.cmp(other));
    }
}

fn draw_debug(img_bitmap: &[Vec<f32>], font_bitmap: &[Vec<f32>], suffix: String) {
    let img_width = img_bitmap.len();
    let img_height = img_bitmap[0].len();
    let font_width = font_bitmap.len();
    let font_height = font_bitmap[0].len();

    let width = img_width + font_width;
    let first_row_height = max(img_height, font_height);
    let height = first_row_height + font_height;
    let ppm_header = ppm_format::make_header(width, height);

    let mut file = File::create(format!("out/ocr_debug/huj{}.ppm", suffix)).unwrap();
    file.write_all(ppm_header.as_bytes()).unwrap();
    for y in 0..height {
        for x in 0..width {
            let mut color = Color::BLACK;
            if y < first_row_height {
                if x < img_width {
                    if y < img_height {
                        let c = img_bitmap[x][y];
                        color = coverage_to_color(c);
                    }
                } else {
                    let rel_x = x - img_width;
                    if y < font_height {
                        let c = font_bitmap[rel_x][y];
                        color = coverage_to_color(c);
                    }
                }
            } else {
                if x < font_width {
                    let rel_y = y - first_row_height;
                    let c = font_bitmap[x][rel_y];
                    color = coverage_to_color(c);
                }
            }
            file.write(&color.to_vector());
        }
    }
}

/// if same pixel on both font bitmap and image bitmap is of completely same lightness, result is 1.0
/// if pixel is completely black on one and completely white on the other, result is 0.0
fn get_pixel_score(x: usize, y: usize, c: f32, img_bitmap: &[Vec<f32>]) -> f32 {
    let mut overbound =
        max(0, x as i64 - img_bitmap.len() as i64 + 1) +
        max(0, y as i64 - img_bitmap[0].len() as i64 + 1);
    return if overbound <= 0 {
        let img_coverage = img_bitmap[x as usize][y as usize];
        let difference = (c - img_coverage).abs();
        1.0 - difference
    } else {
        -c * (overbound * overbound) as f32 / 100.0
    };
}

struct BitmapCompare {
    hardsub_area_score: f32,
    font_area_score: f32,
}

fn is_empty_row(y: usize, check_x: usize, img_bitmap: &[Vec<f32>]) -> bool {
    for x in 0..check_x {
        if img_bitmap[x][y] > 0.001 {
            return false;
        }
    }
    return true;
}

fn trim_y<'a>(check_x: usize, img_bitmap: &[Vec<f32>]) -> Option<Vec<Vec<f32>>> {
    if check_x >= img_bitmap.len() {
        return None;
    }
    let mut trim_y = 0;
    if check_x < img_bitmap.len() {
        for y in 0..img_bitmap[0].len() {
            if is_empty_row(y, check_x, img_bitmap) {
                trim_y = y + 1;
            } else {
                break;
            }
        }
    }
    if trim_y == 0 {
        return None;
    }
    let mut trimmed = vec![
        vec![0.0; img_bitmap[0].len() - trim_y];
        img_bitmap.len()
    ];
    for x in 0..img_bitmap.len() {
        for y in trim_y..img_bitmap[0].len() {
            trimmed[x][y - trim_y] = img_bitmap[x][y];
        }
    }
    return Some(trimmed);
}

fn compare_bitmaps(font_bitmap: &[Vec<f32>], img_bitmap_arg: &[Vec<f32>]) -> BitmapCompare {
    let trimmed_opt = trim_y(font_bitmap.len(), img_bitmap_arg);
    let mut zhopa;
    let img_bitmap = if trimmed_opt.is_some() {
        zhopa = trimmed_opt.unwrap();
        &zhopa
    } else {
        img_bitmap_arg
    };

    let hardsub_area = (img_bitmap.len() * img_bitmap[0].len()) as f32;
    let font_area = (font_bitmap.len() * font_bitmap[0].len()) as f32;

    let img_shift_options = [
        Point { x:  0, y:  0 },
        Point { x:  1, y:  0 },
        Point { x:  1, y:  1 },
        Point { x:  0, y:  1 },
        Point { x: -1, y:  1 },
        Point { x: -1, y:  0 },
        Point { x:  0, y: -1 },
        Point { x:  1, y: -1 },
    ];
    // TODO: try to trim whitespace from top during partial match
    let mut best_score = 0f32;
    for shift in &img_shift_options {
        let mut score = 0f32;
        for (x, font_cells) in font_bitmap.iter().enumerate() {
            for y in 0..max(font_cells.len(), img_bitmap[0].len()) {
                let c = *font_cells.get(y).unwrap_or(&0.0);
                let shifted_x = x as i64 + shift.x;
                let shifted_y = y as i64 + shift.y;
                if shifted_x >= 0 && shifted_y >= 0 {
                    score += get_pixel_score(shifted_x as usize, shifted_y as usize, c, img_bitmap);
                }
            }
        }
        best_score = best_score.max(score);
    }
    return BitmapCompare {
        hardsub_area_score: best_score / hardsub_area,
        font_area_score: best_score / font_area,
    };
}

fn match_bitmap_to_char(
    img_bitmap: &[Vec<f32>],
    char: char,
    font_data: &FontData,
    is_expected: bool,
    suffix: String,
) -> CharMatch {
    let mut matches = BinaryHeap::new();
    for (i, font_matrix) in font_data.get_bitmaps(char).iter().enumerate() {
        let BitmapCompare {
            hardsub_area_score, font_area_score,
        } = compare_bitmaps(&font_matrix.bitmap, img_bitmap);

        let match_option = CharMatch {
            char: char.to_string(),
            match_score: (10000000.0 * hardsub_area_score) as i64,
            font_area_score: (10000000.0 * font_area_score) as i64,
            font_shift_index: i,
        };

        if is_expected {
            // draw_debug(
            //     img_bitmap, &font_matrix.bitmap,
            //     format!(
            //         "_{}_{}_{}_{}", suffix, char, i,
            //         match_option.match_score / 100000
            //     ),
            // );
        }
        matches.push(match_option);
    }
    return matches.pop().unwrap();
}

fn color_to_coverage(color: &Color) -> f32 {
    return (color.r as f32 + color.g as f32 + color.b as f32) / (255.0 * 3.0);
}

fn coverage_to_color(coverage: f32) -> Color {
    let lightness = (255.0 * coverage) as u8;
    return Color { r: lightness, g: lightness, b: lightness };
}

pub fn make_rel_bitmap_from_image(letter_pixels: &[Pixel]) -> RelMatrix {
    return make_rel_bitmap(
        letter_pixels
            .iter()
            .map(|pixel| PixelCoverage {
                x: pixel.point.x as u32,
                y: pixel.point.y as u32,
                c: color_to_coverage(&pixel.color),
            })
            .collect(),
    );
}

pub fn match_letter_to_font(
    rel_bitmap: &Vec<Vec<f32>>,
    font_data: &FontData,
    index: usize,
) -> Vec<CharMatch> {
    let expected = [
        'T', 'h', 'e', 'r', 'e', 'a', 'r', 'e', 'm','a','n','y','t','h','e','o','r','i','e','s','a','b','o','u','t',
        't','h','e','d','i','v','i','s','i','o','n','b','e','t','w','e','e','n','L','a','t','e','M','o','d','e','r','n',
        'P','e','r','i','o','d','a','n','d','C','o','n','t','e','m','p','o','r','a','r','H','i','s','t','o','r',
        'O','n','e','t','h','e','o','r','y','m','a','r','k','s','t','h','e','b','e','g','i','n','n','i','n','g',
        'o','f','t','h','e','C','o','n','t','e','m','p','o','r','a','r','H','i','s','t','o','r','w','i','t','h',
        't','h','e','b','i','r','h','o','f','t','h','e','n','e','w','m','o','d','e','l','t','s','u','r','u','g','i',
    ];

    let mut matches = BinaryHeap::new();
    for char in &CHAR_OPTIONS {
        let is_expected = index < expected.len() && expected[index] == *char;
        let matched = match_bitmap_to_char(rel_bitmap, *char, font_data, is_expected, index.to_string());
        if is_expected {
            println!("expect match #{}: {:?}", index, matched);
        }
        matches.push(matched);
    }
    let mut extra_matches = Vec::new();
    // you also think it should be for arbitrary number of letters?
    if matches.peek().unwrap().match_score < 8000000 {
        for bad_match in &matches {
            if bad_match.font_area_score >= 8000000 {
                println!("ololo index {} partial match {}", index, bad_match.char);
                let options = font_data.get_bitmaps(bad_match.char.chars().next().unwrap());
                let cutout_x = options[bad_match.font_shift_index].bitmap.len();
                if cutout_x < rel_bitmap.len() {
                    let hardsub_proportion = cutout_x as f32 / rel_bitmap.len() as f32;
                    let remaining_bitmap = &rel_bitmap[cutout_x ..];
                    for char in &CHAR_OPTIONS {
                        let remainder_match = match_bitmap_to_char(remaining_bitmap, *char, font_data, false, format!("{}.{}", index, bad_match.char));
                        extra_matches.push(CharMatch {
                            char: format!("{}{}", bad_match.char, remainder_match.char),
                            font_area_score: (
                                bad_match.font_area_score as f32 * hardsub_proportion +
                                remainder_match.font_area_score as f32 * (1.0 - hardsub_proportion)
                            ) as i64,
                            match_score: bad_match.match_score + (remainder_match.match_score as f32 * (1.0 - hardsub_proportion)) as i64,
                            font_shift_index: 100, // idgaf
                        });
                    }
                }
            }
        }
    }
    for extra in extra_matches {
        matches.push(extra);
    }

    return matches.into_vec();
}
