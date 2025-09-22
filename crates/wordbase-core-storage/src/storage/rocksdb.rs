//! See [`Rocksdb`].

use {
    derive_more::Debug,
    eyre::{Context, OptionExt, Result, eyre},
    rocksdb::{
        ColumnFamily, DB, DBPinnableSlice, Env, MergeOperands, Options, WaitForCompactOptions,
        WriteOptions,
    },
    std::{iter, path::Path, sync::LazyLock},
    tokio::task::spawn_blocking,
    wordbase_core::{
        codec::Decoder,
        storage::{self, RecordRow},
    },
    wordbase_types::{RecordId, Term},
};

/// [`storage::Storage`] implementation which uses the [`rocksdb`] wrapper
/// around [RocksDB](https://rocksdb.org/).
#[derive(Debug)]
pub struct Rocksdb;

const RECORDS: &str = "records";
const HEADWORDS: &str = "headwords";
const READINGS: &str = "readings";

impl storage::Storage for Rocksdb {
    type Lookups = LookupStorage;

    #[expect(refining_impl_trait, reason = "explicit refinement")]
    async fn create_import_storage(data_dir: &Path) -> Result<ImportStorage> {
        let data_dir = data_dir.to_path_buf();
        spawn_blocking(move || {
            let env = Env::new().wrap_err("failed to create database env")?;

            let options = db_open_options(&env);
            let mut db = DB::open(&options, data_dir).wrap_err("failed to create database")?;
            db.create_cf(RECORDS, &options)
                .wrap_err_with(|| eyre!("failed to create column family `{RECORDS}`"))?;

            let options = term_cf_options(&env);
            db.create_cf(HEADWORDS, &options)
                .wrap_err_with(|| eyre!("failed to create column family `{HEADWORDS}`"))?;
            db.create_cf(READINGS, &options)
                .wrap_err_with(|| eyre!("failed to create column family `{RECORDS}`"))?;

            Ok(ImportStorage { db })
        })
        .await?
    }

    async fn open(data_dir: &Path) -> Result<Self::Lookups> {
        let data_dir = data_dir.to_path_buf();
        spawn_blocking(move || {
            let env = Env::new().wrap_err("failed to create database env")?;
            let column_families = [
                (RECORDS, db_open_options(&env)),
                (HEADWORDS, term_cf_options(&env)),
                (READINGS, term_cf_options(&env)),
            ];
            let db = DB::open_cf_with_opts_for_read_only(
                &db_open_options(&env),
                data_dir,
                column_families,
                false, // error_if_log_file_exist
            )
            .wrap_err("failed to open database")?;
            Ok(LookupStorage { db })
        })
        .await?
    }
}

fn db_open_options(env: &Env) -> Options {
    let mut options = Options::default();
    options.set_env(env);
    options.create_if_missing(true);
    // <https://github.com/facebook/rocksdb/wiki/RocksDB-FAQ>
    // "What's the fastest way to load data into RocksDB?"
    options.prepare_for_bulk_load();
    options
}

fn term_cf_options(env: &Env) -> Options {
    let mut options = db_open_options(env);
    options.set_merge_operator_associative("concat_terms", concat_terms);
    options
}

/// [`storage::ImportStorage`] for [`Rocksdb`].
#[derive(Debug)]
pub struct ImportStorage {
    db: DB,
}

impl storage::ImportStorage for ImportStorage {
    #[expect(refining_impl_trait, reason = "explicit refinement")]
    fn transaction(&mut self) -> Result<ImportTransaction<'_>> {
        Ok(ImportTransaction { db: &self.db })
    }

    fn commit(self) -> Result<()> {
        Ok(())
    }
}

/// [`storage::ImportTransaction`] for [`Rocksdb`].
#[derive(Debug)]
pub struct ImportTransaction<'stg> {
    db: &'stg DB,
}

impl<'stg> storage::ImportTransaction for ImportTransaction<'stg> {
    type Batch<'txn>
        = ImportBatch<'stg>
    where
        Self: 'txn;

    fn batch(&self) -> Result<Self::Batch<'_>> {
        Ok(ImportBatch {
            records: self
                .db
                .cf_handle(RECORDS)
                .ok_or_else(|| eyre!("no column family `{RECORDS}`"))?,
            headwords: self
                .db
                .cf_handle(HEADWORDS)
                .ok_or_else(|| eyre!("no column family `{HEADWORDS}`"))?,
            readings: self
                .db
                .cf_handle(READINGS)
                .ok_or_else(|| eyre!("no column family `{READINGS}`"))?,
            db: self.db,
        })
    }

    fn commit(self) -> Result<()> {
        self.db
            .flush()
            .wrap_err("failed to flush memtables to SST files")?;
        // <https://github.com/facebook/rocksdb/wiki/RocksDB-FAQ>
        // "What's the fastest way to load data into RocksDB?"
        self.db.compact_range(None::<&[u8]>, None::<&[u8]>);
        self.db
            .wait_for_compact(&WaitForCompactOptions::default())
            .wrap_err("failed to wait for compaction jobs")?;
        Ok(())
    }
}

/// [`storage::ImportBatch`] for [`Rocksdb`].
#[derive(Debug)]
pub struct ImportBatch<'stg> {
    #[debug(skip)]
    records: &'stg ColumnFamily,
    #[debug(skip)]
    headwords: &'stg ColumnFamily,
    #[debug(skip)]
    readings: &'stg ColumnFamily,
    db: &'stg DB,
}

impl storage::ImportBatch for ImportBatch<'_> {
    fn insert_record(&mut self, record_id: RecordId, record: &[u8]) -> Result<()> {
        put(self.db, self.records, &id_to_bytes(record_id), record)
            .wrap_err("failed to insert record")?;
        Ok(())
    }

    fn insert_term(&mut self, term: &Term, record_id: RecordId) -> Result<()> {
        let headword = term.headword().map(|s| s.as_str());
        let reading = term.reading().map(|s| s.as_str());

        if let Some(headword) = headword {
            let entry = TermEntry {
                record_id,
                secondary: reading.unwrap_or(""),
            };
            merge(
                self.db,
                self.headwords,
                headword.as_bytes(),
                &entry.encode().wrap_err("failed to encode headword")?,
            )
            .wrap_err("failed to insert headword")?;
        }
        if let Some(reading) = term.reading() {
            let entry = TermEntry {
                record_id,
                secondary: headword.unwrap_or(""),
            };
            merge(
                self.db,
                self.readings,
                reading.as_bytes(),
                &entry.encode().wrap_err("failed to encode reading")?,
            )
            .wrap_err("failed to insert reading")?;
        }
        Ok(())
    }
}

static WRITE_OPTIONS: LazyLock<WriteOptions> = LazyLock::new(|| {
    let mut opts = WriteOptions::new();
    opts.disable_wal(true);
    opts
});

fn put(db: &DB, cf: &ColumnFamily, key: &[u8], value: &[u8]) -> Result<()> {
    db.put_cf_opt(cf, key, value, &WRITE_OPTIONS)?;
    Ok(())
}

fn merge(db: &DB, cf: &ColumnFamily, key: &[u8], value: &[u8]) -> Result<()> {
    db.merge_cf_opt(cf, key, value, &WRITE_OPTIONS)?;
    Ok(())
}

#[expect(clippy::unnecessary_wraps, reason = "merge function signature")]
fn concat_terms(
    _new_key: &[u8],
    existing: Option<&[u8]>,
    operands: &MergeOperands,
) -> Option<Vec<u8>> {
    let mut buf = existing.map(<[_]>::to_vec).unwrap_or_default();
    for op in operands {
        buf.extend_from_slice(op);
    }
    Some(buf)
}

// Due to the way RocksDB works, when you insert a duplicate value for an
// existing key, the DB will merge the existing byte slice and the new byte
// slice you provide. We define the merge function ourselves, but we're only
// dealing with byte slices at that level.
//
// To make merges fast, we encode multiple `TermEntry::encode`s as a single
// contiguous byte slice with no separators. Therefore, to tell where one entry
// stops and the next starts, we keep track of the length of `secondary`.
#[derive(Debug, PartialEq, Eq)]
struct TermEntry<'a> {
    record_id: RecordId,
    secondary: &'a str,
}

// TODO: clean this dogshit up. octs 2.0 rewrite?
impl<'buf> TermEntry<'buf> {
    fn encode(&self) -> Result<Vec<u8>> {
        let i_secondary_len = size_of::<RecordId>();
        let i_secondary = i_secondary_len + size_of::<u32>();
        let len = i_secondary + self.secondary.len();
        let mut dst = vec![0; len];

        dst[..i_secondary_len].copy_from_slice(&id_to_bytes(self.record_id));

        dst[i_secondary_len..i_secondary].copy_from_slice(
            &u32::try_from(self.secondary.len())
                .wrap_err("term text length is larger than `u32::MAX`")?
                .to_le_bytes(),
        );

        dst[i_secondary..].copy_from_slice(self.secondary.as_bytes());

        Ok(dst)
    }

    fn decode(mut src: &'buf [u8]) -> Result<(Self, &'buf [u8])> {
        let (record_id, rest) = src
            .split_first_chunk::<{ size_of::<RecordId>() }>()
            .ok_or_eyre("buffer too short")?;
        let record_id = bytes_to_id(*record_id);
        src = rest;

        let (secondary_len, rest) = src
            .split_first_chunk::<{ size_of::<u32>() }>()
            .ok_or_eyre("buffer too short")?;
        let secondary_len = u32::from_le_bytes(*secondary_len);
        src = rest;

        let (secondary, rest) = src
            .split_at_checked(secondary_len as usize)
            .ok_or_else(|| eyre!("term length is {secondary_len} but buffer is shorter"))?;

        let secondary = str::from_utf8(secondary).wrap_err("term secondary is not UTF-8")?;
        Ok((
            Self {
                record_id,
                secondary,
            },
            rest,
        ))
    }
}

fn id_to_bytes(record_id: RecordId) -> [u8; size_of::<u64>()] {
    record_id.0.to_le_bytes()
}

fn bytes_to_id(bytes: [u8; size_of::<u64>()]) -> RecordId {
    RecordId(u64::from_le_bytes(bytes))
}

/// [`storage::LookupStorage`] for [`Rocksdb`].
#[derive(Debug)]
pub struct LookupStorage {
    db: DB,
}

impl storage::LookupStorage for LookupStorage {
    fn lookup<D: Decoder>(
        &self,
        make_decoder: impl Fn() -> D,
        lemma: &str,
    ) -> Result<Vec<RecordRow>> {
        let records = get_cf(&self.db, RECORDS)?;
        let headwords = get_cf(&self.db, HEADWORDS)?;
        let readings = get_cf(&self.db, READINGS)?;

        let headword_terms_blob = self
            .db
            .get_pinned_cf(headwords, lemma.as_bytes())
            .wrap_err("failed to get record IDs for headword")?;
        let reading_terms_blob = self
            .db
            .get_pinned_cf(readings, lemma.as_bytes())
            .wrap_err("failed to get record IDs for reading")?;

        let records = iter::empty()
            .chain(
                headword_terms_blob
                    .iter()
                    .map(|blob| (TermPart::Headword, blob)),
            )
            .chain(
                reading_terms_blob
                    .iter()
                    .map(|blob| (TermPart::Reading, blob)),
            )
            .flat_map(move |(term_part, blob)| {
                self.get_by_ids(make_decoder(), records, blob, lemma, term_part)
                    .map(move |r| r.wrap_err_with(|| eyre!("failed to query {term_part:?}")))
            });

        records.collect::<Result<Vec<_>, _>>()
    }
}

#[derive(Debug, Clone, Copy)]
enum TermPart {
    Headword,
    Reading,
}

impl LookupStorage {
    fn get_by_ids<'db, 'blob: 'db>(
        &'db self,
        mut decoder: impl Decoder,
        records: &'db ColumnFamily,
        terms_blob: &'blob DBPinnableSlice<'db>,
        lemma: &'db str,
        term_part: TermPart,
    ) -> impl Iterator<Item = Result<RecordRow>> + 'db {
        let mut get_record = move |id: RecordId| {
            let blob = self
                .db
                .get_pinned_cf(records, id_to_bytes(id))
                .wrap_err_with(|| eyre!("failed to get {id:?}"))?
                .ok_or_else(|| eyre!("no record {id:?}"))?;
            let record = decoder
                .decode(&blob)
                .wrap_err_with(|| eyre!("failed to decode {id:?}"))?;
            eyre::Ok(record)
        };

        entries_in(terms_blob).map(move |entry| {
            let entry = entry?;
            let term = match term_part {
                TermPart::Headword => Term::from_full(lemma, entry.secondary),
                TermPart::Reading => Term::from_full(entry.secondary, lemma),
            }?;
            let record = get_record(entry.record_id)?;
            Ok(RecordRow {
                term,
                record_id: entry.record_id,
                record,
            })
        })
    }
}

fn get_cf<'db>(db: &'db DB, name: &str) -> Result<&'db ColumnFamily> {
    db.cf_handle(name)
        .ok_or_else(|| eyre!("no column family `{name}"))
}

fn entries_in(mut buf: &[u8]) -> impl Iterator<Item = Result<TermEntry<'_>>> {
    iter::from_fn(move || {
        if buf.is_empty() {
            return None;
        }
        let (entry, rest) = match TermEntry::decode(buf) {
            Ok(x) => x,
            Err(err) => return Some(Err(err).wrap_err("failed to decode entry")),
        };
        buf = rest;
        Some(Ok(entry))
    })
}

#[cfg(test)]
mod tests {
    use {
        super::{TermEntry, entries_in},
        wordbase_types::RecordId,
    };

    #[test]
    fn round_trip_entry() {
        let entries = [
            TermEntry {
                record_id: RecordId(123),
                secondary: "foo",
            },
            TermEntry {
                record_id: RecordId(456),
                secondary: "bar",
            },
        ];
        let buf = entries
            .iter()
            .flat_map(|entry| entry.encode().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            &entries[..],
            &entries_in(&buf)
                .collect::<Result<Vec<TermEntry>, _>>()
                .unwrap()[..]
        );
    }
}
