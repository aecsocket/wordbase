#![doc = include_str!("../README.md")]
#![expect(missing_docs)]
#![expect(clippy::missing_errors_doc)]

use std::sync::Arc;

use derive_more::{Deref, DerefMut};
use maud::{Markup, PreEscaped, Render, html};
use wordbase::{
    DictionaryId, Record, RecordKind, Term, for_record_kinds, format, glossary, lang,
    protocol::LookupResponse, record,
};

pub const SUPPORTED_RECORD_KINDS: &[RecordKind] = &[
    // meta
    RecordKind::JpPitch,
    RecordKind::Frequency,
    // glossaries
    RecordKind::GlossaryPlainText,
    RecordKind::GlossaryHtml,
    RecordKind::YomitanGlossary,
];

pub fn to_html(
    name_of_source: impl Fn(DictionaryId) -> Arc<str>,
    records: impl IntoIterator<Item = LookupResponse>,
) -> Markup {
    let mut terms = Terms::default();
    for record in records {
        let term_info = terms
            .entry(record.term.clone())
            .or_insert_with(|| TermInfo {
                meta: TermMeta {
                    term: record.term.clone(),
                    ..Default::default()
                },
                ..Default::default()
            });
        let source_name = name_of_source(record.source);
        let cx = RecordContext {
            source: record.source,
            source_name: source_name.clone(),
            meta: &mut term_info.meta,
            glossaries: term_info
                .glossaries
                .entry(record.source)
                .or_insert_with(|| GlossaryGroup {
                    source: source_name,
                    rows: Vec::new(),
                }),
        };

        macro_rules! display_record { ($($kind:ident($data_ty:path)),* $(,)?) => {{
            match record.record {
                $(Record::$kind(data) => data.insert(cx),)*
                _ => {}
            }
        }}}

        for_record_kinds!(display_record);
    }
    terms.render()
}

type IndexMap<K, V> = indexmap::IndexMap<K, V, foldhash::fast::RandomState>;

#[derive(Debug, Default, Deref, DerefMut)]
struct Terms(IndexMap<Term, TermInfo>);

impl Render for Terms {
    fn render(&self) -> Markup {
        html! {
            .terms {
                @for (_, term) in &self.0 {
                    (term)
                }
            }
        }
    }
}

#[derive(Debug, Default)]
struct TermInfo {
    meta: TermMeta,
    glossaries: IndexMap<DictionaryId, GlossaryGroup>,
}

impl Render for TermInfo {
    fn render(&self) -> Markup {
        html! {
            .header {
                (&self.meta)
            }

            .glossary-page {
                @for (_, group) in &self.glossaries {
                    @if !group.rows.is_empty() {
                        (group)
                    }
                }
            }
        }
    }
}

#[derive(Debug, Default)]
struct TermMeta {
    term: Term,
    jp_pitches: Pitches,
    frequencies: Frequencies,
}

impl Render for TermMeta {
    fn render(&self) -> Markup {
        html! {
            .term {
                ruby {
                    (&self.term.headword)

                    @if let Some(reading) = &self.term.reading {
                        rt {
                            (reading)
                        }
                    }
                }

                (&self.jp_pitches)
            }

            .meta {
                (&self.frequencies)
            }
        }
    }
}

#[derive(Debug, Default, Deref, DerefMut)]
struct Pitches(Vec<lang::jp::Pitch>);

impl Render for Pitches {
    fn render(&self) -> Markup {
        html! {
            .pitches {
                @for pitch in &self.0 {
                    .pitch {
                        "TODO PITCH"
                    }
                }
            }
        }
    }
}

#[derive(Debug, Default, Deref, DerefMut)]
struct Frequencies(IndexMap<DictionaryId, FrequencyGroup>);

impl Render for Frequencies {
    fn render(&self) -> Markup {
        html! {
            .frequencies {
                @for (_, group) in &self.0 {
                    (group)
                }
            }
        }
    }
}

#[derive(Debug)]
struct FrequencyGroup {
    source: Arc<str>,
    values: Vec<record::Frequency>,
}

impl Render for FrequencyGroup {
    fn render(&self) -> Markup {
        html! {
            .group {
                span .source {
                    (&self.source)
                }

                .values {
                    @for frequency in &self.values {
                        span .value {
                            (frequency.rank)
                        }
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
struct GlossaryGroup {
    source: Arc<str>,
    rows: Vec<GlossaryRow>,
}

impl Render for GlossaryGroup {
    fn render(&self) -> Markup {
        html! {
            .group {
                span .source {
                    (&self.source)
                }

                .rows {
                    @for row in &self.rows { (row) }
                }
            }
        }
    }
}

#[derive(Debug, Default)]
struct GlossaryRow {
    tags: Vec<GlossaryTag>,
    content: Vec<Markup>,
}

impl Render for GlossaryRow {
    fn render(&self) -> Markup {
        html! {
            .row {
                @if !self.tags.is_empty() {
                    .tags {
                        @for tag in &self.tags {
                            (tag)
                        }
                    }
                }

                ul .content data-count=(self.content.len()) {
                    @for content in &self.content {
                        li {
                            (content)
                        }
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
struct GlossaryTag {
    name: String,
    description: String,
    css_class: Option<String>,
}

impl Render for GlossaryTag {
    fn render(&self) -> Markup {
        html! {
            .tag title=(self.description) class=[&(self.css_class)] {
                (self.name)
            }
        }
    }
}

struct RecordContext<'c> {
    source: DictionaryId,
    source_name: Arc<str>,
    meta: &'c mut TermMeta,
    glossaries: &'c mut GlossaryGroup,
}

trait RecordInsert {
    fn insert(self, cx: RecordContext);
}

impl RecordInsert for record::Frequency {
    fn insert(self, cx: RecordContext) {
        cx.meta
            .frequencies
            .entry(cx.source)
            .or_insert_with(|| FrequencyGroup {
                source: cx.source_name,
                values: Vec::new(),
            })
            .values
            .push(self);
    }
}

impl RecordInsert for glossary::PlainText {
    fn insert(self, cx: RecordContext) {
        cx.glossaries.rows.push(GlossaryRow {
            tags: Vec::new(),
            content: vec![html! { (self.0) }],
        });
    }
}

impl RecordInsert for glossary::PlainTextFallback {
    fn insert(self, cx: RecordContext) {
        cx.glossaries.rows.push(GlossaryRow {
            tags: Vec::new(),
            content: vec![html! { (self.0) }],
        });
    }
}

impl RecordInsert for glossary::Html {
    fn insert(self, cx: RecordContext) {
        cx.glossaries.rows.push(GlossaryRow {
            tags: Vec::new(),
            content: vec![PreEscaped(self.0)],
        });
    }
}

impl RecordInsert for glossary::HtmlFallback {
    fn insert(self, cx: RecordContext) {
        cx.glossaries.rows.push(GlossaryRow {
            tags: Vec::new(),
            content: vec![PreEscaped(self.0)],
        });
    }
}

impl RecordInsert for lang::jp::Pitch {
    fn insert(self, cx: RecordContext) {
        cx.meta.jp_pitches.push(self);
    }
}

impl RecordInsert for format::yomitan::Glossary {
    fn insert(self, cx: RecordContext) {
        cx.glossaries.rows.push(GlossaryRow {
            tags: self
                .tags
                .into_iter()
                .map(|tag| GlossaryTag {
                    name: tag.name,
                    description: tag.description,
                    css_class: css_class_of_category(&tag.category).map(ToOwned::to_owned),
                })
                .collect(),
            content: self
                .content
                .into_iter()
                .map(|content| content.render())
                .collect(),
        });
    }
}

fn css_class_of_category(category: &str) -> Option<&'static str> {
    match category {
        "name" => Some("name"),
        "expression" => Some("expression"),
        "popular" => Some("popular"),
        "frequent" => Some("frequent"),
        "archaism" => Some("archaism"),
        "dictionary" => Some("dictionary"),
        "frequency" => Some("frequency"),
        "partOfSpeech" => Some("part-of-speech"),
        "search" => Some("search"),
        "pronunciation-dictionary" => Some("pronunciation-dictionary"),
        _ => None,
    }
}
