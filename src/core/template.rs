/*
    Simple templating language parser/evaluator for localization strings.

    Syntax:
    - Filter: $(filter_name arg1 arg2 arg3 ...)
      Loosely based on Bash command substitution syntax.

    More expression types might be added later, but the filter expression
    is already suitable for most if not all use cases.
*/
use fnv::FnvHashMap;

pub enum Token {
    Identifier(String),
    NumberLit(f64),
    StringLit(String),
}

pub type Filter = fn(args: &[Token]) -> Option<String>;

pub trait Context {
    fn on_filter_eval(&mut self, name: &str, args: &[Token]) -> Option<String>;
}

struct EmptyContext();

impl Context for EmptyContext {
    fn on_filter_eval(&mut self, _name: &str, _args: &[Token]) -> Option<String> {
        None
    }
}

struct FilterRemovalContext();

impl Context for FilterRemovalContext {
    fn on_filter_eval(&mut self, _name: &str, _args: &[Token]) -> Option<String> {
        Some(String::new())
    }
}

pub struct Parser {
    filters: FnvHashMap<String, Filter>,
}

impl Parser {
    pub fn new(filters_: &[(&str, Filter)]) -> Parser {
        let mut filters = FnvHashMap::default();
        for (name, filter) in filters_ {
            filters.insert(name.to_string(), filter.to_owned());
        }

        Parser { filters }
    }

    fn eval_filter(&self, tokens: &Vec<Token>, context: &mut impl Context) -> Option<String> {
        if tokens.is_empty() {
            return None;
        }

        if let Token::Identifier(filter_name) = tokens.first().expect("non-empty collection") {
            let args = &tokens.as_slice()[1..];
            let context_res = context.on_filter_eval(filter_name, args);
            if context_res.is_some() {
                return context_res;
            } else if let Some(filter) = self.filters.get(filter_name) {
                return filter(&tokens.as_slice()[1..]);
            }
        }

        None
    }

    fn parse_token(input: &str) -> Option<Token> {
        let mut iter = input.chars();
        let start_char = iter.next().expect("unexpected failure"); // guaranteed to have at least one char
        let end_char = iter.last();

        if start_char == '\'' && end_char.is_some() && end_char.expect("unexpected failure") == '\'' {
            return Some(Token::StringLit(input[1..input.len() - 1].replace("\\'", "'")));
        }

        if start_char.is_numeric() {
            return if let Ok(number) = input.parse::<f64>() {
                Some(Token::NumberLit(number))
            } else if let Ok(number) = input.replace(",", "").parse::<f64>() {
                // Allow commas
                // (not doing in the initial parse; the idea being that numbers with commas are not common)
                Some(Token::NumberLit(number))
            } else {
                None
            };
        }

        if input.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Some(Token::Identifier(input.to_owned()));
        }

        None
    }

    pub fn eval(&self, input: &str) -> String {
        self.eval_with_context(input, &mut EmptyContext {})
    }

    pub fn eval_with_context(&self, input: &str, context: &mut impl Context) -> String {
        let mut output: Vec<u8> = Vec::with_capacity(input.len());

        let mut start_expr = false;
        let mut in_filter = false;
        let mut checkpoint: usize = 0;
        let mut tokens: Vec<Token> = Vec::new();
        let mut token_start: usize = 0;
        let mut in_string = false;
        let mut start_escape = false;

        // Iterate through the bytes directly for the sake of simplicity
        // (it's also faster than going through char())
        // A caveat is that the "syntax parsing" portion of the parser has
        // no knowledge of Unicode characters; it doesn't need to anyways, UTF-8
        // sequences do not conflict with normal ascii characters.
        for (i, c) in input.bytes().enumerate() {
            output.push(c);

            if in_filter {
                // Continue if string char is escaped
                if start_escape {
                    start_escape = false;
                    continue;
                }

                // Check separator and expr close
                match c {
                    b')' => 'filter_close: {
                        if in_string {
                            break 'filter_close;
                        }

                        // Parse token (if it hasnt been terminated by a trailing separator yet)
                        if token_start != 0 {
                            let res = Self::parse_token(&input[token_start..i]);
                            if let Some(token) = res {
                                tokens.push(token);
                                token_start = 0;
                            } else {
                                warn!("Invalid token in '{}' (at pos {})", input, token_start);
                                token_start = 0;
                                tokens.clear();
                                in_filter = false;
                                break 'filter_close;
                            }
                        }

                        if let Some(res) = self.eval_filter(&tokens, context) {
                            output.truncate(checkpoint);
                            output.extend(res.bytes());
                        } else {
                            warn!("Filter evaluation failed in '{}' (at pos {})", input, i);
                        }

                        tokens.clear();
                        in_filter = false;
                    }

                    b' ' | b'\n' | b'\r' | b'\t' => {
                        if !in_string && token_start != 0 {
                            let res = Self::parse_token(&input[token_start..i]);
                            if let Some(token) = res {
                                tokens.push(token);
                            } else {
                                warn!("Invalid token in '{}' (at pos {})", input, token_start);
                                tokens.clear();
                                in_filter = false;
                            }
                            token_start = 0;
                        }
                    }

                    b'\'' => {
                        if token_start == 0 {
                            token_start = i;
                            in_string = true;
                        } else {
                            in_string = false;
                        }
                    }

                    b'\\' => {
                        if in_string {
                            start_escape = true;
                        }
                    }

                    _ => {
                        if token_start == 0 {
                            token_start = i;
                        }
                    }
                }
                continue;
            }

            if start_expr {
                // Check expression opening
                if c == b'(' {
                    // Filter expression
                    in_filter = true;
                }
                start_expr = false;
                continue;
            }

            // Check for expression start
            if c == b'$' {
                start_expr = true;
                checkpoint = output.len() - 1; // before the starting char
            }
        }

        // SAFETY: FFI / raw pointer operation required by IL2CPP interop
        unsafe { String::from_utf8_unchecked(output) }
    }

    /// Evaluate the template with a context that returns an empty string on any filter expr
    pub fn remove_filters(&self, input: &str) -> String {
        self.eval_with_context(input, &mut FilterRemovalContext {})
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;

    fn make_parser() -> Parser {
        Parser::new(&[
            ("upper", |args: &[Token]| {
                if let Some(Token::StringLit(s)) = args.first() {
                    Some(s.to_uppercase())
                } else {
                    None
                }
            }),
            ("add", |args: &[Token]| {
                if args.len() >= 2 {
                    if let (Token::NumberLit(a), Token::NumberLit(b)) = (&args[0], &args[1]) {
                        return Some(format!("{}", (*a as i64) + (*b as i64)));
                    }
                }
                None
            }),
        ])
    }

    // ── Plain text (no expressions) ──

    #[test]
    fn plain_text_passthrough() {
        let p = make_parser();
        assert_eq!(p.eval("hello world"), "hello world");
    }

    #[test]
    fn empty_string() {
        let p = make_parser();
        assert_eq!(p.eval(""), "");
    }

    // ── Filter expressions ──

    #[test]
    fn filter_string_arg() {
        let p = make_parser();
        assert_eq!(p.eval("$(upper 'hello')"), "HELLO");
    }

    #[test]
    fn filter_number_args() {
        let p = make_parser();
        assert_eq!(p.eval("$(add 3 4)"), "7");
    }

    #[test]
    fn filter_embedded_in_text() {
        let p = make_parser();
        assert_eq!(p.eval("result: $(add 1 2) done"), "result: 3 done");
    }

    #[test]
    fn multiple_filters() {
        let p = make_parser();
        assert_eq!(p.eval("$(upper 'a') and $(add 10 20)"), "A and 30");
    }

    #[test]
    fn dollar_without_paren_passthrough() {
        let p = make_parser();
        assert_eq!(p.eval("cost $5"), "cost $5");
    }

    // ── Token parsing ──

    #[test]
    fn parse_token_number() {
        let tok = Parser::parse_token("42");
        assert!(matches!(tok, Some(Token::NumberLit(n)) if (n - 42.0).abs() < f64::EPSILON));
    }

    #[test]
    fn parse_token_number_with_commas() {
        let tok = Parser::parse_token("1,000");
        assert!(matches!(tok, Some(Token::NumberLit(n)) if (n - 1000.0).abs() < f64::EPSILON));
    }

    #[test]
    fn parse_token_string_lit() {
        let tok = Parser::parse_token("'hello'");
        assert!(matches!(tok, Some(Token::StringLit(ref s)) if s == "hello"));
    }

    #[test]
    fn parse_token_string_with_escaped_quote() {
        let tok = Parser::parse_token("'it\\'s'");
        assert!(matches!(tok, Some(Token::StringLit(ref s)) if s == "it's"));
    }

    #[test]
    fn parse_token_identifier() {
        let tok = Parser::parse_token("my_filter");
        assert!(matches!(tok, Some(Token::Identifier(ref s)) if s == "my_filter"));
    }

    // ── Context-based evaluation ──

    #[test]
    fn context_overrides_filter() {
        struct MyCtx;
        impl Context for MyCtx {
            fn on_filter_eval(&mut self, name: &str, _args: &[Token]) -> Option<String> {
                if name == "upper" {
                    Some("CONTEXT_WINS".into())
                } else {
                    None
                }
            }
        }
        let p = make_parser();
        assert_eq!(p.eval_with_context("$(upper 'x')", &mut MyCtx), "CONTEXT_WINS");
    }

    // ── remove_filters ──

    #[test]
    fn remove_filters_strips_expressions() {
        let p = make_parser();
        assert_eq!(p.remove_filters("hello $(upper 'world') end"), "hello  end");
    }

    #[test]
    fn remove_filters_plain_text_unchanged() {
        let p = make_parser();
        assert_eq!(p.remove_filters("no filters here"), "no filters here");
    }
}
