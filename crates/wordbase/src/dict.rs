use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpressionEntry {
    pub reading: Reading,
    pub frequency_sets: Vec<FrequencySet>,
    pub pitch_sets: Vec<PitchSet>,
    pub glossary_sets: Vec<GlossarySet>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reading {
    expression: String,
    reading: String,
    pair_indices: Vec<(usize, usize)>,
}

impl Reading {
    #[must_use]
    pub fn from_no_pairs(expression: impl Into<String>, reading: impl Into<String>) -> Self {
        Self {
            expression: expression.into(),
            reading: reading.into(),
            pair_indices: Vec::new(),
        }
    }

    #[must_use]
    pub fn from_pairs<E: AsRef<str>, R: AsRef<str>>(
        pairs: impl IntoIterator<Item = (E, R)>,
    ) -> Self {
        let mut expression = String::new();
        let mut reading = String::new();
        let mut pair_indices = Vec::new();
        for (expression_part, reading_part) in pairs {
            let expression_part = expression_part.as_ref();
            let reading_part = reading_part.as_ref();
            expression.push_str(expression_part);
            reading.push_str(reading_part);
            pair_indices.push((expression_part.len(), reading_part.len()));
        }
        Self {
            expression,
            reading,
            pair_indices,
        }
    }

    #[must_use]
    pub fn expression(&self) -> &str {
        &self.expression
    }

    #[must_use]
    pub fn reading(&self) -> &str {
        &self.reading
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrequencySet {
    pub dictionary: String,
    pub frequencies: Vec<Frequency>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Frequency {
    pub value: u64,
    pub display_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PitchSet {
    pub dictionary: String,
    pub pitches: Vec<Pitch>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pitch {
    pub position: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlossarySet {
    pub dictionary: String,
    pub glossaries: Vec<Glossary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Glossary {
    pub todo: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        assert_eq!(
            Reading::from_pairs([("協", "きょう"), ("力", "りょく")]),
            Reading {
                expression: "協力".into(),
                reading: "きょうりょく".into(),
                pair_indices: vec![("協".len(), "きょう".len()), ("力".len(), "りょく".len()),],
            }
        );
    }
}
