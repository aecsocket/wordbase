//! General dictionary schema.
//!
//! See [`dictionary-data.d.ts`](https://github.com/yomidevs/yomitan/blob/master/types/ext/dictionary-data.d.ts).
#![expect(missing_docs, reason = "these are not our types")]

use {
    super::structured,
    derive_more::{Deref, DerefMut, From},
    foldhash::HashMap,
    serde::Deserialize,
    serde_repr::Deserialize_repr,
};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize_repr)]
#[repr(u8)]
pub enum Format {
    One = 1,
    Two = 2,
    Three = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FrequencyMode {
    OccurrenceBased,
    RankBased,
}

#[derive(Debug, Clone, Deserialize, Deref, DerefMut)]
pub struct IsoLanguageCode(pub String);

#[derive(Debug, Clone, Default, Deserialize, Deref, DerefMut)]
pub struct TagBank(pub Vec<Tag>);

impl IntoIterator for TagBank {
    type IntoIter = <Vec<Tag> as IntoIterator>::IntoIter;
    type Item = Tag;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tag {
    pub name: String,
    pub category: String,
    pub order: i64,
    pub notes: String,
    pub score: i64,
}

#[derive(Debug, Clone, Default, Deserialize, Deref, DerefMut)]
pub struct TermBank(pub Vec<Term>);

impl IntoIterator for TermBank {
    type IntoIter = <Vec<Term> as IntoIterator>::IntoIter;
    type Item = Term;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

#[derive(Debug, Clone, Deserialize)]
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

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged, deny_unknown_fields)]
pub enum Glossary {
    String(String),
    Deinflection(GlossaryDeinflection),
    Content(GlossaryContent),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GlossaryDeinflection {
    pub uninflected: String,
    pub inflection_rule_chain: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "type", deny_unknown_fields)]
pub enum GlossaryContent {
    Text { text: String },
    Image(structured::ImageElementBase),
    StructuredContent { content: structured::Content },
}

#[derive(Debug, Clone, Default, Deserialize, Deref, DerefMut)]
pub struct TermMetaBank(pub Vec<TermMeta>);

impl IntoIterator for TermMetaBank {
    type IntoIter = <Vec<TermMeta> as IntoIterator>::IntoIter;
    type Item = TermMeta;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct TermMetaRaw {
    expression: String,
    kind: TermMetaKind,
    data: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(try_from = "TermMetaRaw")]
pub struct TermMeta {
    pub expression: String,
    pub data: TermMetaData,
}

#[derive(Debug, Clone, Deserialize, From)]
pub enum TermMetaData {
    Frequency(TermMetaFrequency),
    Pitch(TermMetaPitch),
    Phonetic(TermMetaPhonetic),
}

impl TryFrom<TermMetaRaw> for TermMeta {
    type Error = serde_json::Error;

    fn try_from(
        TermMetaRaw {
            expression,
            kind,
            data,
        }: TermMetaRaw,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            expression,
            data: match kind {
                TermMetaKind::Frequency => TermMetaFrequency::deserialize(data)?.into(),
                TermMetaKind::Pitch => TermMetaPitch::deserialize(data)?.into(),
                TermMetaKind::Phonetic => TermMetaPhonetic::deserialize(data)?.into(),
            },
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub enum TermMetaKind {
    #[serde(rename = "freq")]
    Frequency,
    #[serde(rename = "pitch")]
    Pitch,
    #[serde(rename = "ipa")]
    Phonetic,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", untagged, deny_unknown_fields)]
pub enum TermMetaFrequency {
    Generic(GenericFrequencyData),
    WithReading {
        reading: String,
        frequency: GenericFrequencyData,
    },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged, deny_unknown_fields)]
pub enum GenericFrequencyData {
    String(String),
    Number(u64),
    #[serde(rename_all = "camelCase")]
    Complex {
        value: u64,
        display_value: Option<String>,
    },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TermMetaPitch {
    pub reading: String,
    pub pitches: Vec<Pitch>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Pitch {
    pub position: u64,
    pub nasal: Option<PitchPosition>,
    pub devoice: Option<PitchPosition>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum PitchPosition {
    One(u64),
    Many(Vec<u64>),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TermMetaPhonetic {
    pub reading: String,
    pub transcriptions: Vec<PhoneticTranscription>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PhoneticTranscription {
    pub ipa: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Deref, DerefMut)]
pub struct KanjiBank(pub Vec<Kanji>);

impl IntoIterator for KanjiBank {
    type IntoIter = <Vec<Kanji> as IntoIterator>::IntoIter;
    type Item = Kanji;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Kanji {
    pub character: String,
    pub onyomi: String,
    pub kunyomi: String,
    pub tags: String,
    pub meanings: Vec<String>,
    pub stats: HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Deserialize, Deref, DerefMut)]
pub struct KanjiMetaBank(pub Vec<KanjiMetaFrequency>);

impl IntoIterator for KanjiMetaBank {
    type IntoIter = <Vec<KanjiMetaFrequency> as IntoIterator>::IntoIter;
    type Item = KanjiMetaFrequency;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct KanjiMetaFrequency {
    pub character: String,
    pub mode: String,
    pub data: GenericFrequencyData,
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
]"##,
        )
        .unwrap();
    }

    #[test]
    fn case_2() {
        serde_json::from_str::<TermBank>(
r##"
[
    [
        "される",
        "",
        null,
        "v1",
        0,
        [
            {
                "type": "structured-content",
                "content": [
                    "さ・れる\n〘動下一〙\n",
                    {
                        "tag": "span",
                        "content": {
                            "tag": "img",
                            "appearance": "monochrome",
                            "height": 1.0,
                            "width": 1.0,
                            "sizeUnits": "em",
                            "collapsible": false,
                            "collapsed": false,
                            "background": false,
                            "path": "meikyo2/B172.png"
                        },
                        "style": {
                            "verticalAlign": "text-bottom",
                            "marginRight": 0.25
                        }
                    },
                    " 〘他〙\n❶ 他動詞「する」の尊敬語。なさる。\n「先生が講演を━」\n「これからどう━おつもりですか」\n❷ 他動詞「する」の受身の形。\n「仲間に意地悪を━」\n",
                    {
                        "tag": "span",
                        "content": {
                            "tag": "img",
                            "appearance": "monochrome",
                            "height": 1.0,
                            "width": 1.0,
                            "sizeUnits": "em",
                            "collapsible": false,
                            "collapsed": false,
                            "background": false,
                            "path": "meikyo2/B173.png"
                        },
                        "style": {
                            "verticalAlign": "text-bottom",
                            "marginRight": 0.25
                        }
                    },
                    " 〘自他〙\n❶ サ変動詞「…する」の尊敬語。なさる。\n「さぞかしびっくり━・れたことでしょう」\n「先生は物理学を研究━・れています」\n❷ サ変動詞「…する」の受身の形。\n「山田氏は誰からも尊敬━・れている」\n「田中氏が学位を授与━・れる」\n",
                    {
                        "tag": "span",
                        "content": {
                            "tag": "img",
                            "appearance": "monochrome",
                            "height": 1.0,
                            "width": 1.0,
                            "sizeUnits": "em",
                            "collapsible": false,
                            "collapsed": false,
                            "background": false,
                            "path": "meikyo2/B174.png"
                        },
                        "style": {
                            "verticalAlign": "text-bottom",
                            "marginRight": 0.25
                        }
                    },
                    " 〘自〙《「…と━」の形で》ある意見や考えが、個人的なものではなく、一般的なものだとして提示する。一般に…と言われ（てい）る、…と考えられている。\n「一般に成功は難しいと━」\n「鶴は千年亀は万年生きると━」\n◆「する」の未然形＋助動詞「れる」から。\n",
                    {
                        "tag": "span",
                        "content": {
                            "tag": "img",
                            "title": "注意",
                            "appearance": "auto",
                            "height": 1.0,
                            "width": 3.0,
                            "sizeUnits": "em",
                            "collapsible": false,
                            "collapsed": false,
                            "background": false,
                            "path": "meikyo2/B932.png"
                        },
                        "style": {
                            "verticalAlign": "text-bottom",
                            "marginRight": 0.25
                        }
                    },
                    "謙譲の「お［ご］…する」に尊敬の「れる」を続けた「お［ご］…される」は誤り。助詞「を」を入れたり、「お［ご］…になる」「お［ご］…なさる」を使うのが適切。「課長が×ご説明された（○ご説明をされた）件ですが…」「先生が×お話しされた（○お話しになった）問題」"
                ]
            }
        ],
        26503,
        ""
    ]
]"##
        ).unwrap();
    }

    #[test]
    fn case_3() {
        serde_json::from_str::<GlossaryContent>(
            r##"
{
    "type": "structured-content",
    "content": [
        "さ・れる\n〘動下一〙\n",
        {
            "tag": "span",
            "content": {
                "tag": "img",
                "appearance": "monochrome",
                "height": 1.0,
                "width": 1.0,
                "sizeUnits": "em",
                "collapsible": false,
                "collapsed": false,
                "background": false,
                "path": "meikyo2/B172.png"
            },
            "style": {
                "verticalAlign": "text-bottom",
                "marginRight": 0.25
            }
        },
        " 〘他〙\n❶ 他動詞「する」の尊敬語。なさる。\n「先生が講演を━」\n「これからどう━おつもりですか」\n❷ 他動詞「する」の受身の形。\n「仲間に意地悪を━」\n",
        " 〘自他〙\n❶ サ変動詞「…する」の尊敬語。なさる。\n「さぞかしびっくり━・れたことでしょう」\n「先生は物理学を研究━・れています」\n❷ サ変動詞「…する」の受身の形。\n「山田氏は誰からも尊敬━・れている」\n「田中氏が学位を授与━・れる」\n",
        " 〘自〙《「…と━」の形で》ある意見や考えが、個人的なものではなく、一般的なものだとして提示する。一般に…と言われ（てい）る、…と考えられている。\n「一般に成功は難しいと━」\n「鶴は千年亀は万年生きると━」\n◆「する」の未然形＋助動詞「れる」から。\n",
        "謙譲の「お［ご］…する」に尊敬の「れる」を続けた「お［ご］…される」は誤り。助詞「を」を入れたり、「お［ご］…になる」「お［ご］…なさる」を使うのが適切。「課長が×ご説明された（○ご説明をされた）件ですが…」「先生が×お話しされた（○お話しになった）問題」"
    ]
}"##,
        )
        .unwrap();
    }

    #[test]
    fn case_4() {
        serde_json::from_str::<structured::ImageElement>(
            r##"
{
    "tag": "img",
    "appearance": "monochrome",
    "height": 1.0,
    "width": 1.0,
    "sizeUnits": "em",
    "collapsible": false,
    "collapsed": false,
    "background": false,
    "path": "meikyo2/B172.png"
}"##,
        )
        .unwrap();
    }
}
