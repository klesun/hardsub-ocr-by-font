
//! basically just combination of bounds points and the bitmap + helper methods

use crate::pixel_utils::Point;
use std::cmp::{min, max};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Bounds {
    pub start: Point,
    pub end: Point,
}

impl Bounds {
    pub fn get_width(&self) -> usize {
        return (self.end.x - self.start.x + 1) as usize;
    }

    pub fn get_height(&self) -> usize {
        return (self.end.y - self.start.y + 1) as usize;
    }
}

pub struct PixelCoverage {
    pub x: u32,
    pub y: u32,
    pub c: f32,
}

pub fn get_bounds(letter_pixels: &Vec<PixelCoverage>) -> Bounds {
    let mut min_x = 99999;
    let mut min_y = 99999;
    let mut max_x = 0;
    let mut max_y = 0;
    let collected: Vec<&PixelCoverage> = letter_pixels.iter().filter(|p| p.c > 0.001).collect();
    for PixelCoverage { x, y, .. } in &collected {
        min_x = min(min_x, *x);
        min_y = min(min_y, *y);
        max_x = max(max_x, *x);
        max_y = max(max_y, *y);
    }
    return Bounds {
        start: Point {
            x: min_x as i64,
            y: min_y as i64,
        },
        end: Point {
            x: max_x as i64,
            y: max_y as i64,
        },
    };
}

pub struct RelMatrix {
    pub bounds: Bounds,
    pub bitmap: Vec<Vec<f32>>,
}

pub fn make_rel_bitmap(letter_pixels: Vec<PixelCoverage>) -> RelMatrix {
    let bounds = get_bounds(&letter_pixels);
    let collected: Vec<&PixelCoverage> = letter_pixels.iter().filter(|p| p.c > 0.001).collect();
    let width = bounds.end.x - bounds.start.x + 1;
    let height = bounds.end.y - bounds.start.y + 1;
    let mut rel_bitmap = vec![vec![0.0; height as usize]; width as usize];
    for PixelCoverage { x, y, c } in &collected {
        rel_bitmap[(*x as usize - bounds.start.x as usize)]
            [(*y as usize - bounds.start.y as usize)] = *c;
    }
    return RelMatrix {
        bounds: bounds,
        bitmap: rel_bitmap,
    };
}
