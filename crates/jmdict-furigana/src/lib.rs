#![doc = include_str!("../README.md")]

use std::{io::Cursor, sync::OnceLock};

use async_zip::base::read::seek::ZipFileReader;
use foldhash::HashMap;
use futures::AsyncReadExt;

pub type Term<'a> = (&'a str, &'a str);
pub type Furigana<'a> = &'a [(&'a str, &'a str)];

pub fn get<'a>(headword: &str, reading: &str) -> Option<Furigana<'static>> {
    let entries = ENTRIES
        .get()
        .unwrap_or_else(|| futures::executor::block_on(entries()));
    entries.get(&(headword, reading)).copied()
}

type EntryMap = HashMap<Term<'static>, Furigana<'static>>;

static ENTRIES: OnceLock<EntryMap> = OnceLock::new();

#[expect(clippy::missing_panics_doc, reason = "shouldn't panic")]
pub async fn entries() -> &'static EntryMap {
    if let Some(entries) = ENTRIES.get() {
        return entries;
    }

    let archive = include_bytes!("jmdict_furigana.json.zip");
    let archive = ZipFileReader::with_tokio(Cursor::new(&archive[..]))
        .await
        .expect("failed to read archive as zip file");

    let mut archive_entry = archive.into_entry(0).await.expect("no entry in archive");
    let mut archive_str = String::new();
    archive_entry
        .read_to_string(&mut archive_str)
        .await
        .expect("failed to read archive entry to string");
    let archive_str = archive_str.leak();
    let archive_str = archive_str.strip_prefix("\u{feff}").unwrap_or(archive_str);

    let entries = serde_json::from_str::<Vec<schema::Entry>>(archive_str)
        .expect("failed to parse archive entry as JSON")
        .into_iter()
        .map(|entry| {
            (
                (entry.text, entry.reading),
                entry
                    .furigana
                    .into_iter()
                    .map(|furigana| (furigana.ruby, furigana.rt.unwrap_or_default()))
                    .collect::<Vec<_>>()
                    .leak() as &[_],
            )
        })
        .collect();
    ENTRIES.get_or_init(|| entries)
}

/// Initializes the entry map, reading and parsing it into memory.
///
/// Call this before [`get`] to avoid blocking in [`get`].
pub async fn init() {
    entries().await;
}

mod schema {
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    pub struct Entry {
        pub text: &'static str,
        pub reading: &'static str,
        pub furigana: Vec<Furigana>,
    }

    #[derive(Debug, Deserialize)]
    pub struct Furigana {
        pub ruby: &'static str,
        pub rt: Option<&'static str>,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn foo() {
        assert_eq!(
            *get("食べる", "たべる").unwrap(),
            vec![("食", "た"), ("べる", "")]
        );
        assert_eq!(*get("大人", "おとな").unwrap(), vec![("大人", "おとな")]);
        assert_eq!(
            *get("関係無い", "かんけいない").unwrap(),
            [("関", "かん"), ("係", "けい"), ("無", "な"), ("い", "")]
        );
        assert_eq!(
            *get("黄色い声", "きいろいこえ").unwrap(),
            [("黄", "き"), ("色", "いろ"), ("い", ""), ("声", "こえ")]
        );
    }
}
