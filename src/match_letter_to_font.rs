//! this module takes an array of present pixels (bitmap with alpha conceptually), it then
//! matches the image to every english letter in Sans-serif font and returns the best match
//! an OCR if you will

use crate::pixel_utils::{Color, Pixel, Point};
use ab_glyph::{point, Font, FontRef, Glyph};
use std::cmp::{max, min};

fn match_bitmap_to_char(rel_bitmap: &Vec<Vec<Color>>, c: char, font: &FontRef) {
    let q_glyph: Glyph = font
        .glyph_id(c)
        .with_scale_and_position(21.0, point(0.0, 0.0));

    let q = font.outline_glyph(q_glyph).unwrap();
    let mut matched_cnt = 0;
    let mut mismatched_cnt = 0;
    q.draw(|x, y, c| {
        // println!("{} {} {}", x, y, c);
        let matches: bool;
        let is_empty_in_font = c < 0.001;
        let is_empty_in_bitmap: bool;
        if (x as usize) < rel_bitmap.len() && (y as usize) < rel_bitmap[x as usize].len() {
            is_empty_in_bitmap = rel_bitmap[x as usize][y as usize] == Color::BLACK;
        } else {
            is_empty_in_bitmap = true;
        }
        if is_empty_in_bitmap == is_empty_in_font {
            matched_cnt += 1;
        } else {
            mismatched_cnt += 1;
        }
    });
    println!(
        "letter {} match: {} ok vs {} err",
        c, matched_cnt, mismatched_cnt
    );
}

fn make_rel_bitmap(letter_pixels: &[Pixel]) -> Vec<Vec<Color>> {
    let mut min_x = 99999;
    let mut min_y = 99999;
    let mut max_x = 0;
    let mut max_y = 0;
    for pixel in letter_pixels {
        min_x = min(min_x, pixel.point.x);
        min_y = min(min_y, pixel.point.y);
        max_x = max(max_x, pixel.point.x);
        max_y = max(max_y, pixel.point.y);
    }
    let width = max_x - min_x + 1;
    let height = max_y - min_y + 1;
    let mut rel_bitmap = vec![vec![Color::BLACK; height]; width];
    for Pixel { point, color } in letter_pixels {
        rel_bitmap[point.x - min_x][point.y - min_y] = *color;
    }
    return rel_bitmap;
}

pub fn match_letter_to_font(letter_pixels: &[Pixel], font: &FontRef) {
    let rel_bitmap = make_rel_bitmap(letter_pixels);
    let char_options = [
        'q', 'w', 'e', 'r', 't', 'y', 'u', 'i', 'o', 'p', 'a', 's', 'd', 'f', 'g', 'h', 'j', 'k',
        'l', 'z', 'x', 'c', 'v', 'b', 'n', 'm', 'Q', 'W', 'E', 'R', 'T', 'Y', 'U', 'I', 'O', 'P',
        'A', 'S', 'D', 'F', 'G', 'H', 'J', 'K', 'L', 'Z', 'X', 'C', 'V', 'B', 'N', 'M',
    ];
    for c in &char_options {
        match_bitmap_to_char(&rel_bitmap, *c, font);
    }
}
