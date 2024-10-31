mod colors;

use std::collections::HashMap;
use std::iter::FusedIterator;
use std::ops::Range;
use std::str::Chars;
use glam::Vec4;
use crate::error::MResult;
use crate::types::FloatColor;
use crate::renderer::{AddBitmapBitmapParameter, AddBitmapParameter, AddBitmapSequenceParameter, AddFontParameter, BitmapFormat, BitmapType, Renderer, Resolution};
use crate::renderer::data::font::colors::{ControlCode, ColorCodes};

pub struct Font {
    pub line_height: u32,
    pub characters: HashMap<char, FontCharacter>,
    pub colors: ColorCodes
}

impl Font {
    pub fn load_from_parameters(_: &Renderer, parameter: AddFontParameter) -> MResult<Font> {
        // TODO: Add bold/italic/underline variants

        let characters = parameter
            .characters
            .into_iter()
            .map(|c| {
                let character = FontCharacter {
                    character: c.character,
                    data: c.data,
                    width: c.width,
                    height: c.height,
                    advance_x: c.advance_x
                };
                (c.character, character)
            })
            .collect();

        Ok(Font {
            line_height: parameter.line_height,
            characters,
            colors: ColorCodes::default()
        })
    }
}

pub struct FontCharacter {
    pub character: char,
    pub data: Vec<u8>,
    pub width: usize,
    pub height: usize,
    pub advance_x: i32
}

#[derive(Default, Copy, Clone, PartialEq)]
pub struct FontDrawRequest {
    pub alignment: TextAlignment,
    pub color: FloatColor,
    pub resolution: Resolution,
    pub tab_offsets: [i32; 8],
}

impl Font {
    pub fn generate_string_draws(&self, string: &str, request: FontDrawRequest, characters: &mut Vec<DrawableCharacter>) {
        characters.clear();
        characters.reserve(string.len());
        characters.extend(self.iterate_characters(string, request.color, TextState {
            alignment: request.alignment,
            ..Default::default()
        }));

        let mut offset_x = 0i32;
        let mut current_line_range = 1..usize::MAX;
        for i in 0..characters.len() {
            if !current_line_range.contains(&i) {
                self.handle_new_line(request, &characters, &mut current_line_range, &mut offset_x, i);
            }

            let character = &mut characters[i];
            character.x = offset_x;
            character.y = character.state.y as i32;
            offset_x += self.characters[&character.character].advance_x;
        }
    }

    pub fn draw_string_buffer_to_bitmap(&self, characters: &[DrawableCharacter], request: FontDrawRequest) -> AddBitmapParameter {
        let Some(pixel_count) = request.resolution.width.checked_mul(request.resolution.height) else {
            panic!("width * height overflows")
        };

        let mut bitmap_data: Vec<[u8; 4]> = vec![[0u8; 4]; pixel_count as usize];
        for character in characters {
            // Draw the drop shadow
            self.draw_character(
                request,
                bitmap_data.as_mut_slice(),
                character,
                [0.0, 0.0, 0.0, character.color[3]],
                character.x + 1,
                character.y + 1
            );

            // Now the actual color
            self.draw_character(
                request,
                bitmap_data.as_mut_slice(),
                character,
                character.color,
                character.x,
                character.y
            );
        }

        // SAFETY: If this fails, then it's a skill issue, and you should get a better computer.
        let destruction_9000: Vec<u8> = unsafe {
            let mut v_clone = core::mem::ManuallyDrop::new(bitmap_data);
            Vec::from_raw_parts(v_clone.as_mut_ptr() as *mut u8, v_clone.len() * 4, v_clone.capacity())
        };

        let bitmap = AddBitmapBitmapParameter {
            format: BitmapFormat::A8B8G8R8,
            bitmap_type: BitmapType::Dim2D,
            resolution: request.resolution,
            mipmap_count: 0,
            data: destruction_9000
        };

        AddBitmapParameter {
            bitmaps: vec![bitmap],
            sequences: vec![AddBitmapSequenceParameter::Bitmap { first: 0, count: 1 }]
        }
    }

    fn draw_character(
        &self,
        request: FontDrawRequest,
        bitmap_data: &mut [[u8; 4]],
        character: &DrawableCharacter,
        color: FloatColor,
        x_offset: i32,
        y_offset: i32,
    ) {
        let character_data = &self.characters[&character.character];

        for x in 0..character_data.width {
            let x_offset = x_offset + x as i32;
            if x_offset < 0 {
                continue;
            }
            let x_offset = x_offset as usize;
            if x_offset >= request.resolution.width as usize {
                break;
            }
            for y in 0..character_data.height {
                let y_offset = y_offset + y as i32;
                if y_offset < 0 {
                    continue;
                }
                let y_offset = y_offset as usize;
                if y_offset >= request.resolution.height as usize {
                    break;
                }

                let alpha = character_data.data[x + y * character_data.width] as f32 / 255.0;
                if alpha == 0.0 {
                    continue;
                }

                let mut color = color;
                color[3] *= alpha;

                let modified_pixel = &mut bitmap_data[x_offset + y_offset * request.resolution.width as usize];
                let original_pixel = Vec4::from([
                    modified_pixel[0] as f32 / 255.0,
                    modified_pixel[1] as f32 / 255.0,
                    modified_pixel[2] as f32 / 255.0,
                    modified_pixel[3] as f32 / 255.0
                ]);
                let new_pixel = Vec4::from(color);

                let result = original_pixel.lerp(new_pixel, color[3]).to_array();

                *modified_pixel = [
                    (result[0] * 255.0) as u8,
                    (result[1] * 255.0) as u8,
                    (result[2] * 255.0) as u8,
                    (result[3] * 255.0) as u8,
                ];
            }
        }
    }

    fn handle_new_line(
        &self,
        request: FontDrawRequest,
        characters: &[DrawableCharacter],
        current_line_range:
        &mut Range<usize>,
        offset_x: &mut i32,
        i: usize,
    ) {
        let first_character = &characters[i];
        let start = i;
        let mut end = characters.len();
        for j in i + 1..characters.len() {
            if characters[j].alignment_changed {
                end = j;
                break;
            }
        }
        *current_line_range = start..end;

        let alignment = first_character.state.alignment;
        *offset_x = match alignment {
            TextAlignment::Left => 0,
            TextAlignment::Table(n) => request.tab_offsets.get(n).map(|b| *b).unwrap_or_default(),
            TextAlignment::Right | TextAlignment::Center => {
                let mut total_width = 0i32;
                for i in current_line_range.clone() {
                    total_width += self.characters[&characters[i].character].advance_x
                }

                let offset = (request.resolution.width as i32) - (total_width);
                if alignment == TextAlignment::Center {
                    offset + (total_width) / 2
                } else {
                    offset
                }
            }
        }
    }

    fn iterate_characters<'font, 'string>(
        &'font self,
        string: &'string str,
        color: FloatColor,
        text_position: TextState,
    ) -> FontCharacterIterator<'font, 'string> {
        FontCharacterIterator {
            font: self,
            string: string.chars(),
            color_code_entry: false,
            pipe_entry: false,
            text_state: text_position,
            modified_color: color,
            original_color: color,
            default_alignment: text_position.alignment
        }
    }
}

#[derive(Default, Copy, Clone, Debug)]
pub struct TextState {
    pub alignment: TextAlignment,
    pub y: u32,
    pub bold: bool,
    pub underline: bool,
    pub italics: bool
}

#[derive(Default, Copy, Clone, Debug, PartialEq, Hash)]
#[repr(u8)]
pub enum TextAlignment {
    #[default]
    Left,
    Right,
    Center,
    Table(usize)
}

struct FontCharacterIterator<'font, 'string> {
    font: &'font Font,
    string: Chars<'string>,
    color_code_entry: bool,
    pipe_entry: bool,
    original_color: FloatColor,
    modified_color: FloatColor,
    default_alignment: TextAlignment,
    text_state: TextState,
}

pub struct DrawableCharacter {
    pub character: char,
    pub color: FloatColor,
    pub state: TextState,
    pub alignment_changed: bool,

    pub x: i32,
    pub y: i32
}

impl<'font, 'string> Iterator for FontCharacterIterator<'font, 'string> {
    type Item = DrawableCharacter;

    fn next(&mut self) -> Option<Self::Item> {
        let mut alignment_changed = false;

        let character = loop {
            let next = self.string.next()?;
            if next.is_ascii_control() {
                self.color_code_entry = false;
                self.pipe_entry = false;
                match next {
                    '\n' => {
                        alignment_changed = true;
                        self.newline();
                        continue;
                    },
                    _ => continue
                }
            }

            if self.color_code_entry {
                self.color_code_entry = false;
                if next != '^' {
                    let Some(code) = self.font.colors.codes.get(&next) else {
                        continue;
                    };
                    match code {
                        ControlCode::Color([r, g, b]) => {
                            self.modified_color = [*r, *g, *b, self.modified_color[3]]
                        },
                        ControlCode::ResetColor => self.modified_color = self.original_color
                    }
                    continue;
                }
            }
            else if next == '^' {
                self.color_code_entry = true;
                self.pipe_entry = false;
                continue;
            }

            if self.pipe_entry {
                self.pipe_entry = false;
                if next != '|' {
                    alignment_changed = true;
                    match next {
                        'c' => self.text_state.alignment = TextAlignment::Center,
                        'r' => self.text_state.alignment = TextAlignment::Right,
                        'l' => self.text_state.alignment = TextAlignment::Left,
                        't' => {
                            let index = if let TextAlignment::Table(n) = self.text_state.alignment {
                                n + 1
                            }
                            else {
                                0
                            };
                            self.text_state.alignment = TextAlignment::Table(index)
                        },
                        'n' => self.newline(),
                        'b' => self.text_state.bold = true,
                        'u' => self.text_state.underline = true,
                        'i' => self.text_state.italics = true,
                        _ => ()
                    }
                }
            }
            else if next == '|' {
                self.pipe_entry = true;
                continue
            }

            let Some(as_char) = self.font.characters.get(&next) else {
                continue;
            };

            break as_char;
        };

        Some(DrawableCharacter {
            character: character.character,
            color: self.modified_color,
            state: self.text_state,
            alignment_changed,
            x: 0,
            y: 0
        })
    }
}

impl<'font, 'string> FontCharacterIterator<'font, 'string> {
    fn newline(&mut self) {
        self.text_state.y += self.font.line_height;
        self.text_state.alignment = self.default_alignment;
    }
}

impl<'font, 'string> FusedIterator for FontCharacterIterator<'font, 'string> {}
