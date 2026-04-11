use super::super::*;

pub(in crate::scanner) fn scan_legacy(input: &str) -> Vec<ScanItem<'_>> {
    let mut items = Vec::new();
    let mut pos: usize = 0;

    while pos < input.len() {
        // pos < input.len() is checked by the while condition, and pos always
        // advances by ch.len_utf8(), so it stays on a UTF-8 boundary.
        #[allow(clippy::expect_used)]
        let ch = peek(input, pos).expect("pos is within input.len() and on a UTF-8 boundary");
        let ch_len = ch.len_utf8();

        // 1. Standalone variation selector run
        if is_variation_selector(ch) {
            let (end, selectors) = consume_selector_run(input, pos);
            items.push(ScanItem {
                #[allow(clippy::string_slice)]
                raw: &input[pos..end],
                span: pos..end,
                kind: ScanKind::StandaloneSelectors(selectors),
            });
            pos = end;
            continue;
        }

        // 2. Keycap: base [VS] 20E3
        if is_keycap_base(ch)
            && let Some((end, selectors)) = try_keycap(input, pos, ch_len)
        {
            debug_assert!(end > pos, "keycap scan must make forward progress");
            debug_assert!(
                matches!(input.get(pos..end), Some(raw) if raw.ends_with(KEYCAP_CAP)),
                "keycap scan must end with U+20E3"
            );
            items.push(ScanItem {
                #[allow(clippy::string_slice)]
                raw: &input[pos..end],
                span: pos..end,
                kind: ScanKind::Keycap {
                    base: ch,
                    selectors,
                },
            });
            pos = end;
            continue;
        }

        // 3. ZWJ chain: component (200D component)+
        if let Some((end, components)) = try_zwj(input, pos, ch) {
            items.push(ScanItem {
                #[allow(clippy::string_slice)]
                raw: &input[pos..end],
                span: pos..end,
                kind: ScanKind::Zwj(components),
            });
            pos = end;
            continue;
        }

        // 4. Singleton variation-sequence char [VS]
        if unicode::has_variation_sequence(ch) {
            let (end, selectors) = consume_optional_selector_run(input, pos + ch_len);
            debug_assert!(end > pos, "singleton scan must make forward progress");
            items.push(ScanItem {
                #[allow(clippy::string_slice)]
                raw: &input[pos..end],
                span: pos..end,
                kind: ScanKind::Singleton {
                    base: ch,
                    selectors,
                },
            });
            pos = end;
            continue;
        }

        // 5. Passthrough run
        let start = pos;
        pos = consume_passthrough_run(input, pos);
        debug_assert!(pos > start, "passthrough scan must make forward progress");
        items.push(ScanItem {
            #[allow(clippy::string_slice)]
            raw: &input[start..pos],
            span: start..pos,
            kind: ScanKind::Passthrough,
        });
    }

    items
}

/// Try to match keycap: base selector* 20E3. Returns (`end_byte`, selectors).
fn try_keycap(input: &str, pos: usize, base_len: usize) -> Option<(usize, Vec<char>)> {
    let (mut cursor, selectors) = consume_optional_selector_run(input, pos + base_len);

    match peek(input, cursor) {
        Some(KEYCAP_CAP) => {
            cursor += KEYCAP_CAP.len_utf8();
            Some((cursor, selectors))
        }
        _ => None,
    }
}
