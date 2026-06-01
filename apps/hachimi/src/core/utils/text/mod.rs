//! Text-handling utilities: UTF-16 index conversion, visual length, markup-aware
//! wrapping/fitting/truncation, and tag isolation.

mod isolate;
mod truncate;
mod wrap;

pub use isolate::IsolateTags;
pub use truncate::{truncate_chars, truncate_text_il2cpp};
pub use wrap::{fit_text, fit_text_il2cpp, wrap_fit_text, wrap_fit_text_il2cpp, wrap_text, wrap_text_il2cpp};

use crate::il2cpp::{ext::Il2CppStringExt, types::Il2CppString};

pub fn char_to_utf16_index(text: &str, char_idx: usize) -> i32 {
    text.chars().take(char_idx).map(char::len_utf16).sum::<usize>() as i32
}

pub fn utf16_to_char_index(text: &str, utf16_idx: usize) -> usize {
    let mut current_utf16_pos = 0;
    let mut char_pos = 0;

    for c in text.chars() {
        if current_utf16_pos >= utf16_idx {
            break;
        }
        current_utf16_pos += c.len_utf16();
        char_pos += 1;
    }
    char_pos
}

pub fn str_visual_len(text: &str) -> usize {
    let mut count = 0;
    let mut is_in_tag = false;
    let mut chars = text.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '<' => is_in_tag = true,
            '>' => is_in_tag = false,
            '\\' => {
                if let Some(&'n') = chars.peek() {
                    chars.next();
                } else if !is_in_tag {
                    count += 1;
                }
            }
            _ => {
                if !is_in_tag {
                    count += 1;
                }
            }
        }
    }
    count
}

pub fn add_size_tag(string: &str, size: i32) -> String {
    // <size=xx>...</size>
    let mut new_str = String::with_capacity(9 + string.len() + 7);
    new_str.push_str("<size=");
    new_str.push_str(&size.to_string());
    new_str.push('>');
    new_str.push_str(string);
    new_str.push_str("</size>");
    new_str
}

// Checks for both \n and \\n
pub fn game_str_has_newline(string: *mut Il2CppString) -> bool {
    let mut got_backslash = false;
    // SAFETY: FFI / raw pointer operation required by IL2CPP interop
    for c in unsafe { (*string).as_utf16str().as_slice().iter() } {
        if got_backslash {
            if *c == 0x6E {
                // n
                return true;
            }
            got_backslash = false;
        }

        if *c == 0x0A {
            // newline
            return true;
        } else if *c == 0x5C {
            // backslash
            got_backslash = true; //
        }
    }

    false
}

#[cfg(test)]
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;

    // ── str_visual_len ──

    #[test]
    fn visual_len_plain_text() {
        assert_eq!(str_visual_len("hello"), 5);
    }

    #[test]
    fn visual_len_empty() {
        assert_eq!(str_visual_len(""), 0);
    }

    #[test]
    fn visual_len_skips_tags() {
        assert_eq!(str_visual_len("<size=16>hello</size>"), 5);
    }

    #[test]
    fn visual_len_nested_tags() {
        assert_eq!(str_visual_len("<b><i>hi</i></b>"), 2);
    }

    #[test]
    fn visual_len_backslash_n_is_zero_width() {
        assert_eq!(str_visual_len("ab\\ncd"), 4);
    }

    #[test]
    fn visual_len_backslash_other_counts() {
        // \x is not a newline escape, so \ counts as 1 char and x counts as 1 char
        assert_eq!(str_visual_len("a\\xb"), 4);
    }

    // ── char_to_utf16_index / utf16_to_char_index ──

    #[test]
    fn utf16_index_ascii() {
        assert_eq!(char_to_utf16_index("hello", 3), 3);
        assert_eq!(utf16_to_char_index("hello", 3), 3);
    }

    #[test]
    fn utf16_index_with_surrogate_pairs() {
        // 🎮 is U+1F3AE, needs 2 UTF-16 code units
        let s = "a🎮b";
        // char index 1 = after 'a', UTF-16 index = 1
        assert_eq!(char_to_utf16_index(s, 1), 1);
        // char index 2 = after 🎮, UTF-16 index = 3 (1 for 'a' + 2 for 🎮)
        assert_eq!(char_to_utf16_index(s, 2), 3);
        // reverse: UTF-16 index 3 = char index 2
        assert_eq!(utf16_to_char_index(s, 3), 2);
    }

    #[test]
    fn utf16_index_roundtrip() {
        let s = "日本語テスト";
        for i in 0..=s.chars().count() {
            let utf16 = char_to_utf16_index(s, i) as usize;
            assert_eq!(utf16_to_char_index(s, utf16), i);
        }
    }

    // ── add_size_tag ──

    #[test]
    fn add_size_tag_basic() {
        assert_eq!(add_size_tag("hello", 16), "<size=16>hello</size>");
    }
}
