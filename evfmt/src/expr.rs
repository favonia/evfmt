//! Character-set expression language for formatter policy.
//!
//! This module owns the expression language as a semantic concept:
//!
//! - the [`Expr`] tree and named-set identifiers
//! - membership evaluation against characters
//! - user-facing help text for the language
//!
//! Parsing lives in the private `parser` submodule and is re-exported here so
//! callers can continue using [`crate::expr::parse`].

use std::fmt;

use crate::unicode::{self, DefaultSide};

mod parser;

pub use parser::{ParseError, ParseResult, ParseWarning, parse, parse_expr_only};

/// Help text for the expression language, printed by `--help-expression`.
pub const EXPRESSION_HELP: &str = "\
Expression Language
===================

Both --prefer-bare-for and --treat-bare-as-text-for accept an expression that
describes a set of characters.
\
Here \"variation-sequence character\" means a character listed in Unicode's
emoji-variation-sequences.txt.

Atoms
-----
  all               All variation-sequence characters
  none              No character
  ascii             Variation-sequence characters in U+0000-U+007F
  emoji-defaults    Variation-sequence characters whose Unicode default is emoji
  rights-marks      Characters currently listed in Unicode's
                    emoji-variation-sequences.txt as rights marks:
                    ©︎ (U+00A9), ®︎ (U+00AE), ™︎ (U+2122)
                    This set may change when evfmt upgrades Unicode.
  arrows            Characters currently listed in Unicode's
                    emoji-variation-sequences.txt as arrows:
                    ↔︎ (U+2194), ↕︎ (U+2195), ↖︎ (U+2196), ↗︎ (U+2197),
                    ↘︎ (U+2198), ↙︎ (U+2199), ↩︎ (U+21A9), ↪︎ (U+21AA),
                    ➡︎ (U+27A1), ⤴︎ (U+2934), ⤵︎ (U+2935), ⬅︎ (U+2B05),
                    ⬆︎ (U+2B06), ⬇︎ (U+2B07)
                    This set may change when evfmt upgrades Unicode.
  card-suits        Characters currently listed in Unicode's
                    emoji-variation-sequences.txt as card suits:
                    ♠︎ (U+2660), ♣︎ (U+2663), ♥︎ (U+2665), ♦︎ (U+2666)
                    This set may change when evfmt upgrades Unicode.

Literals
--------
  u(00A9)           Single code point (4-6 hex digits)
  '#'               Single character; selectors inside quotes do not matter
  \"#*\"              Union of characters (sugar for union('#', '*');
                    selectors inside quotes do not matter)

Combinators
-----------
  union(e1, e2, ...)              Match any (1+ arguments)
  subtract(base, ex1, ex2, ...)   Match base but not excluded (2+ arguments)
  except(e)                       Top-level only; sugar for subtract(all, e)

Examples
--------
  --prefer-bare-for='ascii'
  --treat-bare-as-text-for='ascii'
  --prefer-bare-for='subtract(ascii, \"#*\")'
  --treat-bare-as-text-for='union(ascii, rights-marks)'
  --prefer-bare-for='union(ascii, rights-marks)'
";

/// Identifier for a named set in the expression language.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamedSetId {
    /// ASCII characters (U+0000-U+007F).
    Ascii,
    /// Variation-sequence characters whose Unicode default side is emoji.
    EmojiDefaults,
    /// ©️ (U+00A9), ®️ (U+00AE), ™️ (U+2122).
    RightsMarks,
    /// Arrow characters used by the formatter policy docs.
    Arrows,
    /// ♠️ (U+2660), ♣️ (U+2663), ♥️ (U+2665), ♦️ (U+2666).
    CardSuits,
}

impl NamedSetId {
    /// Check if a character belongs to this named set.
    #[must_use]
    pub fn matches(&self, ch: char) -> bool {
        match self {
            NamedSetId::Ascii => ch.is_ascii(),
            NamedSetId::EmojiDefaults => unicode::variation_sequence_info(ch)
                .is_some_and(|info| info.default_side == DefaultSide::Emoji),
            NamedSetId::RightsMarks => matches!(ch, '\u{00A9}' | '\u{00AE}' | '\u{2122}'),
            NamedSetId::Arrows => matches!(
                ch,
                '\u{2194}'
                    | '\u{2195}'
                    | '\u{2196}'
                    | '\u{2197}'
                    | '\u{2198}'
                    | '\u{2199}'
                    | '\u{21A9}'
                    | '\u{21AA}'
                    | '\u{27A1}'
                    | '\u{2934}'
                    | '\u{2935}'
                    | '\u{2B05}'
                    | '\u{2B06}'
                    | '\u{2B07}'
            ),
            NamedSetId::CardSuits => {
                matches!(ch, '\u{2660}' | '\u{2663}' | '\u{2665}' | '\u{2666}')
            }
        }
    }
}

impl fmt::Display for NamedSetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NamedSetId::Ascii => write!(f, "ascii"),
            NamedSetId::EmojiDefaults => write!(f, "emoji-defaults"),
            NamedSetId::RightsMarks => write!(f, "rights-marks"),
            NamedSetId::Arrows => write!(f, "arrows"),
            NamedSetId::CardSuits => write!(f, "card-suits"),
        }
    }
}

/// An expression in the set DSL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    /// Matches every character.
    All,
    /// Matches no character.
    None,
    /// Matches characters in a named set.
    NamedSet(NamedSetId),
    /// Matches a single character.
    CodePoint(char),
    /// Matches if any sub-expression matches.
    Union(Vec<Expr>),
    /// Matches characters in `base` that are not in any excluded set.
    Subtract(Box<Expr>, Vec<Expr>),
}

impl Expr {
    /// Evaluate whether this expression matches the given character.
    #[must_use]
    pub fn matches(&self, ch: char) -> bool {
        match self {
            Expr::All => true,
            Expr::None => false,
            Expr::NamedSet(id) => id.matches(ch),
            Expr::CodePoint(cp) => ch == *cp,
            Expr::Union(exprs) => exprs.iter().any(|e| e.matches(ch)),
            Expr::Subtract(base, excluded) => {
                base.matches(ch) && !excluded.iter().any(|e| e.matches(ch))
            }
        }
    }
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expr::All => write!(f, "all"),
            Expr::None => write!(f, "none"),
            Expr::NamedSet(id) => write!(f, "{id}"),
            Expr::CodePoint(ch) => write!(f, "u({:04X})", *ch as u32),
            Expr::Union(exprs) => {
                write!(f, "union(")?;
                for (i, e) in exprs.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{e}")?;
                }
                write!(f, ")")
            }
            Expr::Subtract(base, excluded) => {
                write!(f, "subtract({base}")?;
                for e in excluded {
                    write!(f, ", {e}")?;
                }
                write!(f, ")")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    fn parse_ok(input: &str) -> Expr {
        parse(input).unwrap().expr
    }

    #[test]
    fn test_matches_all() {
        let expr = parse_ok("all");
        assert!(expr.matches('#'));
        assert!(expr.matches('\u{00A9}'));
    }

    #[test]
    fn test_matches_none() {
        let expr = parse_ok("none");
        assert!(!expr.matches('#'));
        assert!(!expr.matches('\u{00A9}'));
    }

    #[test]
    fn test_matches_ascii() {
        let expr = parse_ok("ascii");
        assert!(expr.matches('#'));
        assert!(expr.matches('A'));
        assert!(!expr.matches('\u{00A9}'));
    }

    #[test]
    fn test_matches_emoji_defaults() {
        let expr = parse_ok("emoji-defaults");
        assert!(expr.matches('\u{2728}'));
        assert!(!expr.matches('\u{00A9}'));
        assert!(!expr.matches('#'));
        assert!(!expr.matches('A'));
    }

    #[test]
    fn test_matches_rights_marks() {
        let expr = parse_ok("rights-marks");
        assert!(expr.matches('\u{00A9}'));
        assert!(expr.matches('\u{00AE}'));
        assert!(expr.matches('\u{2122}'));
        assert!(!expr.matches('A'));
        assert!(!expr.matches('\u{2660}'));
    }

    #[test]
    fn test_matches_arrows() {
        let expr = parse_ok("arrows");
        assert!(expr.matches('\u{2194}'));
        assert!(expr.matches('\u{27A1}'));
        assert!(expr.matches('\u{2B05}'));
        assert!(!expr.matches('A'));
        assert!(!expr.matches('\u{2660}'));
    }

    #[test]
    fn test_matches_card_suits() {
        let expr = parse_ok("card-suits");
        assert!(expr.matches('\u{2660}'));
        assert!(expr.matches('\u{2663}'));
        assert!(expr.matches('\u{2665}'));
        assert!(expr.matches('\u{2666}'));
        assert!(!expr.matches('A'));
        assert!(!expr.matches('\u{00A9}'));
    }

    #[test]
    fn test_matches_subtract_binary_with_space_before_paren() {
        let expr = parse_ok("subtract (all, ascii)");
        assert!(!expr.matches('A'));
        assert!(expr.matches('\u{00A9}'));
    }

    #[test]
    fn test_matches_subtract_binary() {
        let expr = parse_ok("subtract(all, ascii)");
        assert!(!expr.matches('A'));
        assert!(expr.matches('\u{00A9}'));
    }

    #[test]
    fn test_matches_subtract_variadic() {
        let expr = parse_ok("subtract(all, ascii, emoji-defaults)");
        assert!(!expr.matches('A'));
        assert!(!expr.matches('\u{2728}'));
        assert!(expr.matches('\u{00A9}'));
    }

    #[test]
    fn test_matches_union() {
        let expr = parse_ok("union(u(0023), u(002A))");
        assert!(expr.matches('#'));
        assert!(expr.matches('*'));
        assert!(!expr.matches('A'));
    }

    #[test]
    fn test_matches_union_with_space_before_paren() {
        let expr = parse_ok("union (u(0023), u(002A))");
        assert!(expr.matches('#'));
        assert!(expr.matches('*'));
        assert!(!expr.matches('A'));
    }

    #[test]
    fn test_matches_u_with_space_before_paren() {
        let expr = parse_ok("u (0023)");
        assert!(expr.matches('#'));
        assert!(!expr.matches('A'));
    }

    #[test]
    fn test_matches_single_quoted() {
        let expr = parse_ok("'#'");
        assert!(expr.matches('#'));
        assert!(!expr.matches('A'));
    }

    #[test]
    fn test_matches_single_quoted_with_variant_selector() {
        let expr = parse_ok("'#\u{FE0F}'");
        assert!(expr.matches('#'));
        assert!(!expr.matches('\u{FE0F}'));
        assert!(!expr.matches('A'));
    }

    #[test]
    fn test_matches_double_quoted() {
        let expr = parse_ok("\"#*\"");
        assert!(expr.matches('#'));
        assert!(expr.matches('*'));
        assert!(!expr.matches('A'));
    }

    #[test]
    fn test_matches_double_quoted_with_variant_selectors() {
        let expr = parse_ok("\"#\u{FE0F}*\u{FE0E}\"");
        assert!(expr.matches('#'));
        assert!(expr.matches('*'));
        assert!(!expr.matches('\u{FE0E}'));
        assert!(!expr.matches('\u{FE0F}'));
        assert!(!expr.matches('A'));
    }

    #[test]
    fn test_matches_except() {
        let expr = parse_ok("except(ascii)");
        assert!(!expr.matches('A'));
        assert!(expr.matches('\u{00A9}'));
    }

    #[test]
    fn test_matches_except_with_space_before_paren() {
        let expr = parse_ok("except (ascii)");
        assert!(!expr.matches('A'));
        assert!(expr.matches('\u{00A9}'));
    }

    #[test]
    fn test_display_round_trip() {
        let exprs = [
            "all",
            "none",
            "ascii",
            "emoji-defaults",
            "rights-marks",
            "arrows",
            "card-suits",
            "u(0023)",
            "union(u(0023), u(002A))",
            "subtract(all, ascii)",
            "subtract(all, ascii, emoji-defaults)",
            "subtract(all, union(ascii, emoji-defaults))",
        ];
        for s in &exprs {
            let parsed = parse_ok(s);
            let displayed = format!("{parsed}");
            let reparsed = parse_ok(&displayed);
            assert_eq!(parsed, reparsed, "round-trip failed for {s:?}");
        }
    }

    #[test]
    fn test_help_text_mentions_all_named_sets() {
        let named_sets = [
            NamedSetId::Ascii,
            NamedSetId::EmojiDefaults,
            NamedSetId::RightsMarks,
            NamedSetId::Arrows,
            NamedSetId::CardSuits,
        ];
        for id in &named_sets {
            let name = format!("{id}");
            assert!(
                EXPRESSION_HELP.contains(&name),
                "EXPRESSION_HELP missing named set {name:?}"
            );
        }
    }

    #[test]
    fn test_help_text_mentions_combinators() {
        for keyword in ["union(", "subtract(", "except("] {
            assert!(
                EXPRESSION_HELP.contains(keyword),
                "EXPRESSION_HELP missing combinator {keyword:?}"
            );
        }
    }

    #[test]
    fn test_help_text_mentions_literal_forms() {
        for form in ["u(", "'", "\""] {
            assert!(
                EXPRESSION_HELP.contains(form),
                "EXPRESSION_HELP missing literal form {form:?}"
            );
        }
    }

    #[test]
    fn test_help_text_mentions_keywords() {
        for keyword in ["all", "none"] {
            assert!(
                EXPRESSION_HELP.contains(keyword),
                "EXPRESSION_HELP missing keyword {keyword:?}"
            );
        }
    }
}
