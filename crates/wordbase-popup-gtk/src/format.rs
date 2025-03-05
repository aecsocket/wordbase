use derive_more::Deref;
use foldhash::HashMap;
use wordbase::schema::{self, Dictionary, DictionaryId, Frequency, Glossary, LookupInfo, Pitch};

#[derive(Debug, Clone, Deref)]
pub struct Format(Vec<(Term, TermInfo)>);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Term {
    pub expression: String,
    pub reading: String,
}

#[derive(Debug, Clone, Default)]
pub struct TermInfo {
    pub frequencies: Vec<(DictionaryTitle, Frequency)>,
    pub pitches: Vec<(DictionaryTitle, Pitch)>,
    pub glossaries: Vec<(DictionaryTitle, Glossary)>,
}

pub type DictionaryTitle = String;

impl Format {
    pub fn new(dictionaries: &Vec<Dictionary>, info: LookupInfo) -> Self {
        #[derive(Debug, Clone, Default)]
        struct TermMap {
            terms: HashMap<Term, TermInfo>,
            indices: Vec<Term>,
        }

        impl TermMap {
            fn get_or_new(&mut self, key: Term) -> &mut TermInfo {
                self.terms.entry(key).or_insert_with_key(|key| {
                    self.indices.push(key.clone());
                    TermInfo::default()
                })
            }

            fn into_ordered(mut self) -> Vec<(Term, TermInfo)> {
                self.indices
                    .into_iter()
                    .map(|index| self.terms.remove_entry(&index).expect("should exist"))
                    .collect()
            }
        }

        let source_title_of = |id: DictionaryId| -> DictionaryTitle {
            dictionaries
                .iter()
                .find(|dictionary| dictionary.id == id)
                .map(|dictionary| dictionary.title.clone())
                .unwrap_or_else(|| format!("{id:?}"))
        };

        let mut map = TermMap::default();

        for term in info.terms {
            map.get_or_new(term.into());
        }

        for (term, frequency) in info.frequencies {
            let source_title = source_title_of(term.source);
            map.get_or_new(term.into())
                .frequencies
                .push((source_title, frequency));
        }

        for (term, pitch) in info.pitches {
            let source_title = source_title_of(term.source);
            map.get_or_new(term.into())
                .pitches
                .push((source_title, pitch));
        }

        for (term, glossary) in info.glossaries {
            let source_title = source_title_of(term.source);
            map.get_or_new(term.into())
                .glossaries
                .push((source_title, glossary));
        }

        Self(map.into_ordered())
    }
}

impl From<schema::Term> for Term {
    fn from(value: schema::Term) -> Self {
        Self {
            expression: value.expression,
            reading: value.reading,
        }
    }
}
