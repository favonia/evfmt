// build.rs — Build script that reads Unicode data files and generates a Rust
// source file containing lookup tables for emoji-related Unicode properties.
// The generated file gets included into `src/unicode.rs` via `include!`.

// This build script treats malformed Unicode data and output failures as fatal.
// There is no runtime caller to recover here; panicking stops compilation with
// a direct build error, and localizing every generated-line write would obscure
// the table-generation flow.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Build scripts have no public API to document.
#![allow(missing_docs)]

use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let data_dir = manifest_dir.join("data");
    let variation_sequences_path = data_dir.join("emoji-variation-sequences.txt");
    let emoji_data_path = data_dir.join("emoji-data.txt");
    let proplist_path = data_dir.join("PropList.txt");

    println!(
        "cargo:rerun-if-changed={}",
        variation_sequences_path.display()
    );
    println!("cargo:rerun-if-changed={}", emoji_data_path.display());
    println!("cargo:rerun-if-changed={}", proplist_path.display());

    let (has_text_vs, has_emoji_vs) = parse_variation_sequences(&variation_sequences_path);
    let emoji_presentation = parse_ucd_property(&emoji_data_path, "Emoji_Presentation");
    let emoji_modifiers = parse_ucd_property(&emoji_data_path, "Emoji_Modifier");
    let emoji_chars = parse_ucd_property(&emoji_data_path, "Emoji");
    let ri_chars = parse_ucd_property(&proplist_path, "Regional_Indicator");

    // Every character in emoji-variation-sequences.txt has both a text (FE0E)
    // and an emoji (FE0F) variation sequence. Assert this so downstream code
    // can treat table membership as "both selectors are sanctioned."
    assert_eq!(
        has_text_vs, has_emoji_vs,
        "expected every variation-sequence character to have both text and emoji entries"
    );

    // AUDIT NOTE: BTreeSet guarantees sorted order in the generated output,
    // which is required for binary search at runtime.
    let variation_chars: BTreeSet<u32> = has_text_vs;

    // --- Generate the Rust source file ---

    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("unicode_data.rs");
    let mut f = fs::File::create(&dest_path).unwrap();

    write_header(&mut f);
    write_char_table(
        &mut f,
        "VARIATION_ENTRIES",
        "variation sequence",
        &variation_chars,
    );
    write_char_table(
        &mut f,
        "EMOJI_MODIFIERS",
        "Emoji_Modifier",
        &emoji_modifiers,
    );
    write_range_table(
        &mut f,
        "EMOJI_PRESENTATION_RANGES",
        "Emoji_Presentation",
        &compress_to_ranges(&emoji_presentation),
    );
    write_range_table(
        &mut f,
        "EMOJI_RANGES",
        "Emoji",
        &compress_to_ranges(&emoji_chars),
    );
    write_range_table(
        &mut f,
        "RI_RANGES",
        "Regional_Indicator",
        &compress_to_ranges(&ri_chars),
    );
}

fn parse_variation_sequences(path: &Path) -> (BTreeSet<u32>, BTreeSet<u32>) {
    let data = fs::read_to_string(path).expect("failed to read emoji-variation-sequences.txt");
    let mut has_text_vs = BTreeSet::new();
    let mut has_emoji_vs = BTreeSet::new();

    for line in data.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let Some((before_semi, after_semi)) = line.split_once(';') else {
            continue;
        };

        let mut parts = before_semi.split_whitespace();
        let Some(cp_hex) = parts.next() else { continue };
        let Some(selector) = parts.next() else {
            continue;
        };
        if parts.next().is_some() {
            continue;
        }

        let cp = u32::from_str_radix(cp_hex, 16).expect("invalid code point");
        let description = after_semi.trim();
        match selector {
            "FE0E" => {
                has_text_vs.insert(cp);
                assert!(
                    description.starts_with("text style"),
                    "FE0E line should be text style: {line}"
                );
            }
            "FE0F" => {
                has_emoji_vs.insert(cp);
                assert!(
                    description.starts_with("emoji style"),
                    "FE0F line should be emoji style: {line}"
                );
            }
            _ => {}
        }
    }

    (has_text_vs, has_emoji_vs)
}

/// Parse a UCD-format property file (`codepoint(s) ; property # comment`)
/// and return all code points matching `wanted_property`.
fn parse_ucd_property(path: &Path, wanted_property: &str) -> BTreeSet<u32> {
    let data = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
    let mut matching = BTreeSet::new();

    for line in data.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let Some((before_semi, after_semi)) = line.split_once(';') else {
            continue;
        };

        let property = after_semi.split('#').next().unwrap_or("").trim();
        if property != wanted_property {
            continue;
        }

        if let Some((start, end)) = before_semi.split_once("..") {
            let start = u32::from_str_radix(start.trim(), 16).expect("invalid range start");
            let end = u32::from_str_radix(end.trim(), 16).expect("invalid range end");
            for cp in start..=end {
                matching.insert(cp);
            }
        } else {
            let cp = u32::from_str_radix(before_semi.trim(), 16).expect("invalid code point");
            matching.insert(cp);
        }
    }

    matching
}

/// Compress a sorted set of code points into non-overlapping `(start, end)` ranges.
fn compress_to_ranges(set: &BTreeSet<u32>) -> Vec<(u32, u32)> {
    let mut ranges = Vec::new();
    let mut iter = set.iter().copied();
    let Some(first) = iter.next() else {
        return ranges;
    };
    let mut start = first;
    let mut end = first;
    for cp in iter {
        if cp == end + 1 {
            end = cp;
        } else {
            ranges.push((start, end));
            start = cp;
            end = cp;
        }
    }
    ranges.push((start, end));
    ranges
}

// --- Code generation helpers ---

fn write_header(f: &mut fs::File) {
    writeln!(f, "// Auto-generated by build.rs from Unicode 17.0 data").unwrap();
    writeln!(f, "// Do not edit manually.").unwrap();
    writeln!(f).unwrap();
    writeln!(f, "/// The Unicode version this data was generated from.").unwrap();
    writeln!(f, "#[allow(dead_code)]").unwrap();
    writeln!(f, "pub(crate) const UNICODE_VERSION: &str = \"17.0\";").unwrap();
}

fn write_char_table(f: &mut fs::File, name: &str, property: &str, chars: &BTreeSet<u32>) {
    writeln!(f).unwrap();
    writeln!(
        f,
        "/// Sorted table of code points with the `{property}` property."
    )
    .unwrap();
    writeln!(f, "const {name}: [char; {}] = [", chars.len()).unwrap();
    for &cp in chars {
        let ch = char::from_u32(cp).expect("invalid code point");
        writeln!(f, "    '\\u{{{:04X}}}',", ch as u32).unwrap();
    }
    writeln!(f, "];").unwrap();
}

fn write_range_table(f: &mut fs::File, name: &str, property: &str, ranges: &[(u32, u32)]) {
    writeln!(f).unwrap();
    writeln!(
        f,
        "/// Sorted, non-overlapping range table for the `{property}` property."
    )
    .unwrap();
    writeln!(f, "const {name}: [(char, char); {}] = [", ranges.len()).unwrap();
    for &(start, end) in ranges {
        let start_ch = char::from_u32(start).expect("invalid range start");
        let end_ch = char::from_u32(end).expect("invalid range end");
        writeln!(
            f,
            "    ('\\u{{{:04X}}}', '\\u{{{:04X}}}'),",
            start_ch as u32, end_ch as u32
        )
        .unwrap();
    }
    writeln!(f, "];").unwrap();
}
