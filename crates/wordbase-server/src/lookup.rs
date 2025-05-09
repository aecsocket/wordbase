use {
    crate::Term,
    poem::Result,
    poem_openapi::{
        Object, Union,
        types::{Any, Example},
    },
    serde::{Deserialize, Serialize},
    wordbase::{DictionaryId, ProfileId, Record, RecordId, RecordKind},
    wordbase_engine::Engine,
};

pub async fn expr(engine: &Engine, req: ExprRequest) -> Result<Vec<RecordLookup>> {
    Ok(engine
        .lookup(req.profile_id, &req.sentence, req.cursor, &req.record_kinds)
        .await?
        .into_iter()
        .map(RecordLookup::from)
        .collect())
}

#[derive(Debug, Clone, Serialize, Deserialize, Object)]
#[oai(example)]
pub struct ExprRequest {
    profile_id: ProfileId,
    sentence: String,
    cursor: usize,
    record_kinds: Vec<RecordKind>,
}

impl Example for ExprRequest {
    fn example() -> Self {
        Self {
            profile_id: ProfileId(1),
            sentence: "本を読んだ".into(),
            cursor: "本を".len(),
            record_kinds: vec![RecordKind::YomitanGlossary],
        }
    }
}

pub async fn lemma(engine: &Engine, req: Lemma) -> Result<Vec<RecordLookup>> {
    Ok(engine
        .lookup_lemma(req.profile_id, &req.lemma, &req.record_kinds)
        .await?
        .into_iter()
        .map(RecordLookup::from)
        .collect())
}

#[derive(Debug, Clone, Serialize, Deserialize, Object)]
#[oai(example)]
pub struct Lemma {
    profile_id: ProfileId,
    lemma: String,
    record_kinds: Vec<RecordKind>,
}

impl Example for Lemma {
    fn example() -> Self {
        Self {
            profile_id: ProfileId(1),
            lemma: "読む".into(),
            record_kinds: vec![RecordKind::YomitanGlossary],
        }
    }
}

pub async fn deinflect(engine: &Engine, req: Deinflect) -> Vec<Deinflection> {
    engine
        .deinflect(&req.text)
        .into_iter()
        .map(Deinflection::from)
        .collect()
}

#[derive(Debug, Clone, Object)]
#[oai(example)]
pub struct Deinflect {
    text: String,
}

impl Example for Deinflect {
    fn example() -> Self {
        Self {
            text: "読まなかった".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Object)]
pub struct Deinflection {
    lemma: String,
    scan_len: usize,
}

impl From<wordbase_engine::deinflect::Deinflection<'_>> for Deinflection {
    fn from(value: wordbase_engine::deinflect::Deinflection) -> Self {
        Self {
            lemma: value.lemma.into_owned(),
            scan_len: value.scan_len,
        }
    }
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
            profile_sorting_frequency: value.profile_sorting_frequency.map(FrequencyValue::from),
            source_sorting_frequency: value.source_sorting_frequency.map(FrequencyValue::from),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Union)]
#[oai(discriminator_name = "kind")]
pub enum FrequencyValue {
    Rank(FrequencyRank),
    Occurrence(FrequencyOccurrence),
}

#[derive(Debug, Clone, Serialize, Deserialize, Object)]
pub struct FrequencyRank {
    pub rank: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Object)]
pub struct FrequencyOccurrence {
    pub occurrence: i64,
}

impl From<wordbase::FrequencyValue> for FrequencyValue {
    fn from(value: wordbase::FrequencyValue) -> Self {
        match value {
            wordbase::FrequencyValue::Rank(rank) => Self::Rank(FrequencyRank { rank }),
            wordbase::FrequencyValue::Occurrence(occurrence) => {
                Self::Occurrence(FrequencyOccurrence { occurrence })
            }
        }
    }
}
