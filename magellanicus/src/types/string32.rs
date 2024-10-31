use std::cmp::Ordering;
use std::fmt::{Debug, Display, Formatter};
use std::hash::{Hash, Hasher};

/// Describes a string that can be represented in 32 bytes or fewer.
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct String32 {
    string_data: [u8; 32],
    string_length: usize
}

impl String32 {
    pub fn as_str(&self) -> &str {
        unsafe { std::str::from_utf8_unchecked(self.as_bytes()) }
    }
    pub fn len(&self) -> usize {
        self.string_length
    }
    pub fn as_bytes(&self) -> &[u8] {
        &self.string_data[..self.string_length]
    }
}

impl Display for String32 {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl AsRef<str> for String32 {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl AsRef<[u8]> for String32 {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl PartialEq<&str> for String32 {
    fn eq(&self, other: &&str) -> bool {
        self.as_str().eq(*other)
    }
}

impl PartialOrd<&str> for String32 {
    fn partial_cmp(&self, other: &&str) -> Option<Ordering> {
        self.as_str().partial_cmp(*other)
    }
}

impl PartialOrd<String32> for String32 {
    fn partial_cmp(&self, other: &String32) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for String32 {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl Hash for String32 {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_str().hash(state)
    }
}

impl TryFrom<&str> for String32 {
    type Error = &'static str;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let bytes = value.as_bytes();
        let string_length = bytes.len();
        if string_length > 32 {
            return Err("string is too long")
        }
        let mut string_data = [0u8;32];
        string_data[..string_length].copy_from_slice(bytes);
        Ok(Self {
            string_data,
            string_length
        })
    }
}

impl Debug for String32 {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}
