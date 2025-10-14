use {
    crate::{App, AppError, Result},
    axum::{Json, extract::State},
    serde::{Deserialize, Serialize},
    utoipa::ToSchema,
    utoipa_axum::{router::UtoipaMethodRouter, routes},
    wordbase_types::{Deinflection, DictionaryId, ProfileId, Record, RecordId, Term},
};

pub fn routes() -> UtoipaMethodRouter<App> {
    routes!(lookup_deinflect, lookup_lemma, lookup_sentence)
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[schema(examples(example_deinflect))]
struct Deinflect {
    sentence: String,
    cursor: usize,
}

fn example_deinflect() -> Deinflect {
    Deinflect {
        sentence: "読まなかった".into(),
        cursor: 0,
    }
}

#[axum::debug_handler]
#[utoipa::path(post, path = "/lookup/deinflect", responses((status = OK, body = Vec<Deinflection>)))]
async fn lookup_deinflect(
    State(app): State<App>,
    Json(req): Json<Deinflect>,
) -> Result<Json<Vec<Deinflection<'static>>>> {
    let deinflections = app
        .deinflectors
        .deinflect(&req.sentence, req.cursor)
        .map_err(AppError::Internal)?;
    let deinflections = deinflections
        .into_iter()
        .map(Deinflection::into_owned)
        .collect::<Vec<Deinflection>>();
    Ok(Json(deinflections))
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[schema(examples(example_lookup_lemma))]
struct LookupLemma {
    profile_id: ProfileId,
    lemma: String,
}

fn example_lookup_lemma() -> LookupLemma {
    LookupLemma {
        profile_id: ProfileId(uuid::uuid!("f619b6a1-28cb-4e32-8294-85b7f51f76c5")),
        lemma: "読む".into(),
    }
}

#[axum::debug_handler]
#[utoipa::path(post, path = "/lookup/lemma", responses((status = OK, body = Vec<RecordEntry>)))]
async fn lookup_lemma(
    State(app): State<App>,
    Json(req): Json<LookupLemma>,
) -> Result<Json<Vec<RecordEntry>>> {
    let entries = app
        .storage
        .lookup_lemma(req.profile_id, &req.lemma)
        .map_err(AppError::Internal)?
        .into_iter()
        .map(RecordEntry::from)
        .collect();
    Ok(Json(entries))
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[schema(examples(example_lookup_sentence))]
struct LookupSentence {
    profile_id: ProfileId,
    sentence: String,
    cursor: usize,
}

fn example_lookup_sentence() -> LookupSentence {
    LookupSentence {
        profile_id: ProfileId(uuid::uuid!("f619b6a1-28cb-4e32-8294-85b7f51f76c5")),
        sentence: "本を読んだ".into(),
        cursor: "本を".len(),
    }
}

#[axum::debug_handler]
#[utoipa::path(post, path = "/lookup/sentence", responses((status = OK, body = Vec<RecordEntry>)))]
async fn lookup_sentence(
    State(app): State<App>,
    Json(req): Json<LookupSentence>,
) -> Result<Json<Vec<RecordEntry>>> {
    let entries = app
        .deinflectors
        .lookup(&app.storage, req.profile_id, &req.sentence, req.cursor)
        .map_err(AppError::Internal)?
        .into_iter()
        .map(RecordEntry::from)
        .collect();
    Ok(Json(entries))
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
struct RecordEntry {
    dictionary_id: DictionaryId,
    term: Term,
    record_id: RecordId,
    record: Record,
}

impl From<wordbase_engine::dictionaries::RecordEntry> for RecordEntry {
    fn from(value: wordbase_engine::dictionaries::RecordEntry) -> Self {
        Self {
            dictionary_id: value.dictionary.id,
            term: value.term,
            record_id: value.record_id,
            record: value.record,
        }
    }
}
