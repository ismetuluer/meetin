//! Yalnızca bellekte tutulan, en fazla 64 karakterlik tuş arabelleği (§7.4).
//!
//! Bu arabellek ASLA diske veya loga yazılmaz. Pencere değişimi, fare tıklaması
//! veya yön tuşlarında temizlenir.

use std::collections::VecDeque;

pub const MAX_BUFFER_CHARS: usize = 64;

#[derive(Default)]
pub struct KeyBuffer {
    chars: VecDeque<char>,
}

impl KeyBuffer {
    pub fn new() -> Self {
        Self {
            chars: VecDeque::with_capacity(MAX_BUFFER_CHARS),
        }
    }

    pub fn push(&mut self, c: char) {
        if self.chars.len() >= MAX_BUFFER_CHARS {
            self.chars.pop_front();
        }
        self.chars.push_back(c);
    }

    pub fn backspace(&mut self) {
        self.chars.pop_back();
    }

    pub fn clear(&mut self) {
        self.chars.clear();
    }

    pub fn len(&self) -> usize {
        self.chars.len()
    }

    pub fn is_empty(&self) -> bool {
        self.chars.is_empty()
    }

    pub fn as_string(&self) -> String {
        self.chars.iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn respects_64_char_limit_with_turkish_chars() {
        let mut buf = KeyBuffer::new();
        for _ in 0..70 {
            buf.push('ş');
        }
        assert_eq!(buf.len(), MAX_BUFFER_CHARS);
        assert_eq!(buf.as_string().chars().count(), MAX_BUFFER_CHARS);
    }

    #[test]
    fn push_backspace_and_clear() {
        let mut buf = KeyBuffer::new();
        for c in "CİPY".chars() {
            buf.push(c);
        }
        assert_eq!(buf.as_string(), "CİPY");
        buf.backspace();
        assert_eq!(buf.as_string(), "CİP");
        buf.push('Y');
        assert_eq!(buf.as_string(), "CİPY");
        buf.clear();
        assert!(buf.is_empty());
        assert_eq!(buf.as_string(), "");
    }
}
