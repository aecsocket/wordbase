use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Dictionary {
    pub id: DictionaryId,
    pub title: String,
    pub revision: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DictionaryId(pub i64);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Frequency {
    pub value: u64,
    pub display_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pitch {
    pub position: u64,
    pub nasal: Vec<u64>,
    pub devoice: Vec<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Glossary {
    Definition { text: String },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LookupInfo {
    pub lemma: String,
    pub terms: Vec<ExpressionInfo>,
    pub frequencies: Vec<(ExpressionInfo, Frequency)>,
    pub pitches: Vec<(ExpressionInfo, Pitch)>,
    pub glossaries: Vec<(ExpressionInfo, Glossary)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpressionInfo {
    pub source: DictionaryId,
    pub expression: String,
    pub reading: String,
}
