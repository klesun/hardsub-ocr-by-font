
/**
 * @see https://stackoverflow.com/a/9493060/2750743
 *
 * Converts an RGB color value to HSL. Conversion formula
 * adapted from http://en.wikipedia.org/wiki/HSL_color_space.
 * Assumes r, g, and b are contained in the set [0, 255] and
 * returns h, s, and l in the set [0, 1].
 *
 * @param   {number}  r       The red color value
 * @param   {number}  g       The green color value
 * @param   {number}  b       The blue color value
 * @return  {Array}           The HSL representation
 */
pub fn rgb_to_hsl(pixel: &Color) -> [f64; 3] {
    let r: f64 = pixel.r as f64 / 255.0;
    let g: f64 = pixel.g as f64 / 255.0;
    let b: f64 = pixel.b as f64 / 255.0;
    let max = r.max(g.max(b));
    let min = r.min(g.min(b));
    let l = (max + min) as f64 / 2.0;

    return if max == min {
        [0.0, 0.0, l] // achromatic
    } else {
        let d = max - min;
        let s = if l > 0.5
        { d / (2.0 - max - min) } else { d / (max + min) };
        let mut h = if max == r {
            (g - b) / d + (if g < b { 6.0 } else { 0.0 })
        } else if max == g {
            (b - r) / d + 2.0
        } else { // max == b
            (r - g) / d + 4.0
        };
        h /= 6.0;
        [h, s, l]
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub const BLACK: Color = Color { r: 0, g: 0, b: 0 };

    pub fn get_hue(&self) -> f64 {
        let [h, _s, _l] = rgb_to_hsl(self);
        return h;
    }

    pub fn get_saturation(&self) -> f64 {
        let [_h, s, _l] = rgb_to_hsl(self);
        return s;
    }

    pub fn get_lightness(&self) -> f64 {
        let [_h, _s, l] = rgb_to_hsl(self);
        return l;
    }

    pub fn is_nearly_white(&self) -> bool {
        let [_h, _s, l] = rgb_to_hsl(&self);
        return l > 0.95;
    }

    pub fn is_closely_white(&self) -> bool {
        let [_h, _s, l] = rgb_to_hsl(&self);
        return l > 0.80;
    }

    pub fn is_somewhat_white(&self) -> bool {
        let [_h, s, l] = rgb_to_hsl(&self);
        return self.is_closely_white() || s < 0.20 && l > 0.35;
    }

    pub fn to_vector(&self) -> [u8; 3] {
        return [self.r, self.g, self.b];
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Point {
    pub x: i64,
    pub y: i64,
}

pub struct Pixel {
    pub point: Point,
    pub color: Color,
}

pub fn get_surrounding(base_point: &Point, width: usize, height: usize) -> Vec<Point> {
    let mut options = Vec::new();
    if base_point.x > 0 {
        options.push(Point { x: base_point.x - 1, y: base_point.y });
    }
    if (base_point.x as usize) < width - 1 {
        options.push(Point { x: base_point.x + 1, y: base_point.y });
    }
    if base_point.y > 0 {
        options.push(Point { x: base_point.x, y: base_point.y - 1 });
    }
    if (base_point.y as usize) < height - 1 {
        options.push(Point { x: base_point.x, y: base_point.y + 1 });
    }
    return options;
}
