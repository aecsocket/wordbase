#![doc = include_str!("../README.md")]

use std::sync::OnceLock;

use async_zip::base::read::seek::ZipFileReader;
use foldhash::HashMap;
use futures::{AsyncReadExt, io::Cursor};

type Term<'a> = (&'a str, &'a str);
type Furigana<'a> = &'a [(&'a str, &'a str)];
type EntryMap = HashMap<Term<'static>, Furigana<'static>>;

/// Gets furigana sections for the given headword/reading pair.
///
/// # Panics
///
/// Panics if the furigana map has not yet been [initialized][init].
#[must_use]
pub fn get(headword: &str, reading: &str) -> Option<Furigana<'static>> {
    entries().get(&(headword, reading)).copied()
}

/// Gets the map of all headword/reading pairs to furigana sections.
///
/// # Panics
///
/// Panics if the furigana map has not yet been [initialized][init].
#[must_use]
pub fn entries() -> &'static EntryMap {
    ENTRIES.get().expect(
        "furigana entry map is not initialized yet - \
        make sure to call `jmdict_furigana::init()` before `get()`",
    )
}

/// Initializes the entry map, reading and parsing it into memory.
///
/// Call this before reading the entry map.
#[expect(clippy::missing_panics_doc, reason = "shouldn't panic")]
pub async fn init() {
    if ENTRIES.get().is_some() {
        return;
    }

    let archive = include_bytes!("jmdict_furigana.json.zip");
    let archive = ZipFileReader::new(Cursor::new(&archive[..]))
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
    _ = ENTRIES.set(entries);
}

static ENTRIES: OnceLock<EntryMap> = OnceLock::new();

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
    #[should_panic = "furigana entry map is not initialized yet - make sure to call `jmdict_furigana::init()` before `get()`"]
    fn get_before_init() {
        _ = get("", "");
    }

    #[tokio::test]
    async fn furigana() {
        init().await;
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
