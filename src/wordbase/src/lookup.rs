use {
    derive_more::{Deref, DerefMut},
    eyre::{Context, Result, eyre},
    redb::{
        AccessGuard, MultimapTableHandle as _, ReadOnlyDatabase, ReadOnlyMultimapTable,
        ReadOnlyTable, ReadTransaction, ReadableDatabase as _, ReadableMultimapTable,
        ReadableTable, TableHandle as _,
    },
    rkyv::rancor::Panic,
    std::{iter, path::Path},
    wordbase_api::{ArchivedRecord, Record},
};

// pub struct Lookups {
//     records: ReadOnlyTable<RecordId, &'static [u8]>,
//     headwords: ReadOnlyMultimapTable<&'static str, RecordId>,
//     readings: ReadOnlyMultimapTable<&'static str, RecordId>,
//     _txn: ReadTransaction,
//     _db: ReadOnlyDatabase,
// }

// impl Lookups {
//     pub fn new(db_path: &Path) -> Result<Self> {
//         let db = ReadOnlyDatabase::open(db_path).wrap_err("failed to open
// database")?;         let txn = db
//             .begin_read()
//             .wrap_err("failed to begin read transaction")?;

//         Ok(Self {
//             records: txn
//                 .open_table(RECORDS)
//                 .wrap_err_with(|| eyre!("failed to open table `{}`",
// RECORDS.name()))?,             headwords: txn
//                 .open_multimap_table(HEADWORDS)
//                 .wrap_err_with(|| eyre!("failed to open table `{}`",
// HEADWORDS.name()))?,             readings: txn
//                 .open_multimap_table(READINGS)
//                 .wrap_err_with(|| eyre!("failed to open table `{}`",
// READINGS.name()))?,             _txn: txn,
//             _db: db,
//         })
//     }

//     fn lookup_record_ids(&self, lemma: &str) -> Result<impl Iterator<Item =
// RecordId>> {

//     }

//     pub fn lookup_lemma_rkyv(&self, lemma: &str) -> Result<Lookup> {
//         let records = self
//             .lookup_record_ids(lemma)?
//             .filter_map(|id| self.records.get(id).ok().flatten())
//             .collect::<Vec<_>>();

//         Ok(Lookup { records })
//     }

//     pub fn lookup_lemma_rmp(&self, lemma: &str) -> Result<impl Iterator<Item
// = Result<Record>>> {         Ok(self
//             .lookup_record_ids(lemma)?
//             .filter_map(|id| match self.records.get(id) {
//                 Ok(Some(blob)) => Some(
//                     rmp_serde::from_slice::<Record>(blob.value())
//                         .wrap_err_with(|| eyre!("failed to deserialize
// {id:?}")),                 ),
//                 Ok(None) => None,
//                 Err(err) => Some(Err(err.into())),
//             }))
//     }
// }

// pub struct Lookup {
//     records: Vec<AccessGuard<'static, &'static [u8]>>,
// }

// impl Lookup {
//     pub fn unsorted(&self) -> impl Iterator<Item = RecordLookup<'_>> {
//         self.records.iter().map(|record| RecordLookup {
//             record: rkyv::access::<ArchivedRecord,
// Panic>(record.value()).unwrap(),         })
//     }
// }

// #[derive(Deref, DerefMut)]
// pub struct RecordLookup<'a> {
//     #[deref]
//     #[deref_mut]
//     record: &'a ArchivedRecord,
// }

// impl RecordLookup<'_> {
//     pub fn deserialize(&self) -> Record {
//         rkyv::deserialize::<_, Panic>(self.record).unwrap()
//     }
// }
