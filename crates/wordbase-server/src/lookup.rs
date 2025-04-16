use poem_openapi::{ApiResponse, Object, payload::Json, types::Any};
use serde::{Deserialize, Serialize};
use wordbase::{DictionaryId, FrequencyValue, ProfileId, Record, RecordId, RecordKind};
use wordbase_engine::Engine;

pub async fn deinflect(engine: &Engine, req: Json<DeinflectRequest>) -> DeinflectResponse {
    DeinflectResponse::Ok(Json(
        engine
            .deinflect(&req.text)
            .into_iter()
            .map(|v| Deinflection {
                lemma: v.lemma.into_owned(),
                scan_len: v.scan_len,
            })
            .collect(),
    ))
}

#[derive(Debug, Clone, Object)]
pub struct DeinflectRequest {
    text: String,
}

#[derive(Debug, Clone, ApiResponse)]
pub enum DeinflectResponse {
    #[oai(status = 200)]
    Ok(Json<Vec<Deinflection>>),
}

#[derive(Debug, Clone, Serialize, Deserialize, Object)]
pub struct Deinflection {
    lemma: String,
    scan_len: usize,
}

pub async fn lemma(engine: &Engine, req: Json<LemmaRequest>) -> RecordsResponse {
    RecordsResponse::Ok(Json(
        engine
            .lookup_lemma(req.profile_id, &req.lemma, &req.record_kinds)
            .await
            .unwrap()
            .into_iter()
            .map(RecordLookup::from)
            .collect(),
    ))
}

#[derive(Debug, Clone, Serialize, Deserialize, Object)]
pub struct LemmaRequest {
    profile_id: ProfileId,
    lemma: String,
    record_kinds: Vec<RecordKind>,
}

#[derive(Debug, Clone, ApiResponse)]
pub enum RecordsResponse {
    #[oai(status = 200)]
    Ok(Json<Vec<RecordLookup>>),
}

#[derive(Debug, Clone, Serialize, Deserialize, Object)]
pub struct RecordLookup {
    pub bytes_scanned: usize,
    pub source: DictionaryId,
    pub term: Term,
    pub record_id: RecordId,
    pub record: Any<Record>,
    pub profile_sorting_frequency: Option<FrequencyValue>,
    pub source_sorting_frequency: Option<FrequencyValue>,
}

impl From<wordbase::RecordLookup> for RecordLookup {
    fn from(value: wordbase::RecordLookup) -> Self {
        Self {
            bytes_scanned: value.bytes_scanned,
            source: value.source,
            term: value.term.into(),
            record_id: value.record_id,
            record: Any(value.record),
            profile_sorting_frequency: value.profile_sorting_frequency,
            source_sorting_frequency: value.source_sorting_frequency,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Object)]
pub struct Term {
    pub headword: Option<String>,
    pub reading: Option<String>,
}

impl From<wordbase::Term> for Term {
    fn from(value: wordbase::Term) -> Self {
        Self {
            headword: value.headword().map(ToString::to_string),
            reading: value.reading().map(ToString::to_string),
        }
    }
}

pub async fn expr(engine: &Engine, req: Json<ExprRequest>) -> RecordsResponse {
    RecordsResponse::Ok(Json(
        engine
            .lookup(req.profile_id, &req.sentence, req.cursor, &req.record_kinds)
            .await
            .unwrap()
            .into_iter()
            .map(RecordLookup::from)
            .collect(),
    ))
}

#[derive(Debug, Clone, Serialize, Deserialize, Object)]
pub struct ExprRequest {
    profile_id: ProfileId,
    sentence: String,
    cursor: usize,
    record_kinds: Vec<RecordKind>,
}
