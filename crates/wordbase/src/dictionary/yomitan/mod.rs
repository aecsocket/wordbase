pub mod structured;

#[cfg(feature = "parse-yomitan")]
mod parse;
#[cfg(feature = "parse-yomitan")]
pub use parse::*;

use alloc::{string::String, vec::Vec};
use derive_more::{Deref, DerefMut};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

// https://github.com/yomidevs/yomitan/blob/master/ext/data/schemas/dictionary-index-schema.json
// https://github.com/yomidevs/yomitan/blob/3ca2800d4aeff0a93be23642db9892ddbae1aa55/types/ext/dictionary-data.d.ts#L22
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Index {
    #[serde(alias = "version")]
    pub format: Format,
    pub title: String,
    pub revision: String,
    pub minimum_yomitan_version: Option<String>,
    pub sequenced: Option<bool>,
    pub is_updatable: Option<bool>,
    pub index_url: Option<String>,
    pub download_url: Option<String>,
    pub author: Option<String>,
    pub url: Option<String>,
    pub description: Option<String>,
    pub attribution: Option<String>,
    pub source_language: Option<IsoLanguageCode>,
    pub target_language: Option<IsoLanguageCode>,
    pub frequency_mode: Option<FrequencyMode>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum Format {
    One = 1,
    Two = 2,
    Three = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FrequencyMode {
    OccurrenceBased,
    RankBased,
}

#[derive(Debug, Clone, Serialize, Deserialize, Deref, DerefMut)]
pub struct IsoLanguageCode(pub String);

#[derive(Debug, Clone, Default, Serialize, Deserialize, Deref, DerefMut)]
pub struct TagBank(pub Vec<Tag>);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tag {
    pub name: String,
    pub category: String,
    pub order: i64,
    pub notes: String,
    pub score: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Deref, DerefMut)]
pub struct TermBank(pub Vec<Term>);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Term {
    pub expression: String,
    pub reading: String,
    pub definition_tags: Option<String>,
    pub rules: String,
    pub score: i64,
    pub glossary: Vec<Glossary>,
    pub sequence: i64,
    pub term_tags: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged, deny_unknown_fields)]
pub enum Glossary {
    String(String),
    Deinflection(GlossaryDeinflection),
    Content(GlossaryContent),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GlossaryDeinflection {
    pub uninflected: String,
    pub inflection_rule_chain: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "type", deny_unknown_fields)]
pub enum GlossaryContent {
    Text { text: String },
    Image(structured::ImageElementBase),
    StructuredContent { content: structured::Content },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Deref, DerefMut)]
pub struct TermMetaBank(pub Vec<TermMeta>);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TermMeta {
    Frequency(TermMetaFrequency),
    Pitch(TermMetaPitch),
    Phonetic(TermMetaPhonetic),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TermMetaFrequency {
    pub expression: String,
    // pub data:
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", untagged, deny_unknown_fields)]
pub enum GenericFrequencyData {
    String(String),
    Number(u64),
    Complex {
        value: u64,
        display_value: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TermMetaPitch {
    pub expression: String,
    pub data: TermMetaPitchData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TermMetaPitchData {
    pub reading: String,
    pub pitches: Vec<Pitch>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Pitch {
    pub position: u64,
    pub nasal: Option<PitchPosition>,
    pub devoice: Option<PitchPosition>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PitchPosition {
    One(u64),
    Many(Vec<u64>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TermMetaPhonetic {
    pub expression: String,
    pub data: TermMetaPhoneticData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TermMetaPhoneticData {
    pub reading: String,
    pub transcriptions: Vec<PhoneticTranscription>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PhoneticTranscription {
    pub ipa: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn case_1() {
        serde_json::from_str::<TermBank>(
            r##"
[
    [
        "マルクス経済学",
        "マルクスけいざいがく",
        "",
        "",
        0,
        [
            {
                "type": "structured-content",
                "content": [
                    {
                        "tag": "div",
                        "content": [
                            {
                                "tag": "span",
                                "title": "noun (common) (futsuumeishi)",
                                "style": {
                                    "fontSize": "0.8em",
                                    "fontWeight": "bold",
                                    "padding": "0.2em 0.3em",
                                    "wordBreak": "keep-all",
                                    "borderRadius": "0.3em",
                                    "verticalAlign": "text-bottom",
                                    "backgroundColor": "#565656",
                                    "color": "white",
                                    "cursor": "help",
                                    "marginRight": "0.25em"
                                },
                                "data": {
                                    "code": "n"
                                },
                                "content": "noun"
                            },
                            {
                                "tag": "div",
                                "content": {
                                    "tag": "ul",
                                    "style": {
                                        "listStyleType": "none",
                                        "paddingLeft": "0"
                                    },
                                    "data": {
                                        "content": "glossary"
                                    },
                                    "content": {
                                        "tag": "li",
                                        "content": "Marxian economics"
                                    }
                                }
                            }
                        ]
                    },
                    {
                        "tag": "div",
                        "style": {
                            "fontSize": "0.7em",
                            "textAlign": "right"
                        },
                        "data": {
                            "content": "attribution"
                        },
                        "content": {
                            "tag": "a",
                            "href": "https://www.edrdg.org/jmwsgi/entr.py?svc=jmdict&q=1968780",
                            "content": "JMdict"
                        }
                    }
                ]
            }
        ],
        1968780,
        ""
    ]
]
            "##,
        )
        .unwrap();
    }
}
