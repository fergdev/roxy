use cow_utils::CowUtils;
use ratatui::style::Color;
use serde::{Deserialize, Deserializer, Serialize, de};
use tracing::error;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RoxyColors {
    #[serde(deserialize_with = "deserialize_color")]
    pub primary: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub on_primary: Color,

    #[serde(deserialize_with = "deserialize_color")]
    pub secondary: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub on_secondary: Color,

    #[serde(deserialize_with = "deserialize_color")]
    pub surface: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub surface_hl: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub on_surface: Color,

    #[serde(deserialize_with = "deserialize_color")]
    pub background: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub on_background: Color,

    #[serde(deserialize_with = "deserialize_color")]
    pub outline: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub outline_unfocused: Color,

    #[serde(deserialize_with = "deserialize_color")]
    pub error: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub success: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub info: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub warn: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub debug: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub trace: Color,
}

pub fn deserialize_color<'de, D>(deserializer: D) -> Result<Color, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer).unwrap_or("#ffffff".to_string());
    parse_color(&s).map_err(de::Error::custom)
}

pub fn parse_color(raw: &str) -> Result<Color, String> {
    let raw = raw.trim();

    if let Ok(c) = parse_named_color(raw) {
        return Ok(c);
    }

    if let Some(rgb) = raw.strip_prefix("Rgb(").and_then(|s| s.strip_suffix(")")) {
        let parts: Vec<_> = rgb.split(',').map(|s| s.trim()).collect();
        if parts.len() == 3 {
            let r = parts[0].parse::<u8>().map_err(|_| "bad red value")?;
            let g = parts[1].parse::<u8>().map_err(|_| "bad green value")?;
            let b = parts[2].parse::<u8>().map_err(|_| "bad blue value")?;
            return Ok(Color::Rgb(r, g, b));
        }
    }

    if let Some(hex) = raw.strip_prefix('#')
        && hex.len() == 6
    {
        let r = u8::from_str_radix(&hex[0..2], 16).map_err(|_| "bad hex")?;
        let g = u8::from_str_radix(&hex[2..4], 16).map_err(|_| "bad hex")?;
        let b = u8::from_str_radix(&hex[4..6], 16).map_err(|_| "bad hex")?;
        return Ok(Color::Rgb(r, g, b));
    }

    error!("Unable to parse color {raw}");
    Ok(Color::Magenta)
}

fn parse_named_color(name: &str) -> Result<Color, ()> {
    use Color::*;
    Ok(match name.cow_to_lowercase().as_ref() {
        "black" => Black,
        "red" => Red,
        "green" => Green,
        "yellow" => Yellow,
        "blue" => Blue,
        "magenta" => Magenta,
        "cyan" => Cyan,
        "gray" => Gray,
        "darkgray" => DarkGray,
        "lightred" => LightRed,
        "lightgreen" => LightGreen,
        "lightyellow" => LightYellow,
        "lightblue" => LightBlue,
        "lightmagenta" => LightMagenta,
        "lightcyan" => LightCyan,
        "white" => White,
        _ => return Err(()),
    })
}

#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[cfg(test)]
pub mod tests {
    use ratatui::style::Color;

    #[test]
    fn parse_color_named() {
        assert_eq!(Color::Black, super::parse_color("black").unwrap());
        assert_eq!(Color::White, super::parse_color("white").unwrap());
    }
    #[test]
    fn parse_color_rgb() {
        assert_eq!(
            Color::Rgb(0, 0, 0),
            super::parse_color("Rgb(0,0,0)").unwrap()
        );
        assert_eq!(
            Color::Rgb(255, 255, 255),
            super::parse_color("Rgb(255,255,255)").unwrap()
        );
    }
    #[test]
    fn parse_color_hex() {
        assert_eq!(Color::Rgb(0, 0, 0), super::parse_color("#000000").unwrap());
        assert_eq!(
            Color::Rgb(255, 255, 255),
            super::parse_color("#FFFFFF").unwrap()
        );
    }
}
