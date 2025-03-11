#![doc = include_str!("../README.md")]
#![expect(missing_docs)]
#![expect(clippy::missing_errors_doc)]

extern crate gtk4 as gtk;
extern crate libadwaita as adw;
extern crate webkit6 as webkit;

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
        let term_info = terms.entry(record.term.clone()).or_default();
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
                    (group)
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

                ul {
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
    css_class: String,
}

impl Render for GlossaryTag {
    fn render(&self) -> Markup {
        html! {
            .tag title=(self.description) class=(self.css_class) {
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
                    css_class: "TODO".into(),
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

// pub const STYLESHEET: &str = include_str!("style.css");

// #[derive(Debug, Default)]
// struct GlossaryInfo {
//     tags: Vec<gtk::Widget>,
//     content: Vec<gtk::Widget>,
// }

// struct RecordContext<'a> {
//     term: &'a Term,
//     source_name: &'a str,
//     meta_info: &'a mut MetaInfo,
//     glossary_group: &'a mut Vec<GlossaryInfo>,
// }

// trait AddToTermInfo {
//     fn add_to_term_info(self, cx: RecordContext);
// }

// pub fn ui_for<'a>(
//     source_name: impl Fn(DictionaryId) -> &'a str,
//     records: impl IntoIterator<Item = LookupResponse>,
// ) -> ui::Dictionary {
//     let mut terms = IndexMap::<Term, TermInfo>::default();
//     for record in records {
//         let term_info = terms.entry(record.term.clone()).or_default();
//         let record_context = RecordContext {
//             term: &record.term,
//             source_name: source_name(record.source),
//             meta_info: &mut term_info.meta,
//             glossary_group: term_info.glossaries.entry(record.source).or_default(),
//         };

//         macro_rules! add_to_term_info { ($($kind:ident($data_ty:path)),* $(,)?) => {{
//             match record.record {
//                 $(Record::$kind(value) => value.add_to_term_info(record_context),)*
//                 _ => {}
//             }
//         }}}

//         for_record_kinds!(add_to_term_info);
//     }

//     let ui = ui::Dictionary::new();
//     for (row, (term, info)) in terms.into_iter().enumerate() {
//         let Ok(row) = i32::try_from(row) else {
//             continue;
//         };

//         let (meta_ui, glossary_page) = ui_for_term(&source_name, term, info);
//         ui.attach(&meta_ui, 0, row, 1, 1);
//         ui.attach(&glossary_page, 1, row, 1, 1);
//     }
//     ui
// }

// fn ui_for_term<'a>(
//     source_name: &impl Fn(DictionaryId) -> &'a str,
//     term: Term,
//     info: TermInfo,
// ) -> (ui::TermMeta, ui::GlossaryPage) {
//     let meta_ui = ui::TermMeta::new();
//     meta_ui
//         .reading()
//         .set_text(term.reading.as_deref().unwrap_or_default());
//     meta_ui.headword().set_text(&term.headword);
//     for pitch in info.meta.pitches {
//         meta_ui.pitches().append(&pitch);
//     }
//     for frequency in info.meta.frequencies {
//         meta_ui.frequencies().append(&frequency);
//     }

//     let glossary_page = ui::GlossaryPage::new();
//     for (source, glossaries) in info.glossaries {
//         if glossaries.is_empty() {
//             continue;
//         }

//         let glossary_group = ui::GlossaryGroup::new();
//         glossary_page.append(&glossary_group);
//         glossary_group.source().set_text(source_name(source));

//         for glossary_info in glossaries {
//             let glossary_row = ui::GlossaryRow::new();
//             glossary_group.append(&glossary_row);

//             for tag in glossary_info.tags {
//                 glossary_row.tags().append(&tag);
//             }

//             for content in glossary_info.content {
//                 glossary_row.content().append(&content);
//             }
//         }
//     }

//     (meta_ui, glossary_page)
// }
