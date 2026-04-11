//! Private canonicalization helpers.
//!
//! These helpers take already-scanned items and emit their canonical serialized
//! form under formatter policy.

use crate::scanner::{self, KEYCAP_CAP, ScanItem, ScanKind, ZWJ, ZwjComponent, ZwjSequence};
use crate::slot::{self, PolicyView};

/// Canonicalize a single scanned item according to the formatter policy.
#[must_use]
pub(crate) fn canonicalize_item(item: &ScanItem<'_>, policy: &PolicyView<'_>) -> String {
    let mut output = String::with_capacity(item.raw.len());

    match &item.kind {
        ScanKind::Passthrough => {
            output.push_str(item.raw);
        }
        ScanKind::StandaloneSelectors(_) => {}
        ScanKind::Singleton { base, selectors } => {
            format_singleton(&mut output, *base, selectors, policy);
        }
        ScanKind::Keycap { base, .. } => {
            output.push(*base);
            output.push('\u{FE0F}');
            output.push(KEYCAP_CAP);
        }
        ScanKind::Zwj(sequence) => {
            format_zwj(&mut output, sequence);
        }
    }

    output
}

fn format_singleton(output: &mut String, base: char, selectors: &[char], policy: &PolicyView<'_>) {
    let state =
        slot::resolve_singleton_with_view(base, scanner::effective_selector(selectors), policy);

    output.push(base);
    if let Some(selector) = state.as_selector() {
        output.push(selector);
    }
}

fn format_zwj(output: &mut String, sequence: &ZwjSequence) {
    match sequence {
        ZwjSequence::Terminal(component) => {
            format_zwj_component(output, component);
        }
        ZwjSequence::Joined { head, tail, .. } => {
            format_zwj_component(output, head);
            output.push(ZWJ);
            format_zwj(output, tail);
        }
    }
}

fn format_zwj_component(output: &mut String, component: &ZwjComponent) {
    output.push(component.base);
    // UTS #51 requires an emoji modifier to immediately follow its base.
    // Our canonical ZWJ policy therefore never emits both a modifier and a
    // variation selector on the same component: if `emoji_modifier` is present,
    // `canonical_zwj_component_selector` must return `None`.
    if let Some(emoji_modifier) = component.emoji_modifier {
        output.push(emoji_modifier);
    }
    if let Some(sel) = slot::canonical_zwj_component_selector(component) {
        output.push(sel);
    }
}

#[cfg(test)]
mod tests;
