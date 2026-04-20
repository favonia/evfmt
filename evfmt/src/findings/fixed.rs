use crate::scanner::{EmojiModification, EmojiTagRun, Presentation};
use crate::unicode;

/// An emoji-like unit after cleanup decisions have been applied.
///
/// `FixedEmojiLike` is the renderable form of one scanned emoji-like unit:
/// the stem presentation has already been selected, and each modification
/// has been reduced to the content that survives selector cleanup.
pub(super) struct FixedEmojiLike<'a> {
    stem: FixedEmojiStem,
    modifications: Vec<FixedModification<'a>>,
}

#[derive(Clone, Copy)]
enum FixedEmojiStem {
    SingletonBase {
        base: char,
        presentation: Option<Presentation>,
    },
    Flag {
        first_ri: char,
        second_ri: char,
    },
}

#[derive(Clone, Copy)]
enum FixedModification<'a> {
    EmojiModifier(char),
    EnclosingKeycap,
    TagModifier(&'a [EmojiTagRun]),
}

impl<'a> FixedEmojiLike<'a> {
    pub(super) fn singleton_base(
        base: char,
        presentation: Option<Presentation>,
        modifications: &'a [EmojiModification],
    ) -> Self {
        Self {
            stem: FixedEmojiStem::SingletonBase { base, presentation },
            modifications: fixed_modifications(modifications),
        }
    }

    pub(super) fn flag(
        first_ri: char,
        second_ri: char,
        modifications: &'a [EmojiModification],
    ) -> Self {
        Self {
            stem: FixedEmojiStem::Flag {
                first_ri,
                second_ri,
            },
            modifications: fixed_modifications(modifications),
        }
    }

    pub(super) fn render(&self, out: &mut String) {
        match self.stem {
            FixedEmojiStem::SingletonBase { base, presentation } => {
                out.push(base);
                if let Some(presentation) = presentation {
                    out.push(presentation.as_selector());
                }
            }
            FixedEmojiStem::Flag {
                first_ri,
                second_ri,
            } => {
                out.push(first_ri);
                out.push(second_ri);
            }
        }

        for modification in &self.modifications {
            modification.render(out);
        }
    }

    pub(super) fn render_to_string(&self) -> String {
        let mut out = String::new();
        self.render(&mut out);
        out
    }
}

impl FixedModification<'_> {
    fn render(self, out: &mut String) {
        match self {
            Self::EmojiModifier(modifier) => out.push(modifier),
            Self::EnclosingKeycap => out.push(unicode::COMBINING_ENCLOSING_KEYCAP),
            Self::TagModifier(runs) => {
                for run in runs {
                    for ch in &run.tag {
                        out.push(*ch);
                    }
                }
            }
        }
    }
}

fn fixed_modifications(modifications: &[EmojiModification]) -> Vec<FixedModification<'_>> {
    modifications
        .iter()
        .map(|modification| match modification {
            EmojiModification::EmojiModifier { modifier, .. } => {
                FixedModification::EmojiModifier(*modifier)
            }
            EmojiModification::EnclosingKeycap { .. } => FixedModification::EnclosingKeycap,
            EmojiModification::TagModifier(runs) => FixedModification::TagModifier(runs),
        })
        .collect()
}
