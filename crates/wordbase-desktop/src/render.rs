use derive_more::{Deref, DerefMut};
use maud::{Markup, html};
use wordbase::{DictionaryId, Record, RecordLookup, Term, dict};

pub fn to_html(records: impl IntoIterator<Item = RecordLookup>) -> Markup {
    let mut terms = Terms::default();
    for RecordLookup {
        source,
        term,
        record,
    } in records
    {
        let info = terms.entry(term).or_default();

        match record {
            Record::YomitanGlossary(glossary) => {
                info.glossaries.entry(source).or_default().push(glossary);
            }
            Record::YomitanFrequency(frequency) => {
                info.frequencies.push((source, frequency));
            }
            Record::YomitanPitch(pitch) => {
                info.pitches.push((source, pitch));
            }
            _ => {}
        }
    }

    html! {
        ul {
            @for (term, info) in terms.0 {
                li {
                    "term = "
                    ({ format!("{term:?}") })
                }
            }
        }
    }
}

#[derive(Debug, Default, Deref, DerefMut)]
struct Terms(IndexMap<Term, TermInfo>);

#[derive(Debug, Default)]
struct TermInfo {
    frequencies: Vec<(DictionaryId, dict::yomitan::Frequency)>,
    pitches: Vec<(DictionaryId, dict::yomitan::Pitch)>,
    glossaries: IndexMap<DictionaryId, Vec<dict::yomitan::Glossary>>,
}

type IndexMap<K, V> = indexmap::IndexMap<K, V, foldhash::fast::RandomState>;
