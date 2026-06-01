/*
    gettext plural forms expression parser / resolver
    Modified from https://github.com/justinas/gettext
    Limitation: It has no concept of operator precedence, plz use parentheses

    The MIT License (MIT)

    Copyright (c) 2016 Justinas Stankevicius

    Permission is hereby granted, free of charge, to any person obtaining a copy
    of this software and associated documentation files (the "Software"), to deal
    in the Software without restriction, including without limitation the rights
    to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
    copies of the Software, and to permit persons to whom the Software is
    furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all
    copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
    FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
    AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
    LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
    OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
    SOFTWARE.
*/

use self::Resolver::*;
use super::Error;

#[derive(Clone, Debug)]
pub enum Resolver {
    /// A boolean expression
    /// Use Ast::parse to get an Ast
    Expr(Ast),
    /// A function
    Function(fn(u64) -> usize),
}

impl Default for Resolver {
    fn default() -> Self {
        Resolver::Function(|_| 0)
    }
}

/// Finds the index of a pattern, outside of parenthesis
fn index_of(src: &str, pat: &str) -> Option<usize> {
    src.chars()
        .fold((None, 0, 0, 0), |(match_index, i, n_matches, paren_level), ch| {
            if let Some(x) = match_index {
                (Some(x), i, n_matches, paren_level)
            } else {
                let new_par_lvl = match ch {
                    '(' => paren_level + 1,
                    ')' => paren_level - 1,
                    _ => paren_level,
                };

                if Some(ch) == pat.chars().nth(n_matches) {
                    let length = n_matches + 1;
                    if length == pat.len() && new_par_lvl == 0 {
                        (Some(i - n_matches), i + 1, length, new_par_lvl)
                    } else {
                        (match_index, i + 1, length, new_par_lvl)
                    }
                } else {
                    (match_index, i + 1, 0, new_par_lvl)
                }
            }
        })
        .0
}

use self::Ast::*;
#[derive(Clone, Debug, PartialEq)]
pub enum Ast {
    /// A ternary expression
    /// x ? a : b
    ///
    /// the three Ast<'a> are respectively x, a and b.
    Ternary(Box<Ast>, Box<Ast>, Box<Ast>),
    /// The n variable.
    N,
    /// Integer literals.
    Integer(u64),
    /// Binary operators.
    Op(Operator, Box<Ast>, Box<Ast>),
    /// ! operator.
    Not(Box<Ast>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Operator {
    Equal,
    NotEqual,
    GreaterOrEqual,
    SmallerOrEqual,
    Greater,
    Smaller,
    And,
    Or,
    Plus,
    Minus,
    Divide,
    Multiply,
    Modulo,
}

impl Ast {
    fn resolve(&self, n: u64) -> usize {
        match *self {
            Ternary(ref cond, ref ok, ref nok) => {
                if cond.resolve(n) == 0 {
                    nok.resolve(n)
                } else {
                    ok.resolve(n)
                }
            }
            N => n as usize,
            Integer(x) => x as usize,
            Op(ref op, ref lhs, ref rhs) => match *op {
                Operator::Equal => (lhs.resolve(n) == rhs.resolve(n)) as usize,
                Operator::NotEqual => (lhs.resolve(n) != rhs.resolve(n)) as usize,
                Operator::GreaterOrEqual => (lhs.resolve(n) >= rhs.resolve(n)) as usize,
                Operator::SmallerOrEqual => (lhs.resolve(n) <= rhs.resolve(n)) as usize,
                Operator::Greater => (lhs.resolve(n) > rhs.resolve(n)) as usize,
                Operator::Smaller => (lhs.resolve(n) < rhs.resolve(n)) as usize,
                Operator::And => (lhs.resolve(n) != 0 && rhs.resolve(n) != 0) as usize,
                Operator::Or => (lhs.resolve(n) != 0 || rhs.resolve(n) != 0) as usize,
                Operator::Plus => lhs.resolve(n) + rhs.resolve(n),
                Operator::Minus => lhs.resolve(n) - rhs.resolve(n),
                Operator::Divide => lhs.resolve(n) / rhs.resolve(n),
                Operator::Multiply => lhs.resolve(n) * rhs.resolve(n),
                Operator::Modulo => lhs.resolve(n) % rhs.resolve(n),
            },
            Not(ref val) => match val.resolve(n) {
                0 => 1,
                _ => 0,
            },
        }
    }

    pub fn parse(src: &str) -> Result<Ast, Error> {
        Self::parse_parens(src.trim())
    }

    fn parse_parens(src: &str) -> Result<Ast, Error> {
        if src.starts_with('(') {
            let end = src[1..src.len() - 1]
                .chars()
                .fold((1, 2), |(level, index), ch| match (level, ch) {
                    (0, '(') => (level + 1, index + 1),
                    (0, _) => (level, index),
                    (_, '(') => (level + 1, index + 1),
                    (_, ')') => (level - 1, index + 1),
                    (_, _) => (level, index + 1),
                })
                .1;
            if end == src.len() {
                Ast::parse(src[1..src.len() - 1].trim())
            } else {
                Ast::parse_and(src.trim())
            }
        } else {
            Ast::parse_and(src.trim())
        }
    }

    fn parse_and(src: &str) -> Result<Ast, Error> {
        if let Some(i) = index_of(src, "&&") {
            Ok(Ast::Op(
                Operator::And,
                Box::new(Ast::parse(&src[0..i])?),
                Box::new(Ast::parse(&src[i + 2..])?),
            ))
        } else {
            Self::parse_or(src)
        }
    }

    fn parse_or(src: &str) -> Result<Ast, Error> {
        if let Some(i) = index_of(src, "||") {
            Ok(Ast::Op(
                Operator::Or,
                Box::new(Ast::parse(&src[0..i])?),
                Box::new(Ast::parse(&src[i + 2..])?),
            ))
        } else {
            Self::parse_ternary(src)
        }
    }

    fn parse_ternary(src: &str) -> Result<Ast, Error> {
        if let Some(i) = index_of(src, "?") {
            if let Some(l) = index_of(src, ":") {
                Ok(Ast::Ternary(
                    Box::new(Ast::parse(&src[0..i])?),
                    Box::new(Ast::parse(&src[i + 1..l])?),
                    Box::new(Ast::parse(&src[l + 1..])?),
                ))
            } else {
                Err(Error::PluralParsing)
            }
        } else {
            Self::parse_ge(src)
        }
    }

    fn parse_ge(src: &str) -> Result<Ast, Error> {
        if let Some(i) = index_of(src, ">=") {
            Ok(Ast::Op(
                Operator::GreaterOrEqual,
                Box::new(Ast::parse(&src[0..i])?),
                Box::new(Ast::parse(&src[i + 2..])?),
            ))
        } else {
            Self::parse_gt(src)
        }
    }

    fn parse_gt(src: &str) -> Result<Ast, Error> {
        if let Some(i) = index_of(src, ">") {
            Ok(Ast::Op(
                Operator::Greater,
                Box::new(Ast::parse(&src[0..i])?),
                Box::new(Ast::parse(&src[i + 1..])?),
            ))
        } else {
            Self::parse_le(src)
        }
    }

    fn parse_le(src: &str) -> Result<Ast, Error> {
        if let Some(i) = index_of(src, "<=") {
            Ok(Ast::Op(
                Operator::SmallerOrEqual,
                Box::new(Ast::parse(&src[0..i])?),
                Box::new(Ast::parse(&src[i + 2..])?),
            ))
        } else {
            Self::parse_lt(src)
        }
    }

    fn parse_lt(src: &str) -> Result<Ast, Error> {
        if let Some(i) = index_of(src, "<") {
            Ok(Ast::Op(
                Operator::Smaller,
                Box::new(Ast::parse(&src[0..i])?),
                Box::new(Ast::parse(&src[i + 1..])?),
            ))
        } else {
            Self::parse_eq(src)
        }
    }

    fn parse_eq(src: &str) -> Result<Ast, Error> {
        if let Some(i) = index_of(src, "==") {
            Ok(Ast::Op(
                Operator::Equal,
                Box::new(Ast::parse(&src[0..i])?),
                Box::new(Ast::parse(&src[i + 2..])?),
            ))
        } else {
            Self::parse_neq(src)
        }
    }

    fn parse_neq(src: &str) -> Result<Ast, Error> {
        if let Some(i) = index_of(src, "!=") {
            Ok(Ast::Op(
                Operator::NotEqual,
                Box::new(Ast::parse(&src[0..i])?),
                Box::new(Ast::parse(&src[i + 2..])?),
            ))
        } else {
            Self::parse_plus(src)
        }
    }

    fn parse_plus(src: &str) -> Result<Ast, Error> {
        if let Some(i) = index_of(src, "+") {
            Ok(Ast::Op(
                Operator::Plus,
                Box::new(Ast::parse(&src[0..i])?),
                Box::new(Ast::parse(&src[i + 1..])?),
            ))
        } else {
            Self::parse_minus(src.trim())
        }
    }

    fn parse_minus(src: &str) -> Result<Ast, Error> {
        if let Some(i) = index_of(src, "-") {
            Ok(Ast::Op(
                Operator::Minus,
                Box::new(Ast::parse(&src[0..i])?),
                Box::new(Ast::parse(&src[i + 1..])?),
            ))
        } else {
            Self::parse_divide(src.trim())
        }
    }

    fn parse_divide(src: &str) -> Result<Ast, Error> {
        if let Some(i) = index_of(src, "/") {
            Ok(Ast::Op(
                Operator::Divide,
                Box::new(Ast::parse(&src[0..i])?),
                Box::new(Ast::parse(&src[i + 1..])?),
            ))
        } else {
            Self::parse_multiply(src.trim())
        }
    }

    fn parse_multiply(src: &str) -> Result<Ast, Error> {
        if let Some(i) = index_of(src, "*") {
            Ok(Ast::Op(
                Operator::Multiply,
                Box::new(Ast::parse(&src[0..i])?),
                Box::new(Ast::parse(&src[i + 1..])?),
            ))
        } else {
            Self::parse_mod(src.trim())
        }
    }

    fn parse_mod(src: &str) -> Result<Ast, Error> {
        if let Some(i) = index_of(src, "%") {
            Ok(Ast::Op(
                Operator::Modulo,
                Box::new(Ast::parse(&src[0..i])?),
                Box::new(Ast::parse(&src[i + 1..])?),
            ))
        } else {
            Self::parse_not(src.trim())
        }
    }

    fn parse_not(src: &str) -> Result<Ast, Error> {
        if index_of(src, "!") == Some(0) {
            Ok(Ast::Not(Box::new(Ast::parse(&src[1..])?)))
        } else {
            Self::parse_int(src.trim())
        }
    }

    fn parse_int(src: &str) -> Result<Ast, Error> {
        if let Ok(x) = src.parse::<u64>() {
            Ok(Ast::Integer(x))
        } else {
            Self::parse_n(src.trim())
        }
    }

    fn parse_n(src: &str) -> Result<Ast, Error> {
        if src == "n" {
            Ok(Ast::N)
        } else {
            Err(Error::PluralParsing)
        }
    }
}

impl Resolver {
    /// Returns the number of the correct plural form
    /// for `n` objects, as defined by the rule contained in this resolver.
    pub fn resolve(&self, n: u64) -> usize {
        match *self {
            Expr(ref ast) => ast.resolve(n),
            Function(ref f) => f(n),
        }
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;

    // ── Ast::parse basic expressions ──

    #[test]
    fn parse_n() {
        let ast = Ast::parse("n").unwrap();
        assert_eq!(ast, Ast::N);
    }

    #[test]
    fn parse_integer() {
        let ast = Ast::parse("42").unwrap();
        assert_eq!(ast, Ast::Integer(42));
    }

    #[test]
    fn parse_invalid() {
        assert!(Ast::parse("xyz").is_err());
    }

    // ── Ast::resolve ──

    #[test]
    fn resolve_n() {
        let ast = Ast::parse("n").unwrap();
        assert_eq!(ast.resolve(5), 5);
    }

    #[test]
    fn resolve_integer() {
        let ast = Ast::parse("0").unwrap();
        assert_eq!(ast.resolve(999), 0);
    }

    #[test]
    fn resolve_modulo() {
        // n % 10
        let ast = Ast::parse("(n % 10)").unwrap();
        assert_eq!(ast.resolve(23), 3);
        assert_eq!(ast.resolve(10), 0);
    }

    #[test]
    fn resolve_equality() {
        let ast = Ast::parse("(n == 1)").unwrap();
        assert_eq!(ast.resolve(1), 1);
        assert_eq!(ast.resolve(2), 0);
    }

    #[test]
    fn resolve_not_equal() {
        let ast = Ast::parse("(n != 1)").unwrap();
        assert_eq!(ast.resolve(1), 0);
        assert_eq!(ast.resolve(0), 1);
    }

    #[test]
    fn resolve_ternary() {
        // n == 1 ? 0 : 1
        let ast = Ast::parse("(n == 1) ? 0 : 1").unwrap();
        assert_eq!(ast.resolve(1), 0);
        assert_eq!(ast.resolve(2), 1);
    }

    #[test]
    fn resolve_not() {
        let ast = Ast::parse("!n").unwrap();
        assert_eq!(ast.resolve(0), 1);
        assert_eq!(ast.resolve(1), 0);
        assert_eq!(ast.resolve(42), 0);
    }

    #[test]
    fn resolve_comparison_ops() {
        let gt = Ast::parse("(n > 5)").unwrap();
        assert_eq!(gt.resolve(6), 1);
        assert_eq!(gt.resolve(5), 0);

        let lt = Ast::parse("(n < 5)").unwrap();
        assert_eq!(lt.resolve(4), 1);
        assert_eq!(lt.resolve(5), 0);

        let ge = Ast::parse("(n >= 5)").unwrap();
        assert_eq!(ge.resolve(5), 1);
        assert_eq!(ge.resolve(4), 0);

        let le = Ast::parse("(n <= 5)").unwrap();
        assert_eq!(le.resolve(5), 1);
        assert_eq!(le.resolve(6), 0);
    }

    #[test]
    fn resolve_and_or() {
        let and = Ast::parse("(n && 1)").unwrap();
        assert_eq!(and.resolve(0), 0);
        assert_eq!(and.resolve(1), 1);

        let or = Ast::parse("(n || 0)").unwrap();
        assert_eq!(or.resolve(0), 0);
        assert_eq!(or.resolve(1), 1);
    }

    #[test]
    fn resolve_arithmetic() {
        let plus = Ast::parse("(n + 10)").unwrap();
        assert_eq!(plus.resolve(5), 15);

        let minus = Ast::parse("(n - 3)").unwrap();
        assert_eq!(minus.resolve(10), 7);

        let mul = Ast::parse("(n * 2)").unwrap();
        assert_eq!(mul.resolve(7), 14);

        let div = Ast::parse("(n / 3)").unwrap();
        assert_eq!(div.resolve(9), 3);
    }

    // ── Real-world gettext plural forms ──

    #[test]
    fn english_plural() {
        // nplurals=2; plural=(n != 1)
        let ast = Ast::parse("(n != 1)").unwrap();
        assert_eq!(ast.resolve(0), 1); // "0 items"
        assert_eq!(ast.resolve(1), 0); // "1 item"
        assert_eq!(ast.resolve(2), 1); // "2 items"
    }

    #[test]
    fn nested_ternary() {
        // Simple nested ternary: n==1 ? 0 : (n==2 ? 1 : 2)
        let ast = Ast::parse("(n==1) ? 0 : (n==2) ? 1 : 2").unwrap();
        assert_eq!(ast.resolve(1), 0);
        assert_eq!(ast.resolve(2), 1);
        assert_eq!(ast.resolve(3), 2);
    }

    #[test]
    fn french_plural() {
        // nplurals=2; plural=(n > 1)
        let ast = Ast::parse("(n > 1)").unwrap();
        assert_eq!(ast.resolve(0), 0);
        assert_eq!(ast.resolve(1), 0);
        assert_eq!(ast.resolve(2), 1);
    }

    // ── Resolver ──

    #[test]
    fn resolver_default() {
        let r = Resolver::default();
        assert_eq!(r.resolve(0), 0);
        assert_eq!(r.resolve(999), 0);
    }

    #[test]
    fn resolver_function() {
        let r = Resolver::Function(|n| if n == 1 { 0 } else { 1 });
        assert_eq!(r.resolve(1), 0);
        assert_eq!(r.resolve(2), 1);
    }

    #[test]
    fn resolver_expr() {
        let ast = Ast::parse("(n != 1)").unwrap();
        let r = Resolver::Expr(ast);
        assert_eq!(r.resolve(1), 0);
        assert_eq!(r.resolve(2), 1);
    }

    // ── index_of helper ──

    #[test]
    fn index_of_basic() {
        assert_eq!(super::index_of("a && b", "&&"), Some(2));
        assert_eq!(super::index_of("hello", "&&"), None);
    }

    #[test]
    fn index_of_respects_parens() {
        // The && inside parens should be skipped
        assert_eq!(super::index_of("(a && b) && c", "&&"), Some(9));
    }
}
