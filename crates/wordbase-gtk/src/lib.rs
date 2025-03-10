#![doc = include_str!("../README.md")]
#![expect(missing_docs)]
#![expect(clippy::missing_errors_doc)]

extern crate gtk4 as gtk;
extern crate libadwaita as adw;
extern crate webkit6 as webkit;

pub mod format;
mod glossary;
pub mod lang;
mod record;
pub mod ui;

use gtk::prelude::*;
use wordbase::{
    DictionaryId, Record, RecordKind, Term, for_record_kinds, protocol::LookupResponse,
};

pub const STYLESHEET: &str = include_str!("style.css");

pub const SUPPORTED_RECORD_KINDS: &[RecordKind] = &[
    // meta
    RecordKind::JpPitch,
    RecordKind::Frequency,
    // glossaries
    RecordKind::GlossaryPlainText,
    RecordKind::GlossaryHtml,
    RecordKind::YomitanGlossary,
];

type IndexMap<K, V> = indexmap::IndexMap<K, V, foldhash::fast::RandomState>;

#[derive(Debug, Default)]
struct MetaInfo {
    pitches: Vec<gtk::Widget>,
    frequencies: Vec<gtk::Widget>,
}

#[derive(Debug, Default)]
struct GlossaryInfo {
    tags: Vec<gtk::Widget>,
    content: Vec<gtk::Widget>,
}

struct RecordContext<'a> {
    term: &'a Term,
    source_name: &'a str,
    meta_info: &'a mut MetaInfo,
    glossary_group: &'a mut Vec<GlossaryInfo>,
}

trait AddToTermInfo {
    fn add_to_term_info(self, cx: RecordContext);
}

#[derive(Debug, Default)]
struct TermInfo {
    meta: MetaInfo,
    glossary_page: IndexMap<DictionaryId, Vec<GlossaryInfo>>,
}

pub fn ui_for<'a>(
    source_name: impl Fn(DictionaryId) -> &'a str,
    records: impl IntoIterator<Item = LookupResponse>,
) -> ui::Dictionary {
    let mut terms = IndexMap::<Term, TermInfo>::default();
    for record in records {
        let term_info = terms.entry(record.term.clone()).or_default();
        let record_context = RecordContext {
            term: &record.term,
            source_name: source_name(record.source),
            meta_info: &mut term_info.meta,
            glossary_group: term_info.glossary_page.entry(record.source).or_default(),
        };

        macro_rules! add_to_term_info { ($($kind:ident($data_ty:path)),* $(,)?) => {{
            match record.record {
                $(Record::$kind(value) => value.add_to_term_info(record_context),)*
                _ => {}
            }
        }}}

        for_record_kinds!(add_to_term_info);
    }

    let ui = ui::Dictionary::new();
    for (row, (term, info)) in terms.into_iter().enumerate() {
        let Ok(row) = i32::try_from(row) else {
            continue;
        };

        let (meta_ui, glossary_page) = ui_for_term(&source_name, term, info);
        ui.attach(&meta_ui, 0, row, 1, 1);
        ui.attach(&glossary_page, 1, row, 1, 1);
    }
    ui
}

fn ui_for_term<'a>(
    source_name: &impl Fn(DictionaryId) -> &'a str,
    term: Term,
    info: TermInfo,
) -> (ui::TermMeta, ui::GlossaryPage) {
    let meta_ui = ui::TermMeta::new();
    meta_ui
        .reading()
        .set_text(term.reading.as_deref().unwrap_or_default());
    meta_ui.headword().set_text(&term.headword);
    for pitch in info.meta.pitches {
        meta_ui.pitches().append(&pitch);
    }
    for frequency in info.meta.frequencies {
        meta_ui.frequencies().append(&frequency);
    }

    let glossary_page = ui::GlossaryPage::new();
    for (source, glossaries) in info.glossary_page {
        let glossary_group = ui::GlossaryGroup::new();
        glossary_group.source().set_text(source_name(source));

        for glossary_info in glossaries {
            let glossary_row = ui::GlossaryRow::new();
            for tag in glossary_info.tags {
                glossary_row.tags().append(&tag);
            }

            for content in glossary_info.content {
                glossary_row.content().append(&content);
            }
        }
    }

    (meta_ui, glossary_page)
}
