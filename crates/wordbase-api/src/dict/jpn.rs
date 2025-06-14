//! Shared types and logic for Japanese dictionaries.

use std::iter;

use itertools::Itertools as _;
use serde::{Deserialize, Serialize};

/// [Mora][mora] position in a word.
///
/// [mora]: https://en.wikipedia.org/wiki/Mora_(linguistics)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PitchPosition(pub u64);

#[cfg(feature = "uniffi")]
uniffi::custom_newtype!(PitchPosition, u64);

/// Category of a word's [pitch][jpa] position.
///
/// [jpa]: https://en.wikipedia.org/wiki/Japanese_pitch_accent
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PitchCategory {
    /// 平板式: first mora is low; all later morae are high.
    ///
    /// ```text
    /// 　　　／￣￣￣　／￣￣￣￣
    /// 平板　へいばん　へいばんが
    /// ```
    Heiban,
    /// 頭高型: first mora is high; all later morae are low.
    ///
    /// ```text
    /// 　　　￣＼　　　　￣＼＿
    /// 　春　はる　　　　はるが
    /// ```
    /// ```text
    /// 　　　￣￣＼＿＿　￣￣＼＿＿＿
    /// 授業　じゅぎょう　じゅぎょうが
    /// ```
    Atamadaka,
    /// 中高型: first mora is low; morae are high until the downstep; downstep is
    /// before the last mora; the particle is low.
    ///
    /// ```text
    /// 　　　　／￣＼　／￣＼＿
    /// お菓子　おかし　おかしが
    /// ```
    Nakadaka,
    /// 大高型: first mora is low; all later morae are high; particle is low.
    ///
    /// ```text
    /// 　　　／￣￣　／￣￣＼
    /// 　男　おとこ　おとこが
    /// ```
    /// ```text
    /// 　　　／￣￣　／￣￣＼
    /// 言葉　ことば　ことばが
    /// ```
    Odaka,
}

/// Checks if the given character is hiragana.
///
/// # Examples
///
/// ```
/// # use wordbase_api::dict::jpn::is_hiragana;
/// assert!(is_hiragana('あ'));
/// assert!(is_hiragana('ん'));
/// assert!(!is_hiragana('ア'));
/// assert!(!is_hiragana('A'));
/// ```
#[must_use]
pub fn is_hiragana(c: char) -> bool {
    ('\u{3040}'..='\u{309F}').contains(&c)
}

/// Checks if the given character is katakana.
///
/// # Examples
///
/// ```
/// # use wordbase_api::dict::jpn::is_katakana;
/// assert!(is_katakana('ア'));
/// assert!(is_katakana('ン'));
/// assert!(!is_katakana('あ'));
/// assert!(!is_katakana('A'));
/// ```
#[must_use]
pub fn is_katakana(c: char) -> bool {
    ('\u{30A0}'..='\u{30FF}').contains(&c)
}

/// Checks if the given character is either hiragana or katakana.
///
/// # Examples
///
/// ```
/// # use wordbase_api::dict::jpn::is_kana;
/// assert!(is_kana('あ'));
/// assert!(is_kana('ア'));
/// assert!(!is_kana('A'));
/// assert!(!is_kana('漢'));
/// ```
#[must_use]
pub fn is_kana(c: char) -> bool {
    is_hiragana(c) || is_katakana(c)
}

/// Checks if the given character is one of the small kana characters, either
/// hiragana or katakana.
///
/// This returns `false` for `っ` (see [`morae`]).
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

/// Computes the [`PitchCategory`] of a pitch accent reading for a word, given
/// the number of morae in the word and where the downstep lands.
#[must_use]
pub const fn pitch_category_of(n_morae: usize, downstep: usize) -> PitchCategory {
    match downstep {
        0 => PitchCategory::Heiban,
        1 => PitchCategory::Atamadaka,
        n if n == n_morae => PitchCategory::Odaka,
        _ => PitchCategory::Nakadaka,
    }
}

/// Attempts to split a headword/reading pair into furigana pairs, using
/// best-effort heuristics.
///
/// Each item in the resulting vector is a pair of:
/// - a part of the headword
/// - the corresponding part of the reading, or `""` if no furigana should be
///   generated for this part
///
/// Note that this function may not generate the most accurate furigana reading
/// possible, which is a fundamental limitation of using heuristics instead of
/// a hardcoded mapping between terms and furigana pairs. It is recommended to
/// use a hardcoded furigana map first from e.g. [`jmdict-furigana`], and use this
/// function as a fallback.
///
/// # Examples
///
/// ```
/// # use wordbase_api::dict::jpn::furigana_parts;
/// assert_eq!(furigana_parts("日本", "にほん"), [("日本", "にほん")]);
/// assert_eq!(
///     furigana_parts("食べる", "たべる"),
///     [("食", "た"), ("べる", "")]
/// );
/// assert_eq!(
///     furigana_parts("取り扱い説明書", "とりあつかいせつめいしょ"),
///     [
///         ("取", "と"),
///         ("り", ""),
///         ("扱", "あつか"),
///         ("い", ""),
///         ("説明書", "せつめいしょ")
///     ]
/// );
/// ```
///
/// [`jmdict-furigana`]: https://docs.rs/jmdict-furigana
#[must_use]
#[expect(clippy::missing_panics_doc, reason = "shouldn't panic")]
pub fn furigana_parts<'a>(headword: &'a str, mut reading: &'a str) -> Vec<(&'a str, &'a str)> {
    #[derive(Debug, Clone, Copy)]
    struct HeadwordPart<'a> {
        text: &'a str,
        rem: &'a str,
        is_kana: bool,
    }

    if headword == reading || reading.is_empty() {
        return vec![(headword, "")];
    }

    // split 取り扱い説明書
    // into [取, り, 扱, い, 説明書]
    let chunks = headword.char_indices().chunk_by(|(_, c)| is_kana(*c));
    let headword_parts = chunks.into_iter().filter_map(|(is_kana, mut chunk)| {
        let first = chunk.next()?;
        let last = chunk.last().unwrap_or(first);

        let first_pos = first.0;
        let last_pos = last.0 + last.1.len_utf8();

        let text = &headword[first_pos..last_pos];
        let rem = &headword[first_pos..];
        Some(HeadwordPart { text, rem, is_kana })
    });

    let mut headword_parts = headword_parts.multipeek();
    let furigana_parts = iter::from_fn(move || {
        let part = headword_parts.next()?;

        if part.is_kana {
            // "り" doesn't need furigana to tell you it's "り"
            // make sure to also advance the reading forward for future iterations
            // e.g. headword = "お茶", reading = "おちゃ"
            //      -> set reading to "ちゃ"
            if let Some(rem) = reading.strip_prefix(part.text) {
                reading = rem;
            }
            return Some((part.text, ""));
        }

        let Some(&peek) = headword_parts.peek() else {
            // let's say we're on "説明書"
            // we have no next part, so everything left in `reading`
            // is a part of the reading of "説明書"
            return Some((part.text, reading));
        };

        // the next part must be a kana...
        debug_assert!(peek.is_kana);

        // [取]り扱い説明書 -> true
        // 取り[扱]い説明書 -> true
        // 取り扱い[説明書] -> false
        //
        // [関係無]い       -> false
        //
        // [黄色]い声       -> true
        // 黄色い[声]       -> false
        let has_more_kanji = headword_parts.peek().is_some();

        // ...so we split our reading in half, at that kana's position
        // let's say we're on "取"
        // we peek the next part "り"
        // and try to find the next occurrence of "り" in `reading`
        // so everything in `reading` up to that "り" is a part of the reading of "取"
        // and "り" and everything after that is the remainder of the reading
        //
        // let's say we're generating for "聞き" (きき) and we're on "聞"
        // if we look for き we're gonna find the FIRST one, which is wrong
        // instead, we want to find the LAST き before a non-き pattern (or the end)
        //
        // in the general case, if we have some text "xxababCDab", we want to:
        // - find where the "ab" pattern starts
        // - keep going through "ab"s until we find a non-"ab" pattern
        // - collect everything before that last "ab" ("xxab") as this part's reading
        // - keep that "ab" and after ("abCDab") for the reading remainder

        let find = if has_more_kanji {
            // we're processing something like:
            // - [取]り扱い説明書
            // - 取り[扱]い説明書
            // so we look for the FIRST `peek.text` to split on:
            // - と / り / あつかいせつめいしょ
            // - とりあつか / い / せつめいしょ
            reading.find(peek.text)
        } else {
            // we're processing something like:
            // - [関係無]い [かんけいない]
            // so we look for the LAST "い" to split on:
            // - かんけいな / い
            reading.rfind(peek.text)
        };
        let Some(first_peek_pos) = find else {
            // this can happen if the reading is a bit non-standard
            // e.g. 鬼ヶ島 (おにがしま) doesn't have a "proper" split,
            // so we just return what's left of the reading and headword,
            // consuming it all
            let reading_rem = reading;
            reading = "";
            for _ in &mut headword_parts {}
            return Some((part.rem, reading_rem));
        };

        //
        //     た べ る べ る あ い う
        //     ^^ --------------------
        //  pre | | post
        //
        let (_pre, mut post) = reading.split_at(first_peek_pos);

        // now we remove "ab"s until we reach a non-"ab" pattern
        let mut removed = 0usize;
        while let Some(rem) = post.strip_prefix(peek.text) {
            post = rem;
            removed += 1;
        }

        // this is so genuinely stupid but I can't think of a better way to do this
        removed = removed.checked_sub(1).expect(
            "we should have been able to strip at least one `peek.text` from `post`, \
                     since we found `peek.text` in `reading`",
        );

        //
        //             た べ る
        //             ^^ -----
        // part_reading | | rem
        //
        //       た べ る べ る あ い う
        //       ^^^^^^^^ --------------
        // part_reading | | post
        //

        let (part_reading, rem) = reading.split_at(first_peek_pos + peek.text.len() * removed);
        reading = rem;
        Some((part.text, part_reading))
    });
    furigana_parts.collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn term_segments() {
        assert_eq!(furigana_parts("する", ""), [("する", "")]);
        assert_eq!(furigana_parts("する", "する"), [("する", "")]);
        assert_eq!(furigana_parts("日本", "にほん"), [("日本", "にほん")]);
        assert_eq!(
            furigana_parts("食べる", "たべる"),
            [("食", "た"), ("べる", "")]
        );
        assert_eq!(
            furigana_parts("巻き込む", "まきこむ"),
            [("巻", "ま"), ("き", ""), ("込", "こ"), ("む", "")]
        );
        assert_eq!(
            furigana_parts("取り扱い説明書", "とりあつかいせつめいしょ"),
            [
                ("取", "と"),
                ("り", ""),
                ("扱", "あつか"),
                ("い", ""),
                ("説明書", "せつめいしょ")
            ]
        );
        assert_eq!(
            furigana_parts("お茶", "おちゃ"),
            [("お", ""), ("茶", "ちゃ")]
        );
        assert_eq!(
            furigana_parts("聞き流す", "ききながす"),
            [("聞", "き"), ("き", ""), ("流", "なが"), ("す", "")]
        );
        assert_eq!(
            super::furigana_parts("言い争い", "いいあらそい"),
            [("言", "い"), ("い", ""), ("争", "あらそ"), ("い", "")]
        );
        assert_eq!(
            super::furigana_parts("鬼ヶ島", "おにがしま"),
            [("鬼ヶ島", "おにがしま")]
        );
        assert_eq!(
            super::furigana_parts("食べる鬼ヶ島", "たべるおにがしま"),
            [("食", "た"), ("べる", ""), ("鬼ヶ島", "おにがしま")]
        );
        assert_eq!(
            furigana_parts("関係無い", "かんけいない"),
            [("関係無", "かんけいな"), ("い", "")]
        );

        // some cases are not solvable using our heuristic method
        // assert_eq!(
        //     furigana_parts("黄色い声", "きいろいこえ"),
        //     [("黄色", "きいろ"), ("い", ""), ("声", "こえ")]
        // );
    }

    #[test]
    fn morae_split() {
        fn splits_into(reading: &str, target: impl AsRef<[&'static str]>) {
            assert_eq!(&morae(reading).collect::<Vec<_>>(), target.as_ref());
        }

        splits_into("hello", ["h", "e", "l", "l", "o"]);
        splits_into("あいうえお", ["あ", "い", "う", "え", "お"]);
        splits_into("日本", ["日", "本"]);
        splits_into("ぎじゅつ", ["ぎ", "じゅ", "つ"]);
        splits_into("さぎょう", ["さ", "ぎょ", "う"]);
        splits_into("さっそく", ["さ", "っ", "そ", "く"]);
    }

    #[test]
    fn pitch_patterns() {
        fn follows_pattern(downstep: usize, target_pitches: [bool; 4]) {
            let actual_pitches = [
                is_high(downstep, 0),
                is_high(downstep, 1),
                is_high(downstep, 2),
                is_high(downstep, 3),
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
