use core::marker::PhantomData;
use std::{
    io::{Read, Seek},
    sync::LazyLock,
};

use derive_more::{Display, Error};
use rayon::prelude::*;
use regex::Regex;
use serde::de::DeserializeOwned;
use zip::ZipArchive;

use super::{Index, TagBank, TermBank, TermMetaBank};

pub use serde_json::Error as JsonError;
pub use zip::result::ZipError;

pub struct Parse<F, R> {
    new_reader: F,
    tag_bank_names: Vec<String>,
    term_bank_names: Vec<String>,
    term_meta_bank_names: Vec<String>,
    _phantom: PhantomData<fn() -> R>,
}

#[derive(Debug, Display, Error)]
pub enum ParseError {
    #[display("failed to create new archive reader")]
    NewReader(Box<dyn core::error::Error + Send + Sync>),
    #[display("failed to open archive")]
    OpenArchive(ZipError),
    #[display("failed to open {name:?}")]
    OpenEntry { name: String, source: ZipError },
    #[display("failed to parse {name:?}")]
    ParseEntry { name: String, source: JsonError },
}

const INDEX_PATH: &str = "index.json";

static TAG_BANK_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("tag_bank_([0-9]+?)\\.json").expect("should be valid regex"));

static TERM_BANK_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("term_bank_([0-9]+?)\\.json").expect("should be valid regex"));

static TERM_META_BANK_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("term_meta_bank_([0-9]+?)\\.json").expect("should be valid regex"));

impl<F, R> Parse<F, R>
where
    F: Fn() -> Result<R, Box<dyn core::error::Error + Send + Sync>> + Sync,
    R: Read + Seek,
{
    fn new_archive(new_reader: &F) -> Result<ZipArchive<R>, ParseError> {
        let reader = new_reader().map_err(ParseError::NewReader)?;
        let archive = ZipArchive::new(reader).map_err(ParseError::OpenArchive)?;
        Ok(archive)
    }

    pub fn new(new_reader: F) -> Result<(Self, Index), ParseError> {
        let mut archive = Self::new_archive(&new_reader)?;
        let index = parse_from::<Index>(&mut archive, INDEX_PATH)?;

        let (mut tag_bank_names, mut term_bank_names, mut term_meta_bank_names) =
            (Vec::new(), Vec::new(), Vec::new());

        for name in archive.file_names() {
            if TAG_BANK_PATTERN.is_match(name) {
                tag_bank_names.push(name.into());
            } else if TERM_BANK_PATTERN.is_match(name) {
                term_bank_names.push(name.into());
            } else if TERM_META_BANK_PATTERN.is_match(name) {
                term_meta_bank_names.push(name.into());
            }
        }

        Ok((
            Self {
                new_reader,
                tag_bank_names,
                term_bank_names,
                term_meta_bank_names,
                _phantom: PhantomData,
            },
            index,
        ))
    }

    pub fn tag_bank_names(&self) -> &[String] {
        &self.tag_bank_names[..]
    }

    pub fn term_bank_names(&self) -> &[String] {
        &self.term_bank_names[..]
    }

    pub fn term_meta_bank_names(&self) -> &[String] {
        &self.term_meta_bank_names[..]
    }

    pub fn run(
        self,
        use_tag_bank: impl Fn(&str, TagBank) + Sync,
        use_term_bank: impl Fn(&str, TermBank) + Sync,
        use_term_meta_bank: impl Fn(&str, TermMetaBank) + Sync,
    ) -> Result<(), ParseError> {
        let (tag_bank_result, (term_bank_result, term_meta_bank_result)) = rayon::join(
            || {
                self.tag_bank_names.into_par_iter().try_for_each(|name| {
                    let mut archive = Self::new_archive(&self.new_reader)?;
                    let bank = parse_from::<TagBank>(&mut archive, &name)?;
                    use_tag_bank(&name, bank);
                    Ok::<_, ParseError>(())
                })
            },
            || {
                rayon::join(
                    || {
                        self.term_bank_names.into_par_iter().try_for_each(|name| {
                            let mut archive = Self::new_archive(&self.new_reader)?;
                            let bank = parse_from::<TermBank>(&mut archive, &name)?;
                            use_term_bank(&name, bank);
                            Ok::<_, ParseError>(())
                        })
                    },
                    || {
                        self.term_meta_bank_names
                            .into_par_iter()
                            .try_for_each(|name| {
                                let mut archive = Self::new_archive(&self.new_reader)?;
                                let bank = parse_from::<TermMetaBank>(&mut archive, &name)?;
                                use_term_meta_bank(&name, bank);
                                Ok::<_, ParseError>(())
                            })
                    },
                )
            },
        );
        tag_bank_result?;
        term_bank_result?;
        term_meta_bank_result?;
        Ok(())
    }
}

fn parse_from<T: DeserializeOwned>(
    archive: &mut ZipArchive<impl Read + Seek>,
    name: &str,
) -> Result<T, ParseError> {
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

#[cfg(test)]
mod tests {
    use std::{io::Cursor, sync::Mutex};

    use crate::dictionary::yomitan::Glossary;

    use super::*;

    #[test]
    fn jitendex() {
        let (index, tags, terms, term_metas) =
            parse(include_bytes!("../../../../../dictionaries/jitendex.zip"));
        assert!(index.title.contains("Jitendex.org"));
        assert!(tags.iter().any(|tag| {
            tag.name == "★" && tag.category == "popular" && tag.notes == "high priority entry"
        }));
        assert!(
            terms
                .iter()
                .any(|term| { term.expression == "天" && term.reading == "てん" })
        );
        assert!(term_metas.is_empty());
    }

    #[test]
    fn jmnedict() {
        let (index, tags, terms, term_metas) =
            parse(include_bytes!("../../../../../dictionaries/jmnedict.zip"));
        assert!(index.title.contains("JMnedict"));
        assert!(tags.iter().any(|tag| {
            tag.name == "given"
                && tag.category == "name"
                && tag.notes == "given name or forename, gender not specified"
        }));
        assert!(terms.iter().any(|term| {
            term.expression == "菊池大麓"
                && term.reading == "きくちだいろく"
                && term.definition_tags == Some("person".into())
                && matches!(
                    term.glossary.first(),
                    Some(Glossary::String(s))
                    if s == "Kikuchi Dairoku"
                )
        }));
        assert!(term_metas.is_empty());
    }

    #[test]
    fn dojg() {
        parse(include_bytes!("../../../../../dictionaries/dojg.zip"));
    }

    fn parse(bytes: &[u8]) -> (Index, TagBank, TermBank, TermMetaBank) {
        let (parser, index) = Parse::new(|| Ok(Cursor::new(bytes))).unwrap();
        let tags = Mutex::new(TagBank::default());
        let terms = Mutex::new(TermBank::default());
        let term_metas = Mutex::new(TermMetaBank::default());
        parser
            .run(
                |_, bank| {
                    tags.lock().unwrap().extend_from_slice(&bank);
                },
                |_, bank| {
                    terms.lock().unwrap().extend_from_slice(&bank);
                },
                |_, bank| {
                    term_metas.lock().unwrap().extend_from_slice(&bank);
                },
            )
            .unwrap();
        (
            index,
            tags.into_inner().unwrap(),
            terms.into_inner().unwrap(),
            term_metas.into_inner().unwrap(),
        )
    }
}
