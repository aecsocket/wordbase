use {
    crate::{Engine, Event},
    anyhow::{Context, Result, bail},
    derive_more::{Display, Error},
    futures::StreamExt,
    wordbase::{DictionaryId, Profile, ProfileId, ProfileMeta},
};

impl Engine {
    pub async fn current_profile(&self) -> Result<ProfileId> {
        let id = sqlx::query_scalar!("SELECT current_profile FROM config")
            .fetch_one(&self.db)
            .await?;
        Ok(ProfileId(id))
    }

    pub async fn profiles(&self) -> Result<Vec<Profile>> {
        let mut profiles = Vec::<Profile>::new();

        let mut records = sqlx::query!(
            "SELECT
                id,
                meta,
                sorting_dictionary,
                ped.dictionary
            FROM profile
            LEFT JOIN profile_enabled_dictionary ped ON profile.id = ped.profile
            ORDER BY profile.id"
        )
        .fetch(&self.db);
        while let Some(record) = records.next().await {
            let record = record.context("failed to fetch record")?;
            let id = ProfileId(record.id);

            let profile_index =
                if let Some(index) = profiles.iter_mut().position(|profile| profile.id == id) {
                    index
                } else {
                    let index = profiles.len();
                    let meta = serde_json::from_str::<ProfileMeta>(&record.meta)
                        .context("failed to deserialize profile meta")?;
                    profiles.push(Profile {
                        id,
                        meta,
                        enabled_dictionaries: Vec::new(),
                        sorting_dictionary: record.sorting_dictionary.map(DictionaryId),
                    });
                    index
                };

            if let Some(dictionary) = record.dictionary {
                profiles[profile_index]
                    .enabled_dictionaries
                    .push(DictionaryId(dictionary));
            }
        }

        Ok(profiles)
    }

    pub async fn insert_profile(&self, meta: ProfileMeta) -> Result<ProfileId> {
        let current_id = self
            .current_profile()
            .await
            .context("failed to fetch current profile id")?;
        let meta_json = serde_json::to_string(&meta).context("failed to serialize profile meta")?;

        let mut tx = self
            .db
            .begin()
            .await
            .context("failed to begin transaction")?;
        let new_id = sqlx::query!("INSERT INTO profile (meta) VALUES ($1)", meta_json)
            .execute(&mut *tx)
            .await
            .context("failed to insert profile")?
            .last_insert_rowid();
        let new_id = ProfileId(new_id);
        sqlx::query!(
            "INSERT INTO profile_enabled_dictionary (profile, dictionary)
            SELECT $1, dictionary
            FROM profile_enabled_dictionary
            WHERE profile = $2",
            new_id.0,
            current_id.0,
        )
        .execute(&mut *tx)
        .await
        .context("failed to copy enabled dictionaries")?;
        tx.commit().await.context("failed to commit transaction")?;

        // TODO: do we even want to send events?
        _ = self.send_event.send(Event::ProfileAdded {
            profile: Profile {
                id: new_id,
                meta,
                enabled_dictionaries: vec![], // TODO
                sorting_dictionary: None,     // TODO
            },
        });
        Ok(new_id)
    }

    pub async fn set_current_profile(&self, id: ProfileId) -> Result<()> {
        sqlx::query!("UPDATE config SET current_profile = $1", id.0)
            .execute(&self.db)
            .await?;
        Ok(())
    }

    pub async fn set_profile_sorting_dictionary(
        &self,
        profile_id: ProfileId,
        dictionary_id: Option<DictionaryId>,
    ) -> Result<()> {
        let dictionary_id = dictionary_id.map(|id| id.0);
        sqlx::query!(
            "UPDATE profile SET sorting_dictionary = $1 WHERE id = $2",
            dictionary_id,
            profile_id.0
        )
        .execute(&self.db)
        .await?;
        Ok(())
    }

    pub async fn remove_profile(&self, id: ProfileId) -> Result<()> {
        let result = sqlx::query!("DELETE FROM profile WHERE id = $1", id.0)
            .execute(&self.db)
            .await?;
        if result.rows_affected() == 0 {
            bail!(NotFound);
        }

        _ = self.send_event.send(Event::ProfileRemoved { id });
        Ok(())
    }
}

#[derive(Debug, Clone, Display, Error)]
#[display("profile not found")]
pub struct NotFound;
