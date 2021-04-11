
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
pub fn rgb_to_hsl(rb: u8, gb: u8, bb: u8) -> [f64; 3] {
    let r: f64 = rb as f64 / 255.0;
    let g: f64 = gb as f64 / 255.0;
    let b: f64 = bb as f64 / 255.0;
    let max = r.max(g.max(b));
    let min = r.min(g.min(b));
    let l = (max + min) as f64 / 2.0;

    if max == min {
        return [0.0, 0.0, l]; // achromatic
    } else {
        let d = max - min;
        let s = if l > 0.5
        { d / (2.0 - max - min) } else
        { d / (max + min) };
        let mut h = if max == r {
            (g - b) / d + (if g < b { 6.0 } else { 0.0 })
        } else if max == g {
            (b - r) / d + 2.0
        } else { // max == b
            (r - g) / d + 4.0
        };
        h /= 6.0;
        return [h, s, l];
    }
}

pub fn is_whitish(point: &Point, [r, g, b]: [u8; 3]) -> bool {
    let [h, s, l] = rgb_to_hsl(r, g, b);

    let is_whitish = l > 0.8 || s < 0.2 && l > 0.4;
    println!("Ololo is_whitish {}x{} {} - ({}, {}, {}) -> ({}, {}, {})", point.x, point.y, is_whitish, r, g, b, h, s, l);

    return is_whitish;
}

#[derive(Debug, Copy, Clone)]
pub struct Point {
    pub x: usize,
    pub y: usize,
}

pub fn get_surrounding(base_point: &Point, width: usize, height: usize) -> impl Iterator<Item = Point> {
    let options = [
        Point { x: base_point.x - 1, y: base_point.y },
        Point { x: base_point.x    , y: base_point.y + 1 },
        Point { x: base_point.x + 1, y: base_point.y },
        Point { x: base_point.x    , y: base_point.y - 1 },
    ];
    return std::array::IntoIter::new(options)
        .filter(move |option| {
            return option.x < width
                && option.y < height
                && option.x >= 0
                && option.y >= 0;
        });
}