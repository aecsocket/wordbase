use derive_more::{Deref, DerefMut};
use foldhash::HashMap;
use gtk::prelude::{BoxExt, GridExt};
use wordbase::schema::{Dictionary, DictionaryId, Frequency, Glossary, LookupInfo, Pitch, Term};

use crate::ui;

#[derive(Debug, Clone, Default, Deref, DerefMut)]
pub struct Terms(pub IndexMap<Term, TermInfo>);

type IndexMap<K, V> = indexmap::IndexMap<K, V, foldhash::fast::RandomState>;

#[derive(Debug, Clone, Default)]
pub struct TermInfo {
    pub glossaries: IndexMap<DictionaryId, (DictionaryTitle, Vec<Glossary>)>,
    pub frequencies: IndexMap<DictionaryId, (DictionaryTitle, Vec<Frequency>)>,
    pub pitches: IndexMap<DictionaryId, (DictionaryTitle, Vec<Pitch>)>,
}

pub type DictionaryTitle = String;

impl Terms {
    pub fn new(dictionaries: &HashMap<DictionaryId, Dictionary>, info: LookupInfo) -> Self {
        let title_of = |id: DictionaryId| -> DictionaryTitle {
            dictionaries
                .get(&id)
                .map_or_else(|| format!("{id:?}"), |dictionary| dictionary.title.clone())
        };

        let mut this = Self::default();

        for (source, term, glossary) in info.glossaries {
            this.entry(term)
                .or_default()
                .glossaries
                .entry(source)
                .or_insert_with(|| (title_of(source), Vec::new()))
                .1
                .push(glossary);
        }

        for (source, term, frequency) in info.frequencies {
            this.entry(term)
                .or_default()
                .frequencies
                .entry(source)
                .or_insert_with(|| (title_of(source), Vec::new()))
                .1
                .push(frequency);
        }

        for (source, term, pitch) in info.pitches {
            this.entry(term)
                .or_default()
                .pitches
                .entry(source)
                .or_insert_with(|| (title_of(source), Vec::new()))
                .1
                .push(pitch);
        }

        this
    }

    pub fn to_ui(&self) -> ui::Dictionary {
        let dictionary = ui::Dictionary::new();

        for (row, (term, info)) in self.iter().enumerate() {
            let Ok(row) = i32::try_from(row) else {
                break;
            };
            let meta = ui::TermMeta::new();
            dictionary.attach(&meta, 0, row, 1, 1);

            meta.reading()
                .set_text(term.reading.as_deref().unwrap_or_default());
            meta.expression().set_text(&term.expression);

            for (_, (dict_title, frequencies)) in &info.frequencies {
                let tag = ui::FrequencyTag::new();
                meta.frequency_tags().append(&tag);

                tag.dictionary().set_text(dict_title);

                let frequency = frequencies
                    .iter()
                    .map(|frequency| {
                        frequency
                            .display_rank
                            .as_ref()
                            .map_or_else(|| format!("{}", frequency.rank), ToOwned::to_owned)
                    })
                    .collect::<Vec<_>>()
                    .join(" · ");
                tag.frequency().set_text(&frequency);
            }
        }

        dictionary
    }
}
