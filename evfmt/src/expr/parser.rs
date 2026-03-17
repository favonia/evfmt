use std::fmt;

use crate::expr::{Expr, NamedSetId};
use crate::scanner::{VS_EMOJI, VS_TEXT};

/// A non-fatal issue found during parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseWarning {
    /// Human-readable description of the issue.
    pub message: String,
}

impl fmt::Display for ParseWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "expression warning: {}", self.message)
    }
}

/// A successfully parsed expression, possibly with warnings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseResult {
    /// The parsed expression.
    pub expr: Expr,
    /// Non-fatal warnings encountered during parsing.
    pub warnings: Vec<ParseWarning>,
}

/// An error encountered while parsing an expression string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    /// Human-readable description of what went wrong.
    pub message: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "expression parse error: {}", self.message)
    }
}

impl std::error::Error for ParseError {}

/// Parse a complete expression string.
///
/// Returns `ParseResult` containing the parsed expression and any warnings.
///
/// # Errors
///
/// Returns `ParseError` if the input is empty, contains invalid syntax,
/// or has unexpected trailing text after a valid expression.
pub fn parse(input: &str) -> Result<ParseResult, ParseError> {
    let input = input.trim();
    if input.is_empty() {
        return Err(ParseError {
            message: "empty expression".to_owned(),
        });
    }

    let mut warnings = Vec::new();

    if let Some(rest) = strip_function_call(input, "except") {
        let expr = parse_except(rest, &mut warnings)?;
        return Ok(ParseResult { expr, warnings });
    }

    let (expr, rest) = parse_expr(input, &mut warnings)?;
    let rest = rest.trim();

    if !rest.is_empty() {
        return Err(ParseError {
            message: format!("unexpected trailing text: {rest:?}"),
        });
    }
    Ok(ParseResult { expr, warnings })
}

/// Parse a complete expression string, discarding any warnings.
///
/// # Errors
///
/// Returns `ParseError` if the input is invalid.
pub fn parse_expr_only(input: &str) -> Result<Expr, ParseError> {
    parse(input).map(|r| r.expr)
}

/// Parse one expression from the start of `input`.
///
/// Returns the parsed expression together with the unconsumed suffix so callers
/// can continue parsing enclosing function arguments.
fn parse_expr<'a>(
    input: &'a str,
    warnings: &mut Vec<ParseWarning>,
) -> Result<(Expr, &'a str), ParseError> {
    let input = input.trim_start();

    if let Some(rest) = input.strip_prefix("all")
        && is_word_boundary(rest)
    {
        return Ok((Expr::All, rest));
    }
    if let Some(rest) = input.strip_prefix("none")
        && is_word_boundary(rest)
    {
        return Ok((Expr::None, rest));
    }

    if let Some(rest) = input.strip_prefix("emoji-defaults")
        && is_word_boundary(rest)
    {
        return Ok((Expr::NamedSet(NamedSetId::EmojiDefaults), rest));
    }
    if let Some(rest) = input.strip_prefix("ascii")
        && is_word_boundary(rest)
    {
        return Ok((Expr::NamedSet(NamedSetId::Ascii), rest));
    }
    if let Some(rest) = input.strip_prefix("rights-marks")
        && is_word_boundary(rest)
    {
        return Ok((Expr::NamedSet(NamedSetId::RightsMarks), rest));
    }
    if let Some(rest) = input.strip_prefix("arrows")
        && is_word_boundary(rest)
    {
        return Ok((Expr::NamedSet(NamedSetId::Arrows), rest));
    }
    if let Some(rest) = input.strip_prefix("card-suits")
        && is_word_boundary(rest)
    {
        return Ok((Expr::NamedSet(NamedSetId::CardSuits), rest));
    }

    // `except(...)` is handled specially by `parse` as a top-level shorthand for
    // `subtract(all, ...)`. Reaching it here means the caller used it in a nested
    // position such as `union(except(ascii), ...)`, which is intentionally rejected.
    if strip_function_call(input, "except").is_some() {
        return Err(ParseError {
            message: "except(...) is only allowed at the top level; use subtract(all, ...) instead"
                .to_owned(),
        });
    }

    if let Some(rest) = strip_function_call(input, "subtract") {
        return parse_subtract(rest, warnings);
    }
    if let Some(rest) = strip_function_call(input, "union") {
        return parse_union(rest, warnings);
    }
    if let Some(rest) = strip_function_call(input, "u") {
        return parse_u(rest, warnings);
    }

    if input.starts_with('\'') {
        return parse_single_quoted(input);
    }
    if input.starts_with('"') {
        return parse_double_quoted(input);
    }

    Err(ParseError {
        message: format!("unexpected input: {input:?}"),
    })
}

fn is_word_boundary(s: &str) -> bool {
    s.is_empty() || !s.starts_with(|c: char| c.is_alphanumeric() || c == '_')
}

fn strip_function_call<'a>(input: &'a str, name: &str) -> Option<&'a str> {
    let rest = input.strip_prefix(name)?;
    let rest = rest.trim_start();
    rest.strip_prefix('(')
}

fn parse_u<'a>(
    input: &'a str,
    warnings: &mut Vec<ParseWarning>,
) -> Result<(Expr, &'a str), ParseError> {
    let (cp, rest) = parse_hex_codepoint(input, warnings)?;
    let rest = rest.trim_start();
    let rest = rest.strip_prefix(')').ok_or_else(|| ParseError {
        message: "expected ')' in u(...)".to_owned(),
    })?;
    Ok((Expr::CodePoint(cp), rest))
}

fn parse_union<'a>(
    input: &'a str,
    warnings: &mut Vec<ParseWarning>,
) -> Result<(Expr, &'a str), ParseError> {
    let mut args = Vec::new();
    let mut rest = input;

    let (first, r) = parse_expr(rest, warnings)?;
    args.push(first);
    rest = r.trim_start();

    while let Some(r) = rest.strip_prefix(',') {
        let (arg, r) = parse_expr(r, warnings)?;
        args.push(arg);
        rest = r.trim_start();
    }

    let rest = rest.strip_prefix(')').ok_or_else(|| ParseError {
        message: "expected ')' or ',' in union(...)".to_owned(),
    })?;

    Ok((Expr::Union(args), rest))
}

fn parse_subtract<'a>(
    input: &'a str,
    warnings: &mut Vec<ParseWarning>,
) -> Result<(Expr, &'a str), ParseError> {
    let (base, rest) = parse_expr(input, warnings)?;
    let mut rest = rest.trim_start();

    let mut excluded = Vec::new();
    while let Some(r) = rest.strip_prefix(',') {
        let (arg, r) = parse_expr(r, warnings)?;
        excluded.push(arg);
        rest = r.trim_start();
    }

    if excluded.is_empty() {
        return Err(ParseError {
            message: "subtract(...) requires at least two arguments".to_owned(),
        });
    }

    let rest = rest.strip_prefix(')').ok_or_else(|| ParseError {
        message: "expected ')' or ',' in subtract(...)".to_owned(),
    })?;

    Ok((Expr::Subtract(Box::new(base), excluded), rest))
}

fn parse_except(input: &str, warnings: &mut Vec<ParseWarning>) -> Result<Expr, ParseError> {
    let (inner, rest) = parse_expr(input, warnings)?;
    let rest = rest.trim_start();
    let rest = rest.strip_prefix(')').ok_or_else(|| ParseError {
        message: "expected ')' in except(...)".to_owned(),
    })?;
    let rest = rest.trim();
    if !rest.is_empty() {
        return Err(ParseError {
            message: format!("unexpected trailing text: {rest:?}"),
        });
    }
    Ok(Expr::Subtract(Box::new(Expr::All), vec![inner]))
}

fn strip_literal_variant_selectors(input: &str) -> String {
    input
        .chars()
        .filter(|&ch| ch != VS_TEXT && ch != VS_EMOJI)
        .collect()
}

fn parse_single_quoted(input: &str) -> Result<(Expr, &str), ParseError> {
    #[allow(clippy::string_slice)]
    let after_open = &input[1..];

    let close_pos = after_open.find('\'').ok_or_else(|| ParseError {
        message: "unterminated single-quoted literal".to_owned(),
    })?;

    if close_pos == 0 {
        return Err(ParseError {
            message: "empty single-quoted literal".to_owned(),
        });
    }

    #[allow(clippy::string_slice)]
    let content = &after_open[..close_pos];

    let filtered = strip_literal_variant_selectors(content);
    let mut chars = filtered.chars();
    let ch = chars.next().ok_or_else(|| ParseError {
        message: "single-quoted literal must contain exactly one non-selector character".to_owned(),
    })?;
    if chars.next().is_some() {
        return Err(ParseError {
            message: "single-quoted literal must contain exactly one non-selector character"
                .to_owned(),
        });
    }

    let consumed = 1 + close_pos + 1;
    #[allow(clippy::string_slice)]
    let rest = &input[consumed..];
    Ok((Expr::CodePoint(ch), rest))
}

fn parse_double_quoted(input: &str) -> Result<(Expr, &str), ParseError> {
    #[allow(clippy::string_slice)]
    let after_open = &input[1..];

    let close_pos = after_open.find('"').ok_or_else(|| ParseError {
        message: "unterminated double-quoted literal".to_owned(),
    })?;

    if close_pos == 0 {
        return Err(ParseError {
            message: "empty double-quoted literal".to_owned(),
        });
    }

    #[allow(clippy::string_slice)]
    let content = &after_open[..close_pos];

    let filtered = strip_literal_variant_selectors(content);
    let chars: Vec<Expr> = filtered.chars().map(Expr::CodePoint).collect();
    if chars.is_empty() {
        return Err(ParseError {
            message: "double-quoted literal must contain at least one non-selector character"
                .to_owned(),
        });
    }
    if chars.len() == 1 {
        let consumed = 1 + close_pos + 1;
        #[allow(clippy::string_slice)]
        let rest = &input[consumed..];
        return Ok((chars.into_iter().next().unwrap_or(Expr::None), rest));
    }

    let consumed = 1 + close_pos + 1;
    #[allow(clippy::string_slice)]
    let rest = &input[consumed..];
    Ok((Expr::Union(chars), rest))
}

fn parse_hex_codepoint<'a>(
    input: &'a str,
    warnings: &mut Vec<ParseWarning>,
) -> Result<(char, &'a str), ParseError> {
    let input = input.trim_start();
    let hex_end = input
        .find(|c: char| !c.is_ascii_hexdigit())
        .unwrap_or(input.len());
    #[allow(clippy::string_slice)]
    let hex_str = &input[..hex_end];

    let len = hex_str.len();
    if !(4..=6).contains(&len) {
        return Err(ParseError {
            message: format!(
                "u(...) requires exactly 4, 5, or 6 hex digits, got {len}: {hex_str:?}"
            ),
        });
    }

    let has_lower = hex_str.chars().any(|c| c.is_ascii_lowercase());
    let has_upper = hex_str.chars().any(|c| c.is_ascii_uppercase());
    if has_lower && has_upper {
        let canonical = hex_str.to_ascii_uppercase();
        warnings.push(ParseWarning {
            message: format!("u({hex_str}) uses mixed-case hex; canonical form is u({canonical})"),
        });
    }

    let cp = u32::from_str_radix(hex_str, 16).map_err(|_| ParseError {
        message: format!("invalid hex: {hex_str:?}"),
    })?;

    let ch = char::from_u32(cp).ok_or_else(|| ParseError {
        message: format!("U+{cp:04X} is not a valid Unicode scalar value"),
    })?;

    #[allow(clippy::string_slice)]
    Ok((ch, &input[hex_end..]))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    fn parse_ok(input: &str) -> Expr {
        parse(input).unwrap().expr
    }

    #[test]
    fn test_parse_all() {
        assert_eq!(parse_ok("all"), Expr::All);
    }

    #[test]
    fn test_parse_none() {
        assert_eq!(parse_ok("none"), Expr::None);
    }

    #[test]
    fn test_parse_ascii() {
        assert_eq!(parse_ok("ascii"), Expr::NamedSet(NamedSetId::Ascii));
    }

    #[test]
    fn test_parse_emoji_defaults() {
        assert_eq!(
            parse_ok("emoji-defaults"),
            Expr::NamedSet(NamedSetId::EmojiDefaults)
        );
    }

    #[test]
    fn test_parse_rights_marks() {
        assert_eq!(
            parse_ok("rights-marks"),
            Expr::NamedSet(NamedSetId::RightsMarks)
        );
    }

    #[test]
    fn test_parse_arrows() {
        assert_eq!(parse_ok("arrows"), Expr::NamedSet(NamedSetId::Arrows));
    }

    #[test]
    fn test_parse_card_suits() {
        assert_eq!(
            parse_ok("card-suits"),
            Expr::NamedSet(NamedSetId::CardSuits)
        );
    }

    #[test]
    fn test_parse_u_single() {
        assert_eq!(parse_ok("u(0023)"), Expr::CodePoint('\u{0023}'));
        assert_eq!(parse_ok("u(1F600)"), Expr::CodePoint('\u{1F600}'));
    }

    #[test]
    fn test_parse_u_uppercase_no_warning() {
        let result = parse("u(00FF)").unwrap();
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_parse_u_lowercase_no_warning() {
        let result = parse("u(00ff)").unwrap();
        assert_eq!(result.expr, Expr::CodePoint('\u{00FF}'));
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_parse_u_mixed_case_warns() {
        let result = parse("u(00aD)").unwrap();
        assert_eq!(result.expr, Expr::CodePoint('\u{00AD}'));
        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].message.contains("mixed-case"));
    }

    #[test]
    fn test_parse_u_rejects_short() {
        assert!(parse("u(23)").is_err());
        assert!(parse("u(F)").is_err());
    }

    #[test]
    fn test_parse_u_rejects_multiple() {
        assert!(parse("u(0023,002A)").is_err());
    }

    #[test]
    fn test_parse_single_quoted() {
        assert_eq!(parse_ok("'#'"), Expr::CodePoint('#'));
        assert_eq!(parse_ok("'\u{00A9}'"), Expr::CodePoint('\u{00A9}'));
        assert_eq!(parse_ok("'😀'"), Expr::CodePoint('😀'));
    }

    #[test]
    fn test_parse_single_quoted_ignores_variant_selectors() {
        assert_eq!(parse_ok("'\u{00A9}\u{FE0F}'"), Expr::CodePoint('\u{00A9}'));
        assert_eq!(parse_ok("'\u{00A9}\u{FE0E}'"), Expr::CodePoint('\u{00A9}'));
        assert_eq!(
            parse_ok("'\u{FE0F}\u{00A9}\u{FE0E}\u{FE0F}'"),
            Expr::CodePoint('\u{00A9}')
        );
    }

    #[test]
    fn test_parse_single_quoted_rejects_empty() {
        assert!(parse("''").is_err());
    }

    #[test]
    fn test_parse_single_quoted_rejects_multi() {
        assert!(parse("'ab'").is_err());
    }

    #[test]
    fn test_parse_single_quoted_rejects_selector_only() {
        assert!(parse("'\u{FE0F}'").is_err());
        assert!(parse("'\u{FE0F}\u{FE0E}'").is_err());
    }

    #[test]
    fn test_parse_single_quoted_counts_non_selectors_after_filtering() {
        assert!(parse("'a\u{FE0F}b'").is_err());
        assert!(parse("'\u{FE0E}a\u{FE0F}b\u{FE0E}'").is_err());
    }

    #[test]
    fn test_parse_double_quoted() {
        assert_eq!(
            parse_ok("\"abc\""),
            Expr::Union(vec![
                Expr::CodePoint('a'),
                Expr::CodePoint('b'),
                Expr::CodePoint('c'),
            ])
        );
    }

    #[test]
    fn test_parse_double_quoted_ignores_variant_selectors() {
        assert_eq!(
            parse_ok("\"\u{00A9}\u{FE0F}#\u{FE0E}\""),
            Expr::Union(vec![Expr::CodePoint('\u{00A9}'), Expr::CodePoint('#')])
        );
        assert_eq!(
            parse_ok("\"\u{FE0F}\u{00A9}\u{FE0E}\u{FE0F}#\u{FE0E}\""),
            Expr::Union(vec![Expr::CodePoint('\u{00A9}'), Expr::CodePoint('#')])
        );
    }

    #[test]
    fn test_parse_double_quoted_single_char() {
        assert_eq!(parse_ok("\"a\""), Expr::CodePoint('a'));
    }

    #[test]
    fn test_parse_double_quoted_single_char_after_filtering() {
        assert_eq!(parse_ok("\"a\u{FE0F}\""), Expr::CodePoint('a'));
        assert_eq!(
            parse_ok("\"\u{FE0E}a\u{FE0F}\u{FE0E}\""),
            Expr::CodePoint('a')
        );
    }

    #[test]
    fn test_parse_double_quoted_rejects_empty() {
        assert!(parse("\"\"").is_err());
    }

    #[test]
    fn test_parse_double_quoted_rejects_selector_only() {
        assert!(parse("\"\u{FE0F}\u{FE0E}\"").is_err());
    }

    #[test]
    fn test_parse_quoted_in_subtract() {
        assert_eq!(
            parse_ok("subtract(ascii, \"#*\")"),
            Expr::Subtract(
                Box::new(Expr::NamedSet(NamedSetId::Ascii)),
                vec![Expr::Union(vec![
                    Expr::CodePoint('#'),
                    Expr::CodePoint('*'),
                ])],
            )
        );
    }

    #[test]
    fn test_parse_quoted_with_selectors_in_subtract() {
        assert_eq!(
            parse_ok("subtract(ascii, \"#\u{FE0F}*\u{FE0E}\")"),
            Expr::Subtract(
                Box::new(Expr::NamedSet(NamedSetId::Ascii)),
                vec![Expr::Union(vec![
                    Expr::CodePoint('#'),
                    Expr::CodePoint('*'),
                ])],
            )
        );
    }

    #[test]
    fn test_parse_union() {
        assert_eq!(
            parse_ok("union(u(0023), u(002A))"),
            Expr::Union(vec![
                Expr::CodePoint('\u{0023}'),
                Expr::CodePoint('\u{002A}'),
            ])
        );
    }

    #[test]
    fn test_parse_union_single() {
        assert_eq!(
            parse_ok("union(ascii)"),
            Expr::Union(vec![Expr::NamedSet(NamedSetId::Ascii)])
        );
    }

    #[test]
    fn test_parse_subtract_binary() {
        assert_eq!(
            parse_ok("subtract(all, ascii)"),
            Expr::Subtract(Box::new(Expr::All), vec![Expr::NamedSet(NamedSetId::Ascii)])
        );
    }

    #[test]
    fn test_parse_subtract_variadic() {
        assert_eq!(
            parse_ok("subtract(all, ascii, emoji-defaults)"),
            Expr::Subtract(
                Box::new(Expr::All),
                vec![
                    Expr::NamedSet(NamedSetId::Ascii),
                    Expr::NamedSet(NamedSetId::EmojiDefaults),
                ],
            )
        );
    }

    #[test]
    fn test_parse_subtract_rejects_unary() {
        assert!(parse("subtract(all)").is_err());
    }

    #[test]
    fn test_parse_except() {
        assert_eq!(
            parse_ok("except(ascii)"),
            Expr::Subtract(Box::new(Expr::All), vec![Expr::NamedSet(NamedSetId::Ascii)])
        );
    }

    #[test]
    fn test_parse_except_nested_rejected() {
        assert!(parse("union(except(ascii))").is_err());
    }

    #[test]
    fn test_parse_nested() {
        assert_eq!(
            parse_ok("subtract(all, union(ascii, emoji-defaults))"),
            Expr::Subtract(
                Box::new(Expr::All),
                vec![Expr::Union(vec![
                    Expr::NamedSet(NamedSetId::Ascii),
                    Expr::NamedSet(NamedSetId::EmojiDefaults),
                ])],
            )
        );
    }

    #[test]
    fn test_warnings_propagate_through_nesting() {
        let result = parse("subtract(all, u(00fF))").unwrap();
        assert_eq!(
            result.expr,
            Expr::Subtract(Box::new(Expr::All), vec![Expr::CodePoint('\u{00FF}')])
        );
        assert_eq!(result.warnings.len(), 1);
    }

    #[test]
    fn test_empty_is_error() {
        assert!(parse("").is_err());
    }

    #[test]
    fn test_parse_warning_display_includes_prefix() {
        let warning = ParseWarning {
            message: "mixed-case hex".to_owned(),
        };

        assert_eq!(warning.to_string(), "expression warning: mixed-case hex");
    }

    #[test]
    fn test_parse_error_display_includes_prefix() {
        let error = ParseError {
            message: "unexpected input".to_owned(),
        };

        assert_eq!(
            error.to_string(),
            "expression parse error: unexpected input"
        );
    }

    #[test]
    fn test_keyword_prefix_without_boundary_reports_unexpected_input() {
        let error = parse("ascii_suffix").unwrap_err();

        assert_eq!(
            error.to_string(),
            "expression parse error: unexpected input: \"ascii_suffix\""
        );
    }

    #[test]
    fn test_trailing_text_is_error() {
        assert!(parse("ascii foo").is_err());
    }

    #[test]
    fn test_old_syntax_rejected() {
        assert!(parse("true").is_err());
        assert!(parse("false").is_err());
        assert!(parse("not(ascii)").is_err());
        assert!(parse("and(ascii, all)").is_err());
        assert!(parse("or(ascii, all)").is_err());
        assert!(parse("urange(0023,0039)").is_err());
    }
}
