use {
    crate::{App, Result, extractor::ExtractProfile},
    axum::{Json, extract::State},
    serde::{Deserialize, Serialize},
    utoipa::ToSchema,
    utoipa_axum::{router::OpenApiRouter, routes},
    wordbase_types::{Deinflection, DictionaryId, Record, RecordId, Term},
};

pub fn routes() -> OpenApiRouter<App> {
    OpenApiRouter::new()
        .routes(routes!(lookup_deinflect))
        .routes(routes!(lookup_lemma))
        .routes(routes!(lookup_sentence))
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

/// Reads a sentence and generates deinflections for the word at the given
/// cursor position.
///
/// The generated deinflections can be later looked up to get dictionary results
/// for the word.
#[axum::debug_handler]
#[utoipa::path(
    post,
    path = "/lookup/deinflect",
    request_body = inline(Deinflect),
    responses((status = OK, body = Vec<Deinflection>)),
)]
async fn lookup_deinflect(
    State(app): State<App>,
    Json(req): Json<Deinflect>,
) -> Result<Json<Vec<Deinflection<'static>>>> {
    let deinflections = app.deinflectors.deinflect(&req.sentence, req.cursor)?;
    let deinflections = deinflections
        .into_iter()
        .map(Deinflection::into_owned)
        .collect::<Vec<Deinflection>>();
    Ok(Json(deinflections))
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[schema(examples(example_lookup_lemma))]
struct LookupLemma {
    lemma: String,
}

fn example_lookup_lemma() -> LookupLemma {
    LookupLemma {
        lemma: "読む".into(),
    }
}

/// Searches dictionaries for records which are used by a given lemma word.
///
/// This will not perform any deinflection or processing on the lemma; it will
/// look them up in the dictionary storages directly.
#[axum::debug_handler]
#[utoipa::path(
    post,
    path = "/lookup/lemma",
    params(ExtractProfile),
    request_body = inline(LookupLemma),
    responses((status = OK, body = Vec<RecordEntry>)),
)]
async fn lookup_lemma(
    State(app): State<App>,
    ExtractProfile(profile): ExtractProfile,
    Json(req): Json<LookupLemma>,
) -> Result<Json<Vec<RecordEntry>>> {
    let entries = app
        .storage
        .lookup_lemma(profile.id, &req.lemma)?
        .into_iter()
        .map(RecordEntry::from)
        .collect();
    Ok(Json(entries))
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[schema(examples(example_lookup_sentence))]
struct LookupSentence {
    sentence: String,
    cursor: usize,
}

fn example_lookup_sentence() -> LookupSentence {
    LookupSentence {
        sentence: "本を読んだ".into(),
        cursor: "本を".len(),
    }
}

/// Deinflects the word at a given position in a sentence, and searches
/// dictionaries for records which that lemma maps to.
///
/// This is a combination of `/lookup/deinflect` and `/lookup/lemma`, but more
/// efficient than calling them separately.
#[axum::debug_handler]
#[utoipa::path(
    post,
    path = "/lookup/sentence",
    params(ExtractProfile),
    responses((status = OK, body = Vec<RecordEntry>))
)]
async fn lookup_sentence(
    State(app): State<App>,
    ExtractProfile(profile): ExtractProfile,
    Json(req): Json<LookupSentence>,
) -> Result<Json<Vec<RecordEntry>>> {
    let entries = app
        .deinflectors
        .lookup(&app.storage, profile.id, &req.sentence, req.cursor)?
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
