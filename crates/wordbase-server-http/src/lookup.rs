use {
    crate::App,
    poem::Result,
    poem_openapi::{
        Object, Union,
        types::{Any, Example},
    },
    std::ops::Range,
    wordbase_types::{DictionaryId, ProfileId, Record, RecordId, Span},
};

#[derive(Debug, Clone, Object)]
#[oai(example)]
pub struct Sentence {
    profile_id: ProfileId,
    sentence: String,
    cursor: usize,
}

pub async fn sentence(app: &App, req: Sentence) -> Result<Vec<RecordEntry>> {
    Ok(app
        .deinflectors
        .lookup(&app.storage, req.profile_id, &req.sentence, req.cursor)?
        .into_iter()
        .map(RecordEntry::from)
        .collect())
}

impl Example for Sentence {
    fn example() -> Self {
        Self {
            profile_id: ProfileId(1),
            sentence: "本を読んだ".into(),
            cursor: "本を".len(),
        }
    }
}

pub async fn lemma(app: &App, req: Lemma) -> Result<Vec<RecordEntry>> {
    Ok(app
        .lookups
        .lookup_lemma(&app.engine, req.profile_id, &req.lemma)
        .await?
        .into_iter()
        .map(RecordEntry::from)
        .collect())
}

#[derive(Debug, Clone, Object)]
#[oai(example)]
pub struct Lemma {
    profile_id: ProfileId,
    lemma: String,
}

impl Example for Lemma {
    fn example() -> Self {
        Self {
            profile_id: ProfileId(1),
            lemma: "読む".into(),
        }
    }
}

pub async fn deinflect(app: &App, req: Deinflect) -> Vec<Deinflection> {
    app.lookups
        .deinflect(&req.text, req.cursor)
        .into_iter()
        .map(Deinflection::from)
        .collect()
}

#[derive(Debug, Clone, Object)]
#[oai(example)]
pub struct Deinflect {
    text: String,
    cursor: usize,
}

impl Example for Deinflect {
    fn example() -> Self {
        Self {
            text: "読まなかった".into(),
            cursor: 0,
        }
    }
}

#[derive(Debug, Clone, Object)]
pub struct Deinflection {
    span: SpanUsize,
    lemma: String,
}

// TODO: unify this type?

#[derive(Debug, Clone, Object)]
pub struct SpanUsize {
    pub start: usize,
    pub end: usize,
}

impl From<Range<usize>> for SpanUsize {
    fn from(value: Range<usize>) -> Self {
        Self {
            start: value.start,
            end: value.end,
        }
    }
}

impl From<wordbase::deinflect::Deinflection<'_>> for Deinflection {
    fn from(value: wordbase::deinflect::Deinflection<'_>) -> Self {
        Self {
            span: value.span.into(),
            lemma: value.lemma.into_owned(),
        }
    }
}

#[derive(Debug, Clone, Object)]
pub struct RecordEntry {
    pub span_bytes: Span,
    pub span_chars: Span,
    pub source: DictionaryId,
    pub term: Term,
    pub record_id: RecordId,
    pub record: Any<Record>,
    pub profile_sorting_frequency: Option<FrequencyValue>,
    pub source_sorting_frequency: Option<FrequencyValue>,
}

impl From<wordbase::RecordEntry> for RecordEntry {
    fn from(value: wordbase::RecordEntry) -> Self {
        Self {
            span_bytes: value.span_bytes,
            span_chars: value.span_chars,
            source: value.source,
            term: value.term.into(),
            record_id: value.record_id,
            record: Any(value.record),
            profile_sorting_frequency: value.profile_sorting_frequency.map(FrequencyValue::from),
            source_sorting_frequency: value.source_sorting_frequency.map(FrequencyValue::from),
        }
    }
}

#[derive(Debug, Clone, Union)]
#[oai(discriminator_name = "kind")]
pub enum FrequencyValue {
    Rank(FrequencyRank),
    Occurrence(FrequencyOccurrence),
}

#[derive(Debug, Clone, Object)]
pub struct FrequencyRank {
    pub rank: i64,
}

#[derive(Debug, Clone, Object)]
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
