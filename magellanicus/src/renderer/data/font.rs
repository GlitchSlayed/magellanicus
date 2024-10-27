mod colors;

use alloc::collections::BTreeMap;
use core::str::Chars;
use core::iter::FusedIterator;
use alloc::vec;
use alloc::vec::Vec;
use core::ops::Range;
use std::println;
use glam::Vec4;
use crate::error::MResult;
use crate::FloatColor;
use crate::renderer::{AddBitmapBitmapParameter, AddBitmapParameter, AddBitmapSequenceParameter, AddFontParameter, BitmapFormat, BitmapType, Renderer, Resolution};
use crate::renderer::data::font::colors::{ControlCode, ColorCodes};

pub struct Font {
    pub line_height: u32,
    pub characters: BTreeMap<char, FontCharacter>,
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
                    data: c.data,
                    width: c.width,
                    height: c.height,
                    advance_x: c.advance_x
                };
                (c.characters, character)
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
    pub fn draw_string_to_bitmap(&self, string: &str, request: FontDrawRequest) -> AddBitmapParameter {
        let Some(pixel_count) = request.resolution.width.checked_mul(request.resolution.height) else {
            panic!("width * height overflows")
        };

        let mut bitmap_data: Vec<FloatColor> = vec![[0f32; 4]; pixel_count as usize];
        let characters: Vec<FontCharacterIterated> = self.iterate_characters(string, request.color, TextState {
            default_alignment: request.alignment,
            current_alignment: None,
            ..Default::default()
        }).collect();

        let mut current_line_range = 1..usize::MAX;
        let mut offset_x = 0i32;
        for i in 0..characters.len() {
            let character = &characters[i];
            if !current_line_range.contains(&i) {
                Self::handle_new_line(request, &characters, &mut current_line_range, &mut offset_x, i, character);
            }

            let offset_y = character.state.y as i32;

            // Draw the drop shadow
            Self::draw_character(
                request,
                bitmap_data.as_mut_slice(),
                character,
                [0.0, 0.0, 0.0, character.color[3]],
                offset_x + 1,
                offset_y + 1
            );

            // Now the actual color
            Self::draw_character(
                request,
                bitmap_data.as_mut_slice(),
                character,
                character.color,
                offset_x,
                offset_y
            );

            offset_x += character.character.advance_x;
        }

        // SAFETY: If this fails, then it's a skill issue, and you should get a better computer.
        let destruction_9000: Vec<u8> = unsafe {
            let mut v_clone = core::mem::ManuallyDrop::new(bitmap_data);
            Vec::from_raw_parts(v_clone.as_mut_ptr() as *mut u8, v_clone.len() * 16, v_clone.capacity())
        };

        let bitmap = AddBitmapBitmapParameter {
            format: BitmapFormat::R32G32B32A32SFloat,
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
        request: FontDrawRequest,
        bitmap_data: &mut [[f32; 4]],
        character: &FontCharacterIterated,
        color: FloatColor,
        x_offset: i32,
        y_offset: i32,
    ) {
        for x in 0..character.character.width {
            let x_offset = x_offset + x as i32;
            if x_offset < 0 {
                continue;
            }
            let x_offset = x_offset as usize;
            if x_offset >= request.resolution.width as usize {
                break;
            }
            for y in 0..character.character.height {
                let y_offset = y_offset + y as i32;
                if y_offset < 0 {
                    continue;
                }
                let y_offset = y_offset as usize;
                if y_offset >= request.resolution.height as usize {
                    break;
                }

                let alpha = character.character.data[x + y * character.character.width] as f32 / 255.0;
                if alpha == 0.0 {
                    continue;
                }

                let mut color = color;
                color[3] *= alpha;

                let modified_pixel = &mut bitmap_data[x_offset + y_offset * request.resolution.width as usize];
                let original_pixel = Vec4::from(*modified_pixel);
                let new_pixel = Vec4::from(color);

                *modified_pixel = original_pixel.lerp(new_pixel, color[3]).to_array();
            }
        }
    }

    fn handle_new_line(request: FontDrawRequest, characters: &[FontCharacterIterated], current_line_range: &mut Range<usize>, offset_x: &mut i32, i: usize, character: &FontCharacterIterated) {
        let start = i;
        let mut end = characters.len();
        for j in i + 1..characters.len() {
            if characters[j].alignment_changed {
                end = j;
                break;
            }
        }
        *current_line_range = start..end;

        let alignment = character.state.current_alignment.unwrap_or(character.state.default_alignment);
        *offset_x = match alignment {
            TextAlignment::Left => 0,
            TextAlignment::Table(n) => request.tab_offsets.get(n).map(|b| *b).unwrap_or_default(),
            TextAlignment::Right | TextAlignment::Center => {
                let mut total_width = 0i32;
                for i in current_line_range.clone() {
                    total_width += characters[i].character.advance_x
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
        assert!(text_position.current_alignment.is_none());
        FontCharacterIterator {
            font: self,
            string: string.chars(),
            color_code_entry: false,
            pipe_entry: false,
            text_state: text_position,
            modified_color: color,
            original_color: color
        }
    }
}

#[derive(Default, Copy, Clone, Debug)]
struct TextState {
    pub default_alignment: TextAlignment,
    pub current_alignment: Option<TextAlignment>,
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
    text_state: TextState,
}

struct FontCharacterIterated<'font> {
    pub character: &'font FontCharacter,
    pub color: FloatColor,
    pub state: TextState,
    pub alignment_changed: bool,
}

impl<'font, 'string> Iterator for FontCharacterIterator<'font, 'string> {
    type Item = FontCharacterIterated<'font>;

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
                            println!("{r} {g} {b}");
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
                        'c' => self.text_state.current_alignment = Some(TextAlignment::Center),
                        'r' => self.text_state.current_alignment = Some(TextAlignment::Right),
                        'l' => self.text_state.current_alignment = Some(TextAlignment::Left),
                        't' => {
                            let table_position = self.text_state.current_alignment.unwrap_or(self.text_state.default_alignment);
                            let index = if let TextAlignment::Table(n) = table_position {
                                n + 1
                            }
                            else {
                                0
                            };
                            self.text_state.current_alignment = Some(TextAlignment::Table(index))
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

        Some(FontCharacterIterated {
            character,
            color: self.modified_color,
            state: self.text_state,
            alignment_changed
        })
    }
}

impl<'font, 'string> FontCharacterIterator<'font, 'string> {
    fn newline(&mut self) {
        self.text_state.y += self.font.line_height;
        self.text_state.current_alignment = None;
    }
}

impl<'font, 'string> FusedIterator for FontCharacterIterator<'font, 'string> {}
