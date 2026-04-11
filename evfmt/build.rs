// build.rs — Build script that reads Unicode data files and generates a Rust
// source file containing a lookup table of emoji variation sequence information.
// The generated file gets included into `src/variation.rs` via `include!`.

// This build script treats malformed Unicode data and output failures as fatal.
// There is no runtime caller to recover here; panicking stops compilation with
// a direct build error, and localizing every generated-line write would obscure
// the table-generation flow.
#![allow(clippy::unwrap_used, clippy::expect_used)]
// Build scripts have no public API to document.
#![allow(missing_docs)]

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

struct Entry {
    cp: u32,
    has_text_vs: bool,
    has_emoji_vs: bool,
    default_emoji: bool,
}

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let data_dir = manifest_dir.join("data");
    let variation_sequences_path = data_dir.join("emoji-variation-sequences.txt");
    let emoji_data_path = data_dir.join("emoji-data.txt");

    println!(
        "cargo:rerun-if-changed={}",
        variation_sequences_path.display()
    );
    println!("cargo:rerun-if-changed={}", emoji_data_path.display());

    let (has_text_vs, has_emoji_vs) = parse_variation_sequences(&variation_sequences_path);
    let emoji_presentation = parse_emoji_property(&emoji_data_path, "Emoji_Presentation");
    let emoji_modifiers = parse_emoji_property(&emoji_data_path, "Emoji_Modifier");

    // --- Build the variation entry table ---
    // AUDIT NOTE: BTreeMap ensures entries are sorted by code_point in the
    // generated output, which is required for binary_search_by_key in lookup().

    let all_vs_cps: BTreeSet<u32> = has_text_vs.union(&has_emoji_vs).copied().collect();

    let mut entries: BTreeMap<u32, Entry> = BTreeMap::new();
    for &cp in &all_vs_cps {
        entries.insert(
            cp,
            Entry {
                cp,
                has_text_vs: has_text_vs.contains(&cp),
                has_emoji_vs: has_emoji_vs.contains(&cp),
                default_emoji: emoji_presentation.contains(&cp),
            },
        );
    }

    // --- Generate the Rust source file ---

    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("unicode_data.rs");
    let mut f = fs::File::create(&dest_path).unwrap();
    write_generated_table(&mut f, &entries, &emoji_modifiers);
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

fn parse_emoji_property(path: &Path, wanted_property: &str) -> BTreeSet<u32> {
    let data = fs::read_to_string(path).expect("failed to read emoji-data.txt");
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
            let start = u32::from_str_radix(start.trim(), 16)
                .expect("invalid range start in emoji-data.txt");
            let end =
                u32::from_str_radix(end.trim(), 16).expect("invalid range end in emoji-data.txt");
            for cp in start..=end {
                matching.insert(cp);
            }
        } else {
            let cp = u32::from_str_radix(before_semi.trim(), 16)
                .expect("invalid code point in emoji-data.txt");
            matching.insert(cp);
        }
    }

    matching
}

fn write_generated_table(
    f: &mut fs::File,
    entries: &BTreeMap<u32, Entry>,
    emoji_modifiers: &BTreeSet<u32>,
) {
    writeln!(f, "// Auto-generated by build.rs from Unicode 16.0 data").unwrap();
    writeln!(f, "// Do not edit manually.").unwrap();
    writeln!(f).unwrap();
    writeln!(f, "/// The Unicode version this data was generated from.").unwrap();
    writeln!(f, "#[allow(dead_code)]").unwrap();
    writeln!(f, "pub const UNICODE_VERSION: &str = \"16.0\";").unwrap();
    writeln!(f).unwrap();
    writeln!(f, "/// A single entry in the variation sequence table.").unwrap();
    writeln!(
        f,
        "// Generated struct name intentionally mirrors the module name for clarity."
    )
    .unwrap();
    writeln!(f, "#[allow(clippy::module_name_repetitions)]").unwrap();
    writeln!(f, "#[derive(Debug, Clone, Copy)]").unwrap();
    writeln!(f, "pub struct VariationEntry {{").unwrap();
    writeln!(f, "    /// The Unicode code point.").unwrap();
    writeln!(f, "    pub code_point: char,").unwrap();
    writeln!(
        f,
        "    /// Whether this code point has a text variation sequence (+ FE0E)."
    )
    .unwrap();
    writeln!(f, "    pub has_text_vs: bool,").unwrap();
    writeln!(
        f,
        "    /// Whether this code point has an emoji variation sequence (+ FE0F)."
    )
    .unwrap();
    writeln!(f, "    pub has_emoji_vs: bool,").unwrap();
    writeln!(
        f,
        "    /// Whether the Unicode default presentation is emoji."
    )
    .unwrap();
    writeln!(f, "    pub default_emoji: bool,").unwrap();
    writeln!(f, "}}").unwrap();
    writeln!(f).unwrap();
    writeln!(
        f,
        "/// Sorted table of all eligible variation sequence entries."
    )
    .unwrap();
    writeln!(f, "pub static VARIATION_ENTRIES: &[VariationEntry] = &[").unwrap();

    for entry in entries.values() {
        let ch = char::from_u32(entry.cp).expect("invalid code point");
        writeln!(
            f,
            "    VariationEntry {{ code_point: '\\u{{{:04X}}}', has_text_vs: {}, has_emoji_vs: {}, default_emoji: {} }},",
            ch as u32, entry.has_text_vs, entry.has_emoji_vs, entry.default_emoji
        ).unwrap();
    }

    writeln!(f, "];").unwrap();
    writeln!(f).unwrap();
    writeln!(
        f,
        "/// Sorted table of code points with the `Emoji_Modifier` property."
    )
    .unwrap();
    writeln!(f, "pub static EMOJI_MODIFIERS: &[char] = &[").unwrap();
    for &cp in emoji_modifiers {
        let ch = char::from_u32(cp).expect("invalid code point");
        writeln!(f, "    '\\u{{{:04X}}}',", ch as u32).unwrap();
    }
    writeln!(f, "];").unwrap();
}
