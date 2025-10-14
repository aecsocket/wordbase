pub use wordbase_core::storage::*;
use {
    crate::{
        dictionaries::{Dictionaries, RecordEntry},
        profiles::{self, Profiles},
    },
    arc_swap::ArcSwap,
    eyre::{Context, Result, eyre},
    sqlx::{
        Pool, Sqlite,
        sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
    },
    std::path::Path,
    tokio::fs,
    wordbase_types::{DictionaryId, ProfileId},
};

#[derive(Debug)]
pub struct EngineStorage {
    pub(crate) dictionaries: Dictionaries,
    pub(crate) profiles: ArcSwap<Profiles>,
    pub(crate) db: Pool<Sqlite>,
}

impl EngineStorage {
    pub async fn new(data_dir: impl AsRef<Path>) -> Result<Self> {
        Self::new_(data_dir.as_ref()).await
    }

    async fn new_(data_dir: &Path) -> Result<Self> {
        fs::create_dir_all(data_dir)
            .await
            .wrap_err_with(|| eyre!("failed to create data directory at {data_dir:?}"))?;
        let db = setup_db(&data_dir.join("wordbase.db")).await?;

        let num_profiles = sqlx::query_scalar!("SELECT COUNT(*) FROM profile")
            .fetch_one(&db)
            .await
            .wrap_err("failed to count rows in profile table")?;
        if num_profiles == 0 {
            profiles::create(&db, "")
                .await
                .wrap_err("failed to create default profile")?;
        }

        let dicts_dir = data_dir.join("dictionaries");
        fs::create_dir_all(&dicts_dir)
            .await
            .wrap_err_with(|| eyre!("failed to create dictionaries directory at {dicts_dir:?}"))?;
        let dictionaries = Dictionaries::new(dicts_dir)
            .await
            .wrap_err("failed to setup dictionaries")?;

        let profiles = profiles::fetch(&db)
            .await
            .wrap_err("failed to fetch initial profiles")?;

        Ok(Self {
            dictionaries,
            profiles: ArcSwap::from_pointee(profiles),
            db,
        })
    }

    pub async fn remove_dictionary(&self, dict_id: DictionaryId) -> Result<()> {
        profiles::remove_dictionary(&self.db, dict_id).await?;
        self.dictionaries.remove(dict_id).await?;
        Ok(())
    }

    pub fn lookup_lemma(&self, profile_id: ProfileId, lemma: &str) -> Result<Vec<RecordEntry>> {
        let profiles = self.profiles.load();
        let profile = profiles
            .get(&profile_id)
            .ok_or_else(|| eyre!("invalid profile {profile_id:?}"))?;
        self.dictionaries
            .lookup(lemma, &profile.enabled_dictionaries)
    }
}

async fn setup_db(db_path: &Path) -> Result<Pool<Sqlite>> {
    let options = SqliteConnectOptions::new()
        .filename(db_path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal);

    let db = SqlitePoolOptions::new()
        .max_connections(8)
        .connect_with(options)
        .await
        .context("failed to connect to database")?;

    sqlx::migrate!()
        .run(&db)
        .await
        .context("failed to run migrations")?;

    Ok(db)
}
