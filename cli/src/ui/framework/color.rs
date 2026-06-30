use ratatui::style::Color;

fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

pub fn luminance(r: f32, g: f32, b: f32) -> f32 {
    let r = srgb_to_linear(r);
    let g = srgb_to_linear(g);
    let b = srgb_to_linear(b);
    (0.2126 * r + 0.7152 * g + 0.0722 * b).clamp(0.0, 1.0)
}

pub fn luminance_color(color: &Color) -> f32 {
    let rgb = rgb(color);
    let r = rgb.0 as f32 / 255.0;
    let g = rgb.1 as f32 / 255.0;
    let b = rgb.2 as f32 / 255.0;
    luminance(r, g, b)
}

pub fn color_is_light(color: &Color) -> bool {
    luminance_color(color) > 0.5
}

fn rgb(color: &Color) -> (u8, u8, u8) {
    match color {
        Color::Rgb(r, g, b) => (r.to_owned(), g.to_owned(), b.to_owned()),
        Color::Indexed(i) => indexed_to_rgb(i.to_owned()),
        Color::Black => (0, 0, 0),
        Color::Red => (205, 0, 0),
        Color::Green => (0, 205, 0),
        Color::Yellow => (205, 205, 0),
        Color::Blue => (0, 0, 238),
        Color::Magenta => (205, 0, 205),
        Color::Cyan => (0, 205, 205),
        Color::Gray => (229, 229, 229),
        Color::DarkGray => (127, 127, 127),
        Color::LightRed => (255, 0, 0),
        Color::LightGreen => (0, 255, 0),
        Color::LightYellow => (255, 255, 0),
        Color::LightBlue => (92, 92, 255),
        Color::LightMagenta => (255, 0, 255),
        Color::LightCyan => (0, 255, 255),
        Color::White => (255, 255, 255),
        Color::Reset => (255, 255, 255), // assume light background, or pick a default
    }
}

fn indexed_to_rgb(i: u8) -> (u8, u8, u8) {
    match i {
        0..=15 => match i {
            0 => (0, 0, 0),
            1 => (205, 0, 0),
            2 => (0, 205, 0),
            3 => (205, 205, 0),
            4 => (0, 0, 238),
            5 => (205, 0, 205),
            6 => (0, 205, 205),
            7 => (229, 229, 229),
            8 => (127, 127, 127),
            9 => (255, 0, 0),
            10 => (0, 255, 0),
            11 => (255, 255, 0),
            12 => (92, 92, 255),
            13 => (255, 0, 255),
            14 => (0, 255, 255),
            15 => (255, 255, 255),
            _ => unreachable!(),
        },
        16..=231 => {
            // 6x6x6 color cube
            let i = i - 16;
            let r = (i / 36) % 6;
            let g = (i / 6) % 6;
            let b = i % 6;
            let to_val = |c: u8| if c == 0 { 0 } else { 55 + c * 40 };
            (to_val(r), to_val(g), to_val(b))
        }
        232..=255 => {
            // grayscale ramp
            let gray = 8 + (i - 232) * 10;
            (gray, gray, gray)
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;

    fn assert_close(actual: f32, expected: f32) {
        let diff = (actual - expected).abs();
        assert!(
            diff < 0.0001,
            "actual={actual}, expected={expected}, diff={diff}"
        );
    }

    #[test]
    fn luminance_black_is_zero() {
        assert_close(luminance(0.0, 0.0, 0.0), 0.0);
    }

    #[test]
    fn luminance_white_is_one() {
        assert_close(luminance(1.0, 1.0, 1.0), 1.0);
    }

    #[test]
    fn luminance_primary_colors_match_wcag_weights() {
        assert_close(luminance(1.0, 0.0, 0.0), 0.2126);
        assert_close(luminance(0.0, 1.0, 0.0), 0.7152);
        assert_close(luminance(0.0, 0.0, 1.0), 0.0722);
    }

    #[test]
    fn luminance_color_handles_rgb() {
        assert_close(luminance_color(&Color::Rgb(255, 255, 255)), 1.0);
        assert_close(luminance_color(&Color::Rgb(0, 0, 0)), 0.0);
    }

    #[test]
    fn color_is_light_detects_light_and_dark_colors() {
        assert!(color_is_light(&Color::White));
        assert!(color_is_light(&Color::LightYellow));

        assert!(!color_is_light(&Color::Black));
        assert!(!color_is_light(&Color::Blue));
    }

    #[test]
    fn indexed_basic_colors_match_named_colors() {
        assert_eq!(rgb(&Color::Indexed(0)), rgb(&Color::Black));
        assert_eq!(rgb(&Color::Indexed(1)), rgb(&Color::Red));
        assert_eq!(rgb(&Color::Indexed(15)), rgb(&Color::White));
    }

    #[test]
    fn indexed_color_cube_values_are_correct() {
        assert_eq!(indexed_to_rgb(16), (0, 0, 0));
        assert_eq!(indexed_to_rgb(17), (0, 0, 95));
        assert_eq!(indexed_to_rgb(21), (0, 0, 255));
        assert_eq!(indexed_to_rgb(231), (255, 255, 255));
    }

    #[test]
    fn indexed_grayscale_values_are_correct() {
        assert_eq!(indexed_to_rgb(232), (8, 8, 8));
        assert_eq!(indexed_to_rgb(233), (18, 18, 18));
        assert_eq!(indexed_to_rgb(255), (238, 238, 238));
    }

    #[test]
    fn reset_is_treated_as_light_background() {
        assert!(color_is_light(&Color::Reset));
        assert_close(luminance_color(&Color::Reset), 1.0);
    }
}
