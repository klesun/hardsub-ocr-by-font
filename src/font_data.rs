use ab_glyph::{point, Point, Glyph, Font, FontRef};
use std::collections::HashMap;
use crate::rel_matrix::{RelMatrix, PixelCoverage, make_rel_bitmap};

pub const CHAR_OPTIONS: [char; 54] = [
    'q', 'w', 'e', 'r', 't', 'y', 'u', 'i', 'o', 'p', 'a', 's', 'd', 'f', 'g', 'h', 'j', 'k',
    'l', 'z', 'x', 'c', 'v', 'b', 'n', 'm', 'Q', 'W', 'E', 'R', 'T', 'Y', 'U', 'I', 'O', 'P',
    'A', 'S', 'D', 'F', 'G', 'H', 'J', 'K', 'L', 'Z', 'X', 'C', 'V', 'B', 'N', 'M', ',', '.',
];

fn get_font_bitmap(char: char, shift: &ab_glyph::Point, font: &FontRef) -> RelMatrix {
    let glyph: Glyph = font
        .glyph_id(char)
        .with_scale_and_position(24.0, *shift);

    let outlined = font.outline_glyph(glyph).unwrap();

    let mut coverages = Vec::new();
    outlined.draw(|x, y, c| {
        coverages.push(PixelCoverage { x, y, c });
    });
    return make_rel_bitmap(coverages);
}

pub struct FontData {
    pub char_to_shift_to_matrix: HashMap<char, Vec<RelMatrix>>,
}

impl FontData {
    pub fn init(font: &FontRef) -> FontData {
        let shift_options = [
            point(0.0, 0.0),
            point(0.5, 0.0),
            point(0.0, 0.5),
            point(0.5, 0.5),
        ];
        let mut char_to_shift_to_matrix: HashMap<char, Vec<RelMatrix>> = HashMap::new();
        for char in &CHAR_OPTIONS {
            let bitmaps: Vec<RelMatrix> = shift_options.iter()
                .map(|font_shift| get_font_bitmap(*char, font_shift, font) )
                .collect();
            char_to_shift_to_matrix.insert(*char, bitmaps);
        }
        return FontData {
            char_to_shift_to_matrix,
        }
    }

    pub fn get_bitmaps(&self, char: char) -> &[RelMatrix] {
        return self.char_to_shift_to_matrix.get(&char).unwrap();
    }
}
