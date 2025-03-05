use derive_more::Deref;
use foldhash::HashMap;
use wordbase::schema::{self, Frequency, Glossary, LookupInfo, Pitch};

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
    pub fn new(info: LookupInfo) -> Self {
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

        let mut map = TermMap::default();

        for term in info.terms {
            let (term, _) = convert_term(term);
            map.get_or_new(term);
        }

        for (term, frequency) in info.frequencies {
            let (term, source_title) = convert_term(term);
            map.get_or_new(term)
                .frequencies
                .push((source_title, frequency));
        }

        for (term, pitch) in info.pitches {
            let (term, source_title) = convert_term(term);
            map.get_or_new(term.into())
                .pitches
                .push((source_title, pitch));
        }

        for (term, glossary) in info.glossaries {
            let (term, source_title) = convert_term(term);
            map.get_or_new(term.into())
                .glossaries
                .push((source_title, glossary));
        }

        Self(map.into_ordered())
    }
}

fn convert_term(raw: schema::Term) -> (Term, DictionaryTitle) {
    (
        Term {
            expression: raw.expression,
            reading: raw.reading,
        },
        raw.source_title,
    )
}
