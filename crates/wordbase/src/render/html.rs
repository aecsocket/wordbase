use {
    derive_more::{Deref, DerefMut},
    maud::{Markup, PreEscaped, Render, html},
    std::sync::Arc,
    wordbase::{
        DictionaryId, Record, Term, for_record_kinds, format, glossary, lang,
        protocol::LookupResponse, record,
    },
};

/// Renders [`LookupResponse`]s to HTML [`Markup`].
///
/// - `name_of_source`: a function which maps a dictionary ID to a
///   [`DictionaryMeta::name`].
/// - `records`: the records to render HTML for.
///
/// [`DictionaryMeta::name`]: wordbase::DictionaryMeta::name
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
        let cx = RenderContext {
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
                $(Record::$kind(data) => data.render(cx),)*
            }
        }}}

        for_record_kinds!(display_record);
    }
    terms.render()
}

type IndexMap<K, V> = indexmap::IndexMap<K, V, foldhash::fast::RandomState>;

#[derive(Debug, Default, Deref, DerefMut)]
struct Terms<'a>(IndexMap<Term, TermInfo<'a>>);

impl Render for Terms<'_> {
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
struct TermInfo<'a> {
    meta: TermMeta<'a>,
    glossaries: IndexMap<DictionaryId, GlossaryGroup>,
}

impl Render for TermInfo<'_> {
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
struct TermMeta<'a> {
    term: Term,
    jpn_pitches: Pitches<'a>,
    frequencies: Frequencies,
}

impl Render for TermMeta<'_> {
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

                (&self.jpn_pitches)
            }

            .meta {
                (&self.frequencies)
            }
        }
    }
}

#[derive(Debug, Default, Deref, DerefMut)]
struct Pitches<'a>(Vec<lang::jpn::PitchRender<'a>>);

impl Render for Pitches<'_> {
    fn render(&self) -> Markup {
        html! {
            .pitches {
                @for pitch in &self.0 {
                    (pitch)
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

struct RenderContext<'c> {
    source: DictionaryId,
    source_name: Arc<str>,
    meta: &'c mut TermMeta,
    glossaries: &'c mut GlossaryGroup,
}

trait RenderRecord {
    fn render(self, cx: RenderContext);
}

impl RenderRecord for record::Frequency {
    fn render(self, cx: RenderContext) {
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

impl RenderRecord for glossary::PlainText {
    fn render(self, cx: RenderContext) {
        cx.glossaries.rows.push(GlossaryRow {
            tags: Vec::new(),
            content: vec![html! { (self.0) }],
        });
    }
}

impl RenderRecord for glossary::Html {
    fn render(self, cx: RenderContext) {
        cx.glossaries.rows.push(GlossaryRow {
            tags: Vec::new(),
            content: vec![PreEscaped(self.0)],
        });
    }
}

impl RenderRecord for lang::jpn::Pitch {
    fn render(self, cx: RenderContext) {
        cx.meta.jpn_pitches.push(self);
    }
}

impl RenderRecord for format::yomitan::Glossary {
    fn render(self, cx: RenderContext) {
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
