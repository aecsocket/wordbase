use anyhow::Context;
use poem::Result;
use poem_openapi::{Object, types::Example};
use serde::{Deserialize, Serialize};
use wordbase::{NormString, ProfileId};
use wordbase_engine::Engine;

use crate::Term;

pub async fn note_add(engine: &Engine, req: NoteAdd) -> Result<()> {
    engine
        .add_anki_note(
            req.profile_id,
            &req.sentence,
            req.cursor,
            &req.term.try_into().context("invalid term")?,
            req.sentence_audio.as_deref(),
            req.sentence_image.as_deref(),
        )
        .await?;
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, Object)]
#[oai(example)]
pub struct NoteAdd {
    pub profile_id: ProfileId,
    pub sentence: String,
    pub cursor: usize,
    pub term: Term,
    pub sentence_audio: Option<String>,
    pub sentence_image: Option<String>,
}

impl Example for NoteAdd {
    fn example() -> Self {
        Self {
            profile_id: ProfileId(1),
            sentence: "本を読む".into(),
            cursor: "本を".len(),
            term: Term {
                headword: NormString::new("読む"),
                reading: NormString::new("よむ"),
            },
            sentence_audio: None,
            sentence_image: None,
        }
    }
}
