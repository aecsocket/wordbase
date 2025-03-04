use std::{
    io::{Read, Seek},
    marker::PhantomData,
    sync::LazyLock,
};

use derive_more::{Display, Error};
use rayon::prelude::*;
use regex::Regex;
use serde::de::DeserializeOwned;
use zip::ZipArchive;

use super::{Index, KanjiBank, KanjiMetaBank, TagBank, TermBank, TermMetaBank};

pub use serde_json::Error as JsonError;
pub use zip::result::ZipError;

pub struct Parse<F, R, E> {
    new_reader: F,
    tag_banks: Vec<String>,
    term_banks: Vec<String>,
    term_meta_banks: Vec<String>,
    kanji_banks: Vec<String>,
    kanji_meta_banks: Vec<String>,
    _phantom: PhantomData<fn() -> Result<R, E>>,
}

const INDEX_PATH: &str = "index.json";

static TAG_BANK_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("tag_bank_([0-9]+?)\\.json").expect("should be valid regex"));

static TERM_BANK_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("term_bank_([0-9]+?)\\.json").expect("should be valid regex"));

static TERM_META_BANK_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("term_meta_bank_([0-9]+?)\\.json").expect("should be valid regex"));

static KANJI_BANK_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("kanji_bank_([0-9]+?)\\.json").expect("should be valid regex"));

static KANJI_META_BANK_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new("kanji_meta_bank_([0-9]+?)\\.json").expect("should be valid regex")
});

#[derive(Debug, Display, Error)]
pub enum ParseError<E> {
    #[display("failed to create new archive reader")]
    NewReader(E),
    #[display("failed to open archive")]
    OpenArchive(ZipError),
    #[display("failed to open {name:?}")]
    OpenEntry { name: String, source: ZipError },
    #[display("failed to parse {name:?}")]
    ParseEntry { name: String, source: JsonError },
}

impl<F, R, E> Parse<F, R, E>
where
    F: Fn() -> Result<R, E> + Sync,
    R: Read + Seek,
    E: Send,
{
    fn new_archive(new_reader: &F) -> Result<ZipArchive<R>, ParseError<E>> {
        let reader = new_reader().map_err(ParseError::NewReader)?;
        let archive = ZipArchive::new(reader).map_err(ParseError::OpenArchive)?;
        Ok(archive)
    }

    pub fn new(new_reader: F) -> Result<(Self, Index), ParseError<E>> {
        let mut archive = Self::new_archive(&new_reader)?;
        let index = parse_from::<Index, E>(&mut archive, INDEX_PATH)?;

        let (
            mut tag_banks,
            mut term_banks,
            mut term_meta_banks,
            mut kanji_banks,
            mut kanji_meta_banks,
        ) = (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());

        for name in archive.file_names() {
            if TAG_BANK_PATTERN.is_match(name) {
                tag_banks.push(name.into());
            } else if TERM_BANK_PATTERN.is_match(name) {
                term_banks.push(name.into());
            } else if TERM_META_BANK_PATTERN.is_match(name) {
                term_meta_banks.push(name.into());
            } else if KANJI_BANK_PATTERN.is_match(name) {
                kanji_banks.push(name.into());
            } else if KANJI_META_BANK_PATTERN.is_match(name) {
                kanji_meta_banks.push(name.into());
            }
        }

        Ok((
            Self {
                new_reader,
                tag_banks,
                term_banks,
                term_meta_banks,
                kanji_banks,
                kanji_meta_banks,
                _phantom: PhantomData,
            },
            index,
        ))
    }

    pub fn tag_banks(&self) -> &[String] {
        &self.tag_banks[..]
    }

    pub fn term_banks(&self) -> &[String] {
        &self.term_banks[..]
    }

    pub fn term_meta_banks(&self) -> &[String] {
        &self.term_meta_banks[..]
    }

    pub fn kanji_banks(&self) -> &[String] {
        &self.kanji_banks[..]
    }

    pub fn kanji_meta_banks(&self) -> &[String] {
        &self.kanji_meta_banks[..]
    }

    fn run_on<B: DeserializeOwned>(
        new_reader: &F,
        bank: Vec<String>,
        use_fn: impl Fn(&str, B) + Sync,
    ) -> Result<(), ParseError<E>> {
        bank.into_par_iter().try_for_each(|name| {
            let mut archive = Self::new_archive(new_reader)?;
            let bank = parse_from::<B, E>(&mut archive, &name)?;
            use_fn(&name, bank);
            Ok::<_, ParseError<E>>(())
        })
    }

    pub fn run(
        self,
        use_tag_bank: impl Fn(&str, TagBank) + Send + Sync,
        use_term_bank: impl Fn(&str, TermBank) + Send + Sync,
        use_term_meta_bank: impl Fn(&str, TermMetaBank) + Send + Sync,
        use_kanji_bank: impl Fn(&str, KanjiBank) + Send + Sync,
        use_kanji_meta_bank: impl Fn(&str, KanjiMetaBank) + Send + Sync,
    ) -> Result<(), ParseError<E>> {
        let new_reader = &self.new_reader;
        let (
            (tag_bank_result, (term_bank_result, term_meta_bank_result)),
            (kanji_bank_result, kanji_meta_bank_result),
        ) = rayon::join(
            || {
                rayon::join(
                    || Self::run_on::<TagBank>(new_reader, self.tag_banks, use_tag_bank),
                    || {
                        rayon::join(
                            || Self::run_on::<TermBank>(new_reader, self.term_banks, use_term_bank),
                            || {
                                Self::run_on::<TermMetaBank>(
                                    new_reader,
                                    self.term_meta_banks,
                                    use_term_meta_bank,
                                )
                            },
                        )
                    },
                )
            },
            || {
                rayon::join(
                    || Self::run_on::<KanjiBank>(new_reader, self.kanji_banks, use_kanji_bank),
                    || {
                        Self::run_on::<KanjiMetaBank>(
                            new_reader,
                            self.kanji_meta_banks,
                            use_kanji_meta_bank,
                        )
                    },
                )
            },
        );
        tag_bank_result?;
        term_bank_result?;
        term_meta_bank_result?;
        kanji_bank_result?;
        kanji_meta_bank_result?;
        Ok(())
    }
}

fn parse_from<T: DeserializeOwned, E>(
    archive: &mut ZipArchive<impl Read + Seek>,
    name: &str,
) -> Result<T, ParseError<E>> {
    let file = archive
        .by_name(name)
        .map_err(|source| ParseError::OpenEntry {
            name: name.into(),
            source,
        })?;
    let value = serde_json::from_reader::<_, T>(file).map_err(|source| ParseError::ParseEntry {
        name: name.into(),
        source,
    })?;
    Ok(value)
}

// #[cfg(test)]
// mod tests {
//     use std::{io::Cursor, sync::Mutex};

//     use crate::yomitan::{Glossary, KanjiMetaBank};

//     use super::*;

//     #[test]
//     fn jitendex() {
//         let (index, tags, terms, term_metas, kanjis, kanji_metas) =
//             parse(include_bytes!("../../../../dictionaries/jitendex.zip"));
//         assert!(index.title.contains("Jitendex.org"));
//         assert!(tags.iter().any(|tag| {
//             tag.name == "★" && tag.category == "popular" && tag.notes == "high priority entry"
//         }));
//         assert!(
//             terms
//                 .iter()
//                 .any(|term| { term.expression == "天" && term.reading == "てん" })
//         );
//         assert!(term_metas.is_empty());
//         assert!(kanjis.is_empty());
//         assert!(kanji_metas.is_empty());
//     }

//     #[test]
//     fn jmnedict() {
//         let (index, tags, terms, term_metas, kanjis, kanji_metas) =
//             parse(include_bytes!("../../../../dictionaries/jmnedict.zip"));
//         assert!(index.title.contains("JMnedict"));
//         assert!(tags.iter().any(|tag| {
//             tag.name == "given"
//                 && tag.category == "name"
//                 && tag.notes == "given name or forename, gender not specified"
//         }));
//         assert!(terms.iter().any(|term| {
//             term.expression == "菊池大麓"
//                 && term.reading == "きくちだいろく"
//                 && term.definition_tags == Some("person".into())
//                 && matches!(
//                     term.glossary.first(),
//                     Some(Glossary::String(s))
//                     if s == "Kikuchi Dairoku"
//                 )
//         }));
//         assert!(term_metas.is_empty());
//         assert!(kanjis.is_empty());
//         assert!(kanji_metas.is_empty());
//     }

//     #[test]
//     fn dojg() {
//         parse(include_bytes!("../../../../dictionaries/dojg.zip"));
//     }

//     fn parse(
//         bytes: &[u8],
//     ) -> (
//         Index,
//         TagBank,
//         TermBank,
//         TermMetaBank,
//         KanjiBank,
//         KanjiMetaBank,
//     ) {
//         let (parser, index) = Parse::new(|| Ok(Cursor::new(bytes))).unwrap();
//         let tags = Mutex::new(TagBank::default());
//         let terms = Mutex::new(TermBank::default());
//         let term_metas = Mutex::new(TermMetaBank::default());
//         let kanjis = Mutex::new(KanjiBank::default());
//         let kanji_metas = Mutex::new(KanjiMetaBank::default());
//         parser
//             .run(
//                 |_, bank| {
//                     tags.lock().unwrap().extend_from_slice(&bank);
//                 },
//                 |_, bank| {
//                     terms.lock().unwrap().extend_from_slice(&bank);
//                 },
//                 |_, bank| {
//                     term_metas.lock().unwrap().extend_from_slice(&bank);
//                 },
//                 |_, bank| {
//                     kanjis.lock().unwrap().extend_from_slice(&bank);
//                 },
//                 |_, bank| {
//                     kanji_metas.lock().unwrap().extend_from_slice(&bank);
//                 },
//             )
//             .unwrap();
//         (
//             index,
//             tags.into_inner().unwrap(),
//             terms.into_inner().unwrap(),
//             term_metas.into_inner().unwrap(),
//             kanjis.into_inner().unwrap(),
//             kanji_metas.into_inner().unwrap(),
//         )
//     }
// }
