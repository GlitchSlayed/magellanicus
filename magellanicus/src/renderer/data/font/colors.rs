use std::collections::BTreeMap;

pub struct ColorCodes {
    pub codes: BTreeMap<char, ControlCode>
}

impl Default for ColorCodes {
    fn default() -> Self {
        let colors = include_str!("colors.txt")
            .lines()
            .filter(|l| !l.is_empty() && !l.starts_with("##"))
            .map(|l| {
                let (char_str, color_data) = l.split_at(1);
                let color_data = color_data.trim();
                let char = char_str.chars().next().unwrap();

                match color_data {
                    "XXXXXX" => (char, ControlCode::ResetColor),
                    n if n.len() == 6 => {
                        let mut color_chars = 0u32;
                        let mut color_chars_iterator = color_data.chars();
                        for _ in 0..6 {
                            let character = color_chars_iterator.next().unwrap().to_ascii_uppercase();
                            assert!(character.is_ascii_hexdigit(), "colors.txt is borked; invalid color hex code {n}");
                            let c = match character {
                                x if x.is_ascii_digit() => x as u8 - '0' as u8,
                                x => 0xA + x as u8 - 'A' as u8,
                            };
                            color_chars = (color_chars << 4) | (c as u32);
                        }

                        let r = ((color_chars >> 16) & 0xFF) as f32 / 255.0;
                        let g = ((color_chars >> 8) & 0xFF) as f32 / 255.0;
                        let b = ((color_chars >> 0) & 0xFF) as f32 / 255.0;

                        (char, ControlCode::Color([r,g,b]))
                    }
                    n => panic!("colors.txt is bork: unparseable modifier {n}")
                }
            }).collect();

        Self { codes: colors }
    }
}

pub enum ControlCode {
    Color([f32; 3]),
    ResetColor
}
