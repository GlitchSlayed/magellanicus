use crate::error::{Error, MResult};

pub struct AddFontParameter {
    pub characters: Vec<AddFontParameterCharacter>,
    pub line_height: u32,
}

impl AddFontParameter {
    pub(crate) fn validate(&self) -> MResult<()> {
        for i in &self.characters {
            i.validate()?;
        }
        Ok(())
    }
}

pub struct AddFontParameterCharacter {
    pub character: char,
    pub data: Vec<u8>,
    pub width: usize,
    pub height: usize,
    pub advance_x: i32
}

impl AddFontParameterCharacter {
    pub(crate) fn validate(&self) -> MResult<()> {
        if Some(self.data.len()) != self.width.checked_mul(self.height) {
            return Err(Error::DataError { error: format!("width ({}) x height ({}) != data.len() ({})", self.width, self.height, self.data.len()) });
        }
        Ok(())
    }
}
