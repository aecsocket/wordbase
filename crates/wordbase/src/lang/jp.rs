//! Japanese-specific items.

use std::iter;

use serde::{Deserialize, Serialize};

/// Single pitch reading for a [term].
///
/// Japanese [dictionaries] may collect information on how a specific [term] is
/// [pronounced orally][jpa]. This information is represented in this type.
///
/// A single [term] may have multiple ways of being pronounced, which maps to
/// multiple [`Pitch`] values.
///
/// [term]: crate::Term
/// [dictionaries]: crate::Dictionary
/// [jpa]: https://en.wikipedia.org/wiki/Japanese_pitch_accent
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pitch {
    pub position: u64,
    pub nasal: Vec<u64>,
    pub devoice: Vec<u64>,
}

/// Checks if the given character is one of the small kana characters, either
/// hiragana or katakana.
/// 
/// This returns `false` for `っ` (see [`mora`]).
#[must_use]
#[rustfmt::skip]
pub const fn is_small_kana(c: char) -> bool {
    matches!(
        c,
        'ゃ' | 'ゅ' | 'ょ' | 'ぁ' | 'ぃ' | 'ぅ' | 'ぇ' | 'ぉ' |
        'ャ' | 'ュ' | 'ョ' | 'ァ' | 'ィ' | 'ゥ' | 'ェ' | 'ォ'
    )
}

/// Splits a reading (either hiragana or katakana) into its constituent [morae].
///
/// Rules:
/// - kana followed by a small kana is treated as a single mora, for example
///   `ひょ`, `じゅ`.
/// - `っ` is treated as its own mora.
/// - otherwise, each character is its own mora.
///
/// [morae]: https://en.wikipedia.org/wiki/Mora_(linguistics)
pub fn morae(reading: &str) -> impl Iterator<Item = &str> {
    let mut chars = reading.char_indices().peekable();
    iter::from_fn(move || {
        let (byte_index, char) = chars.next()?;
        if let Some((next_byte_index, next_char)) = chars.peek().copied() {
            if is_small_kana(next_char) {
                _ = chars.next();
                let end = next_byte_index + next_char.len_utf8();
                return Some(&reading[byte_index..end]);
            }
        }

        let end = byte_index + char.len_utf8();
        Some(&reading[byte_index..end])
    })
}

/// For a [pitch position][jpa] where the downstep is on the [mora] at index
/// `downstep`, is the [mora] at index `position` high or low?
///
/// Rules:
/// - if the downstep is at 0 (*heiban*, where there is no downstep):
///   - the first position is low
///   - all later positions are high
/// - if the downstep is at 1 (*atamadaka*):
///   - the first position is high
///   - all later positions are low
/// - if the downstep is after 1 (*nakadaka* or *odaka*);
///   - the first position is low
///   - all positions until `position` are high
///   - `position` and onwards are low
///
/// [jpa]: https://en.wikipedia.org/wiki/Japanese_pitch_accent
/// [mora]: https://en.wikipedia.org/wiki/Mora_(linguistics)
#[must_use]
pub const fn is_high(downstep: usize, position: usize) -> bool {
    match downstep {
        0 => position > 0,
        1 => position == 0,
        _ => position > 0 && position < downstep,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn morae() {
        fn splits_into<'a>(reading: &str, target: impl AsRef<[&'a str]>) {
            assert_eq!(&super::morae(reading).collect::<Vec<_>>(), target.as_ref());
        }

        splits_into("hello", ["h", "e", "l", "l", "o"]);
        splits_into("あいうえお", ["あ", "い", "う", "え", "お"]);
        splits_into("日本", ["日", "本"]);
        splits_into("ぎじゅつ", ["ぎ", "じゅ", "つ"]);
        splits_into("さぎょう", ["さ", "ぎょ", "う"]);
        splits_into("さっそく", ["さ", "っ", "そ", "く"]);
    }

    #[test]
    fn is_high() {
        fn follows_pattern(downstep: usize, target_pitches: [bool; 4]) {
            let actual_pitches = [
                super::is_high(downstep, 0),
                super::is_high(downstep, 1),
                super::is_high(downstep, 2),
                super::is_high(downstep, 3),
            ];
            assert_eq!(actual_pitches, target_pitches);
        }

        // heiban
        follows_pattern(0, [false, true, true, true]);

        // atamadaka
        follows_pattern(1, [true, false, false, false]);

        // nakadaka
        follows_pattern(2, [false, true, false, false]);
        follows_pattern(3, [false, true, true, false]);

        // odaka
        follows_pattern(4, [false, true, true, true]);
    }
}
