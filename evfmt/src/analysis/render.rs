use crate::presentation::Presentation;
use crate::scanner::{EmojiModification, EmojiTagRun};
use crate::unicode;

/// Render scanner structures after analysis has decided which selectors survive.
///
/// This module does not choose presentation, count non-canonicality, or build
/// replacement fragments. It only renders the already-analyzed emoji-like
/// replacement text.
pub(super) fn render_singleton(
    base: char,
    presentation: Option<Presentation>,
    modifications: &[EmojiModification],
) -> String {
    let mut out = String::new();
    out.push(base);
    if let Some(presentation) = presentation {
        out.push(presentation.as_selector());
    }
    render_modifications(&mut out, modifications);
    out
}

pub(super) fn render_flag(
    first_ri: char,
    second_ri: char,
    modifications: &[EmojiModification],
) -> String {
    let mut out = String::new();
    out.push(first_ri);
    out.push(second_ri);
    render_modifications(&mut out, modifications);
    out
}

fn render_modifications(out: &mut String, modifications: &[EmojiModification]) {
    for modification in modifications {
        render_modification(out, modification);
    }
}

fn render_modification(out: &mut String, modification: &EmojiModification) {
    match modification {
        EmojiModification::EmojiModifier { modifier, .. } => out.push(*modifier),
        EmojiModification::EnclosingKeycap { .. } => {
            out.push(unicode::COMBINING_ENCLOSING_KEYCAP);
        }
        EmojiModification::TagModifier(runs) => render_tag_runs(out, runs),
    }
}

fn render_tag_runs(out: &mut String, runs: &[EmojiTagRun]) {
    for run in runs {
        for ch in &run.tag {
            out.push(*ch);
        }
    }
}
