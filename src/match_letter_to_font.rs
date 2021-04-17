//! this module takes an array of present pixels (bitmap with alpha conceptually), it then
//! matches the image to every english letter in Sans-serif font and returns the best match
//! an OCR if you will

use crate::pixel_utils::{Color, Pixel, Point};
use ab_glyph::{point, Font, FontRef, Glyph};
use std::cmp::{max, min, Ordering};
use std::collections::BinaryHeap;
use crate::ppm_format;
use std::fs::File;
use std::io::Write;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct CharMatch {
    pub char: char,
    pub match_score: i64,
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

struct PixelCoverage {
    x: u32,
    y: u32,
    c: f32,
}

fn draw_debug(img_bitmap: &Vec<Vec<f32>>, font_bitmap: &Vec<Vec<f32>>, char: char, index: usize) {
    let img_width = img_bitmap.len();
    let img_height = img_bitmap[0].len();
    let font_width = font_bitmap.len();
    let font_height = font_bitmap[0].len();

    let width = img_width + font_width;
    let height = max(img_height, font_height);
    let ppm_header = ppm_format::make_header(width, height);

    let mut file = File::create(format!("out/ocr_debug/huj_{}_{}.ppm", index, char)).unwrap();
    file.write_all(ppm_header.as_bytes()).unwrap();
    for y in 0..height {
        for x in 0..width {
            let mut color = Color::BLACK;
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
            file.write(&color.to_vector());
        }
    }
}

fn match_bitmap_to_char(img_bitmap: &Vec<Vec<f32>>, char: char, font: &FontRef, is_expected: bool, index: usize) -> CharMatch {
    let mut matches = BinaryHeap::new();
    let shift_options = [
        point(0.0, 0.0),
        point(0.5, 0.0),
        point(0.0, 0.5),
        point(0.5, 0.5),
    ];
    for shift in &shift_options {
        let glyph: Glyph = font
            .glyph_id(char)
            .with_scale_and_position(24.0, *shift);

        let outlined = font.outline_glyph(glyph).unwrap();

        let mut coverages = Vec::new();
        outlined.draw(|x, y, c| {
            coverages.push(PixelCoverage { x, y, c });
        });
        let font_matrix = make_rel_bitmap(coverages);
        if is_expected {
            draw_debug(img_bitmap, &font_matrix.bitmap, char, index);
        }

        let mut matched_score = 0f32;
        let max_possible_score = (img_bitmap.len() * img_bitmap[0].len()) as f32;

        for (x, cols) in font_matrix.bitmap.iter().enumerate() {
            for (y, c) in cols.iter().enumerate() {
                let mut overbound =
                    max(0, x as i64 - img_bitmap.len() as i64 + 1) +
                    max(0, y as i64 - img_bitmap[0].len() as i64 + 1);
                if overbound <= 0 {
                    let img_coverage = img_bitmap[x as usize][y as usize];
                    let difference = (*c - img_coverage).abs();
                    matched_score += (1.0 - difference);
                } else {
                    matched_score -= c * (overbound * overbound) as f32 / 100.0;
                }
            }
        }
        let match_option = CharMatch {
            char,
            match_score: (10000000.0 * matched_score / max_possible_score) as i64,
        };
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

pub struct Bounds {
    pub start: Point,
    pub end: Point,
}

impl Bounds {
    pub fn get_width(&self) -> usize {
        return self.end.x - self.start.x;
    }

    pub fn get_height(&self) -> usize {
        return self.end.y - self.start.y;
    }
}

fn get_bounds(letter_pixels: &Vec<PixelCoverage>) -> Bounds {
    let mut min_x = 99999;
    let mut min_y = 99999;
    let mut max_x = 0;
    let mut max_y = 0;
    let collected: Vec<&PixelCoverage> = letter_pixels
        .iter().filter(|p| p.c > 0.001).collect();
    for PixelCoverage { x, y, .. } in &collected {
        min_x = min(min_x, *x);
        min_y = min(min_y, *y);
        max_x = max(max_x, *x);
        max_y = max(max_y, *y);
    }
    return Bounds {
        start: Point {
            x: min_x as usize,
            y: min_y as usize,
        },
        end: Point {
            x: max_x as usize,
            y: max_y as usize,
        },
    };
}

pub struct RelMatrix {
    pub bounds: Bounds,
    pub bitmap: Vec<Vec<f32>>,
}

fn make_rel_bitmap(letter_pixels: Vec<PixelCoverage>) -> RelMatrix {
    let bounds = get_bounds(&letter_pixels);
    let collected: Vec<&PixelCoverage> = letter_pixels
        .iter().filter(|p| p.c > 0.001).collect();
    let width = bounds.end.x - bounds.start.x + 1;
    let height = bounds.end.y - bounds.start.y + 1;
    let mut rel_bitmap = vec![vec![0.0; height]; width];
    for PixelCoverage { x, y, c } in &collected {
        rel_bitmap[(*x as usize - bounds.start.x)][(*y as usize - bounds.start.y)] = *c;
    }
    return RelMatrix {
        bounds: bounds,
        bitmap: rel_bitmap,
    };
}

pub fn make_rel_bitmap_from_image(letter_pixels: &[Pixel]) -> RelMatrix {
    return make_rel_bitmap(
        letter_pixels.iter()
            .map(|pixel| PixelCoverage {
                x: pixel.point.x as u32,
                y: pixel.point.y as u32,
                c: color_to_coverage(&pixel.color),
            })
            .collect()
    );
}

pub fn match_letter_to_font(rel_bitmap: &Vec<Vec<f32>>, font: &FontRef, index: usize) -> Vec<CharMatch> {
    let char_options = [
        'q', 'w', 'e', 'r', 't', 'y', 'u', 'i', 'o', 'p', 'a', 's', 'd', 'f', 'g', 'h', 'j', 'k',
        'l', 'z', 'x', 'c', 'v', 'b', 'n', 'm', 'Q', 'W', 'E', 'R', 'T', 'Y', 'U', 'I', 'O', 'P',
        'A', 'S', 'D', 'F', 'G', 'H', 'J', 'K', 'L', 'Z', 'X', 'C', 'V', 'B', 'N', 'M',
    ];
    let expected = ['T', 'h', 'h', 't', 't', 'b', 'e', 'r', 'e', 'a', 'r', 'e', 'm', 'a', 'n', 'y', 'e', 'o', 'r', 'e'];

    let mut matches = BinaryHeap::new();
    for char in &char_options {
        let is_expected = index < expected.len() && expected[index] == *char;
        let matched = match_bitmap_to_char(rel_bitmap, *char, font, is_expected, index);
        if is_expected {
            println!("expect match: {:?}", matched);
        }
        matches.push(matched);
    }
    return matches.into_vec();
}
