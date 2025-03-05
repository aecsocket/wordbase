use derive_more::From;
use serde::{Deserialize, Serialize};
use wordbase::dict::{Frequency, PitchVariant};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Term {
    Definition(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, From)]
#[serde(tag = "type")]
pub enum TermMeta {
    Frequency(Frequency),
    Pitch {
        reading: String,
        variants: Vec<PitchVariant>,
    },
}
