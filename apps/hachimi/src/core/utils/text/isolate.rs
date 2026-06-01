//! `IsolateTags`: splits a string into runs of plain text vs. Unity formatting
//! tags / template expressions, used to keep markup intact during wrapping.

pub struct IsolateTags<'a> {
    s: &'a str,
    bytes: std::str::Bytes<'a>,
    i: usize,
    current_byte: Option<u8>,
}

impl<'a> IsolateTags<'a> {
    pub fn new(s: &'a str) -> Self {
        let mut bytes = s.bytes();
        Self {
            current_byte: bytes.next(),
            s,
            bytes,
            i: 0,
        }
    }
}

impl<'a> Iterator for IsolateTags<'a> {
    type Item = (&'a str, bool);

    fn next(&mut self) -> Option<Self::Item> {
        self.current_byte?;

        let start = self.i;
        // Unity tags
        let mut tag_start = 0;
        let mut in_tag = false;
        let mut in_closing_tag = false;
        let mut expecting_tag_name = false;
        // Template expressions
        let mut expecting_expr_open = false;
        let mut in_expression = false;

        while let Some(c) = self.current_byte {
            if in_tag {
                match c {
                    b'>' | b'=' | b' ' => 'tag_name_end: {
                        if expecting_tag_name {
                            if !in_closing_tag {
                                // Check for a matching closing tag after
                                let tag_name = &self.s[tag_start + 1..self.i];
                                let mut closing_tag = String::with_capacity(3 + tag_name.len());
                                closing_tag += "</";
                                closing_tag += tag_name;
                                closing_tag += ">";
                                if !self.s[self.i..].contains(&closing_tag) {
                                    in_tag = false;
                                    break 'tag_name_end;
                                }
                            }
                            expecting_tag_name = false;
                        }

                        if c == b'>' {
                            // in_tag = false;
                            loop {
                                self.i += 1;
                                self.current_byte = self.bytes.next();
                                if let Some(c) = self.current_byte {
                                    // Capture any whitespace that comes right after it
                                    if char::from(c).is_whitespace() {
                                        continue;
                                    }
                                }
                                break;
                            }
                            return Some((&self.s[start..self.i], false));
                        } else if in_closing_tag {
                            // Invalid character
                            in_tag = false;
                        }
                    }
                    b'/' => {
                        if self.i == tag_start + 1 {
                            in_closing_tag = true;
                        } else if expecting_tag_name {
                            in_tag = false;
                        }
                    }
                    _ => {
                        if expecting_tag_name && !char::from(c).is_ascii_alphabetic() {
                            in_tag = false;
                        }
                    }
                }
            } else if in_expression {
                if c == b')' {
                    if !self.s[self.i..].contains(")") {
                        in_expression = false;
                    } else {
                        loop {
                            self.i += 1;
                            self.current_byte = self.bytes.next();
                            if let Some(c) = self.current_byte {
                                if char::from(c).is_whitespace() {
                                    continue;
                                }
                            }
                            break;
                        }
                        return Some((&self.s[start..self.i], false));
                    }
                }
            } else if c == b'<' {
                if start == self.i {
                    in_tag = true;
                    expecting_tag_name = true;
                    tag_start = self.i;
                } else {
                    break;
                }
            } else if c == b'$' {
                expecting_expr_open = true;
            } else if c == b'(' {
                if expecting_expr_open {
                    if self.i != start + 1 {
                        self.i -= 1;
                        self.bytes = self.s.bytes();
                        self.current_byte = self.bytes.nth(self.i);
                        break;
                    }
                    in_expression = true;
                    expecting_expr_open = false;
                }
            } else if expecting_expr_open {
                expecting_expr_open = false;
            }

            self.i += 1;
            self.current_byte = self.bytes.next();
        }

        Some((&self.s[start..self.i], true))
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;

    #[test]
    fn isolate_tags_plain_text() {
        let parts: Vec<_> = IsolateTags::new("hello world").collect();
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0], ("hello world", true));
    }

    #[test]
    fn isolate_tags_with_tag() {
        let parts: Vec<_> = IsolateTags::new("<size=16>hello</size>").collect();
        // Should separate tags from text
        assert!(parts.len() >= 2);
        // At least one part should be non-breakable (tag)
        assert!(parts.iter().any(|(_, is_text)| !is_text));
    }

    #[test]
    fn isolate_tags_empty() {
        let parts: Vec<_> = IsolateTags::new("").collect();
        assert!(parts.is_empty());
    }

    #[test]
    fn isolate_tags_only_text() {
        let parts: Vec<_> = IsolateTags::new("no tags here").collect();
        assert_eq!(parts.len(), 1);
        assert!(parts[0].1); // is text
    }

    #[test]
    fn isolate_tags_template_expr() {
        let parts: Vec<_> = IsolateTags::new("hello $(expr) world").collect();
        // Should isolate the $(expr) part
        assert!(parts.len() >= 2);
    }
}
