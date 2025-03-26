use {
    super::schema::{Index, KanjiBank, KanjiMetaBank, TagBank, TermBank, TermMetaBank},
    crate::import::yomitan::INDEX_PATH,
    anyhow::Result,
    derive_more::{Display, Error},
    rayon::prelude::*,
    regex::Regex,
    serde::de::DeserializeOwned,
    std::{
        io::{Read, Seek},
        marker::PhantomData,
        sync::LazyLock,
    },
    zip::ZipArchive,
};
pub use {serde_json::Error as JsonError, zip::result::ZipError};

/// Parses a [Yomitan] dictionary from a zip archive.
///
/// # Example
///
/// ```
/// # use wordbase::format::yomitan::Parse;
/// # fn run() {
/// let archive = std::fs::read("jitendex.zip").expect("failed to read dictionary to memory");
///
/// let (parser, index) =
///     Parse::new(|| Ok(std::io::Cursor::new(&archive))).expect("failed to parse index");
///
/// let term_banks_left = AtomicUsize::new(index.term_banks().len());
///
/// parser
///     .run(
///         |_, _tag_bank| {},
///         |_, term_bank| {
///             let left = term_banks_left.fetch_sub(1, atomic::Ordering::SeqCst) - 1;
///             println!("{left} term banks left to parse");
///         },
///         |_, _term_meta_bank| {},
///         |_, _kanji_bank| {},
///         |_, _kanji_meta_bank| {},
///     )
///     .expect("failed to parse bank");
/// # }
/// ```
///
/// [Yomitan]: super
pub struct Parse<F, R, E> {
    new_reader: F,
    tag_banks: Vec<String>,
    term_banks: Vec<String>,
    term_meta_banks: Vec<String>,
    kanji_banks: Vec<String>,
    kanji_meta_banks: Vec<String>,
    _phantom: PhantomData<fn() -> Result<R, E>>,
}

/// [`Parse`] error.
#[derive(Debug, Display, Error)]
pub enum ParseError<E> {
    /// Failed to create a new archive reader.
    #[display("failed to create new archive reader")]
    NewReader(E),
    /// Failed to open the given reader as a zip archive.
    #[display("failed to open archive")]
    OpenArchive(ZipError),
    /// Failed to open an entry in the archive.
    #[display("failed to open {name:?}")]
    OpenEntry {
        /// File name of the entry.
        name: String,
        /// Source error.
        source: ZipError,
    },
    /// Failed to parse an entry in the archive as a bank.
    #[display("failed to parse {name:?}")]
    ParseEntry {
        /// File name of the entry.
        name: String,
        /// Source error.
        source: JsonError,
    },
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

    /// Creates a new parser and parses the dictionary [`Index`].
    ///
    /// `new_reader` is a function which creates a new `R` reader used to open
    /// the archive. If this function returns `E`, [`ParseError::NewReader`] is
    /// returned.
    ///
    /// # Errors
    ///
    /// Errors if the index could not be parsed.
    pub fn new(new_reader: F) -> Result<(Self, Index), ParseError<E>> {
        static TAG_BANK_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new("tag_bank_([0-9]+?)\\.json").expect("should be valid regex")
        });

        static TERM_BANK_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new("term_bank_([0-9]+?)\\.json").expect("should be valid regex")
        });

        static TERM_META_BANK_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new("term_meta_bank_([0-9]+?)\\.json").expect("should be valid regex")
        });

        static KANJI_BANK_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new("kanji_bank_([0-9]+?)\\.json").expect("should be valid regex")
        });

        static KANJI_META_BANK_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new("kanji_meta_bank_([0-9]+?)\\.json").expect("should be valid regex")
        });

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

    /// [`TagBank`] file names in the archive.
    pub fn tag_banks(&self) -> &[String] {
        &self.tag_banks[..]
    }

    /// [`TermBank`] file names in the archive.
    pub fn term_banks(&self) -> &[String] {
        &self.term_banks[..]
    }

    /// [`TermMetaBank`] file names in the archive.
    pub fn term_meta_banks(&self) -> &[String] {
        &self.term_meta_banks[..]
    }

    /// [`KanjiBank`] file names in the archive.
    pub fn kanji_banks(&self) -> &[String] {
        &self.kanji_banks[..]
    }

    /// [`KanjiMetaBank`] file names in the archive.
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

    /// Parses all banks in the archive.
    ///
    /// This uses [`rayon`] to parallelise parsing, creating a new reader for
    /// each bank, and returning the result to your own callbacks. Therefore,
    /// these callbacks will run on a different thread to the one that calls
    /// [`Parse::run`].
    ///
    /// # Errors
    ///
    /// Errors if one of the banks could not be parsed, terminating early.
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
