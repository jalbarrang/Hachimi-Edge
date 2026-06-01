//! Width-aware character truncation with optional ellipsis.

use unicode_width::UnicodeWidthChar;

use crate::{
    core::Hachimi,
    il2cpp::{
        ext::{Il2CppStringExt, StringExt},
        types::Il2CppString,
    },
};

fn truncate_chars_internal(
    mut chars: impl Iterator<Item = char>,
    mut width: usize,
    ellipsis: bool,
    line_width_multiplier: f32,
) -> Option<Vec<char>> {
    width = (width as f32 * line_width_multiplier).round() as usize;

    let reserved_width = if ellipsis { width.saturating_sub(1) } else { width };
    let mut v = Vec::with_capacity(width); // it's not the actual max size but it's a good starting point
    let mut total_width = 0;
    let mut dropped_char = None;
    for c in chars.by_ref() {
        let char_width = c.width().unwrap_or(0);
        if char_width == 0 {
            v.push(c);
            continue;
        };

        let next_total_width = total_width + char_width;
        if next_total_width > reserved_width {
            dropped_char = Some(c);
            break;
        }

        v.push(c);

        total_width = next_total_width;
        if total_width == reserved_width {
            break;
        }
    }

    if ellipsis {
        // Don't truncate if adding the last dropped or next char would result in the expected width
        let has_next_char = if let Some(c) = dropped_char {
            if total_width + c.width().unwrap_or(0) <= width && chars.next().is_none() {
                return None;
            }
            true
        }
        // doesn't handle control characters correctly but whatever they are never used here
        else if let Some(c) = chars.next() {
            if c.width().unwrap_or(0) <= 1 && chars.next().is_none() {
                return None;
            }
            true
        } else {
            false
        };

        // Add ellipsis
        return if has_next_char {
            v.push('…');
            Some(v)
        } else {
            None
        };
    }

    if dropped_char.is_some() || chars.next().is_some() {
        Some(v)
    } else {
        None
    }
}

pub fn truncate_chars(chars: impl Iterator<Item = char>, width: usize, ellipsis: bool) -> Option<Vec<char>> {
    let line_width_multiplier = Hachimi::instance().localized_data.load().config.line_width_multiplier?;
    truncate_chars_internal(chars, width, ellipsis, line_width_multiplier)
}

pub fn truncate_text_il2cpp(string: *mut Il2CppString, width: usize, ellipsis: bool) -> Option<*mut Il2CppString> {
    let line_width_multiplier = Hachimi::instance().localized_data.load().config.line_width_multiplier?;
    truncate_chars_internal(
        // SAFETY: FFI / raw pointer operation required by IL2CPP interop
        unsafe { (*string).as_utf16str().chars() },
        width,
        ellipsis,
        line_width_multiplier,
    )
    .map(|chars| chars.iter().collect::<String>().to_il2cpp_string())
}

#[cfg(test)]
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;

    #[test]
    fn truncate_no_truncation_needed() {
        let result = truncate_chars_internal("hi".chars(), 10, false, 1.0);
        assert!(result.is_none());
    }

    #[test]
    fn truncate_basic() {
        let result = truncate_chars_internal("hello world".chars(), 5, false, 1.0);
        assert!(result.is_some());
        let v: String = result.unwrap().into_iter().collect();
        assert_eq!(v, "hello");
    }

    #[test]
    fn truncate_with_ellipsis() {
        let result = truncate_chars_internal("hello world".chars(), 6, true, 1.0);
        assert!(result.is_some());
        let v: String = result.unwrap().into_iter().collect();
        assert!(v.ends_with('…'));
        // 5 chars + ellipsis = 6 width
    }

    #[test]
    fn truncate_ellipsis_not_added_when_fits() {
        // "hi" has 2 chars, width=5 → no truncation
        let result = truncate_chars_internal("hi".chars(), 5, true, 1.0);
        assert!(result.is_none());
    }

    #[test]
    fn truncate_multiplier_expands_width() {
        // "hello world" = 11 chars, width=6, mult=2.0 → effective=12 → no truncation
        let result = truncate_chars_internal("hello world".chars(), 6, false, 2.0);
        assert!(result.is_none());
    }
}
