use anyhow::{Context, Result};
use sqlx::{Executor, Sqlite};
use wordbase::ProfileId;

use crate::Config;

pub async fn get<'e, 'c: 'e, E>(executor: E) -> Result<Config>
where
    E: 'e + Executor<'c, Database = Sqlite>,
{
    let record = sqlx::query!(
        "SELECT current_profile, data
        FROM config"
    )
    .fetch_one(executor)
    .await
    .context("failed to fetch config")?;
    let config =
        serde_json::from_str::<Config>(&record.data).context("failed to deserialize config")?;

    Ok(Config {
        current_profile: ProfileId(record.current_profile),
        ..config
    })
}

pub async fn set<'e, 'c: 'e, E>(executor: E, config: &Config) -> Result<()>
where
    E: 'e + Executor<'c, Database = Sqlite>,
{
    let current_profile = config.current_profile;
    let data = serde_json::to_string(config).context("failed to serialize config")?;

    sqlx::query!(
        "UPDATE config
        SET current_profile = $1, data = $2",
        current_profile.0,
        data
    )
    .execute(executor)
    .await?;
    Ok(())
}
