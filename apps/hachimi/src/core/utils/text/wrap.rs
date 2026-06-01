//! Text wrapping and font-size fitting that preserves Unity markup tags.

use std::borrow::Cow;

use textwrap::{core::Word, wrap_algorithms, WordSeparator::UnicodeBreakProperties};

use crate::{
    core::Hachimi,
    il2cpp::{
        ext::{Il2CppStringExt, StringExt},
        types::Il2CppString,
    },
};

use super::{add_size_tag, isolate::IsolateTags};

fn custom_word_separator(line: &str) -> Box<dyn Iterator<Item = Word<'_>> + '_> {
    // Isolate tags and other text (e.g. ['test', '<size=16>', 'hello world', '</size>'])
    // Iter returns str slice and whether to separate words in the section
    // We're only breaking the string on ascii chars, so it's safe to use the bytes
    // iterator and split them based on the index.
    let mut isolate_iter = IsolateTags::new(line);

    let mut unicode_break_iter: Box<dyn Iterator<Item = Word<'_>> + '_> = Box::new(std::iter::empty());
    Box::new(std::iter::from_fn(move || {
        // Continue breaking current split
        let break_res = unicode_break_iter.next();
        if break_res.is_some() {
            return break_res;
        }

        // Advance to next (non-empty) split
        loop {
            if let Some((next_section, needs_break)) = isolate_iter.next() {
                if needs_break {
                    let mut iter = UnicodeBreakProperties.find_words(next_section);
                    let break_res = iter.next();
                    if break_res.is_some() {
                        unicode_break_iter = iter;
                        return break_res;
                    }
                } else {
                    unicode_break_iter = Box::new(std::iter::empty());
                    return Some(Word::from(next_section));
                }
            } else {
                return None;
            }
        }
    }))
}

fn custom_wrap_algorithm<'a, 'b>(words: &'b [Word<'a>], line_widths: &'b [usize]) -> Vec<&'b [Word<'a>]> {
    // Create intermediate buffer that doesn't contain formatting tags
    let mut clean_fragments = Vec::with_capacity(words.len());
    let mut removed_indices = Vec::with_capacity(words.len());
    let mut remove_offset = 0;
    for (i, word) in words.iter().enumerate() {
        let is_tag = word.starts_with("<") && word.ends_with(">");
        let is_expr = word.starts_with("$(") && word.ends_with(")");
        if is_tag || is_expr {
            removed_indices.push(i - remove_offset);
            remove_offset += 1;
            continue;
        }
        clean_fragments.push(words[i]);
    }

    let config = &Hachimi::instance().localized_data.load();
    let penalties = &config.wrapper_penalties;
    // quick escape!!!11
    let f64_line_widths = line_widths.iter().map(|w| *w as f64).collect::<Vec<_>>();
    if remove_offset == 0 {
        return wrap_algorithms::wrap_optimal_fit(words, &f64_line_widths, penalties).expect("unexpected failure");
    }

    // Wrap without formatting tags
    let wrapped =
        wrap_algorithms::wrap_optimal_fit(&clean_fragments, &f64_line_widths, penalties).expect("unexpected failure");

    // Create results with formatting tags added back
    // Note: The break word option doesn't really affect the extra long lines since
    // the individual tags are separate words (it breaks words, not lines, duh)
    let mut lines = Vec::with_capacity(wrapped.len());
    let mut start = 0;
    let mut clean_start = 0;
    let mut removed_indices_i = 0;
    for (i, line) in wrapped.iter().enumerate() {
        let mut end: usize;
        if i == wrapped.len() - 1 {
            end = words.len();
        } else {
            let clean_end = clean_start + line.len();
            end = start + line.len();
            while let Some(index) = removed_indices.get(removed_indices_i) {
                if *index >= clean_start {
                    if *index < clean_end {
                        end += 1;
                        removed_indices_i += 1;
                    } else {
                        break;
                    }
                }
            }
            clean_start = clean_end;
        }

        lines.push(&words[start..end]);
        start = end;
    }
    lines
}

pub fn wrap_text(string: &str, base_line_width: i32) -> Option<Vec<Cow<'_, str>>> {
    let config = &Hachimi::instance().localized_data.load().config;
    if !config.use_text_wrapper {
        return None;
    }
    Some(wrap_text_internal(
        string,
        base_line_width,
        config.line_width_multiplier?,
    ))
}

fn wrap_text_internal(string: &str, base_line_width: i32, line_width_multiplier: f32) -> Vec<Cow<'_, str>> {
    let line_width = (base_line_width as f32 * line_width_multiplier).round() as usize;
    let options = textwrap::Options::new(line_width)
        .word_separator(textwrap::WordSeparator::Custom(custom_word_separator))
        .wrap_algorithm(textwrap::WrapAlgorithm::Custom(custom_wrap_algorithm));
    textwrap::wrap(string, &options)
}

pub fn wrap_text_il2cpp(string: *mut Il2CppString, base_line_width: i32) -> Option<*mut Il2CppString> {
    let config = &Hachimi::instance().localized_data.load().config;
    if !config.use_text_wrapper {
        return None;
    }

    Some(
        wrap_text_internal(
            // SAFETY: FFI / raw pointer operation required by IL2CPP interop
            unsafe { &(*string).as_utf16str().to_string() },
            base_line_width,
            config.line_width_multiplier?,
        )
        .join("\n")
        .to_il2cpp_string(),
    )
}

pub fn fit_text(string: &str, base_line_width: i32, base_font_size: i32) -> Option<String> {
    let mult = Hachimi::instance().localized_data.load().config.line_width_multiplier?;
    fit_text_internal(string, base_line_width, base_font_size, mult)
}

fn fit_text_internal(
    string: &str,
    base_line_width: i32,
    base_font_size: i32,
    line_width_multiplier: f32,
) -> Option<String> {
    let line_width = base_line_width as f32 * line_width_multiplier;

    let count = string.chars().count() as f32;
    if line_width < count {
        Some(add_size_tag(
            string,
            (base_font_size as f32 * (line_width / count)) as i32,
        ))
    } else {
        None
    }
}

pub fn fit_text_il2cpp(
    string: *mut Il2CppString,
    base_line_width: i32,
    base_font_size: i32,
) -> Option<*mut Il2CppString> {
    let mult = Hachimi::instance().localized_data.load().config.line_width_multiplier?;
    if let Some(result) = fit_text_internal(
        // SAFETY: FFI / raw pointer operation required by IL2CPP interop
        unsafe { &(*string).as_utf16str().to_string() },
        base_line_width,
        base_font_size,
        mult,
    ) {
        return Some(result.to_il2cpp_string());
    }

    None
}

// WRAP IT TILL IT FITS GRAHHH BRUTE FORCE GRAHHH
pub fn wrap_fit_text(
    string: &str,
    base_line_width: i32,
    mut max_line_count: i32,
    base_font_size: i32,
) -> Option<String> {
    let config = &Hachimi::instance().localized_data.load().config;
    if !config.use_text_wrapper {
        return None;
    }
    let line_width_multiplier = config.line_width_multiplier?;

    // don't wanna mess with different sizes
    if string.contains("<size=") {
        return None;
    }

    let mut line_width = base_line_width as f32;
    let mut font_size = base_font_size as f32;

    loop {
        let wrapped = wrap_text_internal(string, line_width.round() as i32, line_width_multiplier);
        if wrapped.len() as i32 <= max_line_count {
            let new_size = font_size.round() as i32;
            let new_text = wrapped.join("\n");
            return Some(if new_size != base_font_size {
                add_size_tag(&new_text, new_size)
            } else {
                new_text
            });
        }

        let prev_max_line_count = max_line_count;
        max_line_count += 1;

        let scale = prev_max_line_count as f32 / max_line_count as f32;
        font_size *= scale;
        line_width /= scale;
    }
}

pub fn wrap_fit_text_il2cpp(
    string: *mut Il2CppString,
    base_line_width: i32,
    max_line_count: i32,
    base_font_size: i32,
) -> Option<*mut Il2CppString> {
    if Hachimi::instance().localized_data.load().config.use_text_wrapper {
        if let Some(result) = wrap_fit_text(
            // SAFETY: FFI / raw pointer operation required by IL2CPP interop
            unsafe { &(*string).as_utf16str().to_string() },
            base_line_width,
            max_line_count,
            base_font_size,
        ) {
            return Some(result.to_il2cpp_string());
        }
    }

    None
}

#[cfg(test)]
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;

    #[test]
    fn fit_text_internal_no_shrink_needed() {
        // line_width=10, text has 5 chars → no fit needed
        let result = fit_text_internal("hello", 10, 24, 1.0);
        assert!(result.is_none());
    }

    #[test]
    fn fit_text_internal_shrinks() {
        // line_width=3, text has 5 chars → needs shrink
        let result = fit_text_internal("hello", 3, 24, 1.0);
        assert!(result.is_some());
        let s = result.unwrap();
        assert!(s.starts_with("<size="));
        assert!(s.contains("hello"));
        assert!(s.ends_with("</size>"));
    }

    #[test]
    fn fit_text_internal_multiplier() {
        // With multiplier 2.0, effective width = 6, text has 5 chars → no fit
        assert!(fit_text_internal("hello", 3, 24, 2.0).is_none());
        // With multiplier 0.5, effective width = 1.5→2, text has 5 chars → fit
        assert!(fit_text_internal("hello", 3, 24, 0.5).is_some());
    }
}
