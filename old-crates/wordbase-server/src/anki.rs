use {
    crate::Term,
    anyhow::Context,
    poem::Result,
    poem_openapi::{Object, types::Example},
    serde::{Deserialize, Serialize},
    wordbase::{NormString, ProfileId},
    wordbase_engine::Engine,
};

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

pub async fn connect(engine: &Engine, req: Connect) -> Result<()> {
    engine.anki_connect(req.server_url, req.api_key).await?;
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, Object)]
#[oai(example)]
pub struct Connect {
    pub server_url: String,
    pub api_key: String,
}

impl Example for Connect {
    fn example() -> Self {
        Self {
            server_url: "http://127.0.0.1:8765".into(),
            api_key: String::new(),
        }
    }
}
