use {
    super::schema::{Index, Kanji, KanjiMeta, Tag, Term, TermMeta},
    crate::import::yomitan::INDEX_PATH,
    anyhow::{Context, Result},
    bytes::Bytes,
    rayon::prelude::*,
    regex::Regex,
    serde::de::DeserializeOwned,
    std::{
        io::{Cursor, Read, Seek},
        sync::{
            LazyLock,
            atomic::{self, AtomicUsize},
        },
    },
    tokio::sync::mpsc,
    zip::ZipArchive,
};

pub fn start_blocking(archive: Bytes) -> Result<(ParseBanks, Index)> {
    macro_rules! re {
        ($re:expr) => {
            LazyLock::new(|| Regex::new($re).expect("should be valid regex"))
        };
    }

    static TAG_BANK_PATTERN: LazyLock<Regex> = re!("tag_bank_([0-9]+?)\\.json");
    static TERM_BANK_PATTERN: LazyLock<Regex> = re!("term_bank_([0-9]+?)\\.json");
    static TERM_META_BANK_PATTERN: LazyLock<Regex> = re!("term_meta_bank_([0-9]+?)\\.json");
    static KANJI_BANK_PATTERN: LazyLock<Regex> = re!("kanji_bank_([0-9]+?)\\.json");
    static KANJI_META_BANK_PATTERN: LazyLock<Regex> = re!("kanji_meta_bank_([0-9]+?)\\.json");

    let mut zip = ZipArchive::new(Cursor::new(&archive)).context("failed to open archive")?;
    let index = parse_archive_file::<Index>(&mut zip, INDEX_PATH)?;

    let (mut tag_banks, mut term_banks, mut term_meta_banks, mut kanji_banks, mut kanji_meta_banks) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());

    for name in zip.file_names() {
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
        ParseBanks {
            archive,
            tag_banks,
            term_banks,
            term_meta_banks,
            kanji_banks,
            kanji_meta_banks,
        },
        index,
    ))
}

fn parse_archive_file<T: DeserializeOwned>(
    archive: &mut ZipArchive<impl Read + Seek>,
    name: &str,
) -> Result<T> {
    let file = archive
        .by_name(name)
        .with_context(|| format!("failed to open `{name}`"))?;
    let value = serde_json::from_reader::<_, T>(file)
        .with_context(|| format!("failed to parse `{name}`"))?;
    Ok(value)
}

pub struct ParseBanks {
    archive: Bytes,
    tag_banks: Vec<String>,
    term_banks: Vec<String>,
    term_meta_banks: Vec<String>,
    kanji_banks: Vec<String>,
    kanji_meta_banks: Vec<String>,
}

macro_rules! join {
    ($a:expr, $b:expr $(,)?) => {
        rayon::join($a, $b)
    };
    ($a:expr, $b:expr, $c:expr $(,)?) => {{
        let ((a, b), c) = rayon::join(|| join!($a, $b), $c);
        (a, b, c)
    }};
    ($a:expr, $b:expr, $c:expr, $d:expr $(,)?) => {{
        let ((a, b, c), d) = rayon::join(|| join!($a, $b, $c), $d);
        (a, b, c, d)
    }};
    ($a:expr, $b:expr, $c:expr, $d:expr, $e:expr $(,)?) => {{
        let ((a, b, c, d), e) = rayon::join(|| join!($a, $b, $c, $d), $e);
        (a, b, c, d, e)
    }};
}

impl ParseBanks {
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

    pub fn parse_blocking(
        self,
        banks_done: &AtomicUsize,
        send_bank_done: &mpsc::Sender<()>,
    ) -> Result<ParsedBanks> {
        let archive = &self.archive;
        let (tag, term, term_meta, kanji, kanji_meta) = join!(
            || parse_banks::<Tag>(archive, self.tag_banks, banks_done, send_bank_done),
            || parse_banks::<Term>(archive, self.term_banks, banks_done, send_bank_done),
            || parse_banks::<TermMeta>(archive, self.term_meta_banks, banks_done, send_bank_done),
            || parse_banks::<Kanji>(archive, self.kanji_banks, banks_done, send_bank_done),
            || parse_banks::<KanjiMeta>(archive, self.kanji_meta_banks, banks_done, send_bank_done),
        );
        Ok(ParsedBanks {
            tag: tag.context("failed to parse tag banks")?,
            term: term.context("failed to parse term banks")?,
            term_meta: term_meta.context("failed to parse term meta banks")?,
            kanji: kanji.context("failed to parse kanji banks")?,
            kanji_meta: kanji_meta.context("failed to parse kanji meta banks")?,
        })
    }
}

#[derive(Debug)]
pub struct ParsedBanks {
    pub tag: Vec<Tag>,
    pub term: Vec<Term>,
    pub term_meta: Vec<TermMeta>,
    pub kanji: Vec<Kanji>,
    pub kanji_meta: Vec<KanjiMeta>,
}

fn parse_banks<T: Send + Sync + DeserializeOwned>(
    archive: &[u8],
    bank_names: Vec<String>,
    banks_done: &AtomicUsize,
    send_bank_done: &mpsc::Sender<()>,
) -> Result<Vec<T>> {
    bank_names
        .into_par_iter()
        .map(|name| {
            let mut archive =
                ZipArchive::new(Cursor::new(archive)).context("failed to open archive")?;
            let bank = parse_archive_file::<Vec<T>>(&mut archive, &name)?;
            banks_done.fetch_add(1, atomic::Ordering::SeqCst);
            _ = send_bank_done.try_send(());
            anyhow::Ok(bank)
        })
        .try_fold(Vec::new, |mut acc, bank| {
            acc.append(&mut bank?);
            Ok(acc)
        })
        .try_reduce(Vec::new, |mut acc, mut bank| {
            acc.append(&mut bank);
            Ok(acc)
        })
}
