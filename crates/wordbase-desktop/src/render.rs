use std::sync::Arc;

use derive_more::{Deref, DerefMut};
use foldhash::{HashMap, HashMapExt};
use maud::{Markup, html};
use relm4::{
    adw::{gdk, prelude::*},
    prelude::*,
};
use webkit6::prelude::*;
use wordbase::{Dictionary, DictionaryId, Record, RecordLookup, Term, dict};

#[derive(Debug)]
pub struct Model {
    theme_css: String,
    web_view: webkit6::WebView,
    dictionaries: Arc<HashMap<DictionaryId, Dictionary>>,
    records: Vec<RecordLookup>,
}

#[derive(Debug)]
pub struct Init {
    pub theme_css: String,
}

#[derive(Debug)]
pub enum Msg {
    SetThemeCss {
        theme_css: String,
    },
    Lookup {
        dictionaries: Arc<HashMap<DictionaryId, Dictionary>>,
        records: Vec<RecordLookup>,
    },
}

#[derive(Debug)]
pub enum Response {}

#[relm4::component(pub)]
impl SimpleComponent for Model {
    type Init = Init;
    type Input = Msg;
    type Output = Response;

    view! {
        webkit6::WebView {
            set_hexpand: true,
            set_vexpand: true,
            set_background_color: &gdk::RGBA::new(0.0, 0.0, 0.0, 0.0),
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let widgets = view_output!();
        let model = Self {
            theme_css: init.theme_css,
            web_view: root,
            dictionaries: Arc::new(HashMap::new()),
            records: Vec::new(),
        };
        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>) {
        match message {
            Msg::SetThemeCss { theme_css } => {
                self.theme_css = theme_css;
                self.update_web_view();
            }
            Msg::Lookup {
                dictionaries,
                records,
            } => {
                self.dictionaries = dictionaries;
                self.records = records;
                self.update_web_view();
            }
        }
    }
}

impl Model {
    fn update_web_view(&self) {
        let record_html = to_html(&self.dictionaries, &self.records);
        let full_html = html! {
            style {
                (self.theme_css)
            }

            (record_html)
        };
        self.web_view.load_html(&full_html.0, None);
    }
}

fn to_html(dictionaries: &HashMap<DictionaryId, Dictionary>, records: &[RecordLookup]) -> Markup {
    let mut terms = Terms::default();
    for record in records {
        let source = record.source;
        let info = terms.entry(record.term.clone()).or_default();

        match &record.record {
            Record::YomitanGlossary(glossary) => {
                info.glossaries.entry(source).or_default().push(glossary);
            }
            Record::YomitanFrequency(frequency) => {
                info.frequencies.push((source, frequency));
            }
            Record::YomitanPitch(pitch) => {
                info.pitches.push((source, pitch));
            }
            Record::YomichanAudioForvo(audio) => {
                info.audio.push((source, Audio::Forvo(audio)));
            }
            Record::YomichanAudioJpod(audio) => {
                info.audio.push((source, Audio::Jpod(audio)));
            }
            Record::YomichanAudioNhk16(audio) => {
                info.audio.push((source, Audio::Nhk16(audio)));
            }
            Record::YomichanAudioShinmeikai8(audio) => {
                info.audio.push((source, Audio::Shinmeikai8(audio)));
            }
            _ => {}
        }
    }

    html! {
        @for (term, info) in &terms.0 {
            .term-box {
                .term {
                    (render_term(term))
                }

                .pitch-box {
                    @for (_, pitch) in &info.pitches {
                        .pitch {
                            (render_pitch(term, pitch))
                        }
                    }
                }

                .frequency-box {
                    @for &(source, frequency) in &info.frequencies {
                        .frequency {
                            (render_frequency(dictionaries, source, frequency))
                        }
                    }
                }

                .source-glossaries-box {
                    @for (&source, glossaries) in &info.glossaries {
                        .glossaries {
                            (render_glossaries(dictionaries, source, glossaries))
                        }
                    }
                }
            }
        }
    }
}

type IndexMap<K, V> = indexmap::IndexMap<K, V, foldhash::fast::RandomState>;

#[derive(Debug, Default, Deref, DerefMut)]
struct Terms<'a>(IndexMap<Term, TermInfo<'a>>);

#[derive(Debug, Default)]
struct TermInfo<'a> {
    glossaries: IndexMap<DictionaryId, SourceGlossaries<'a>>,
    frequencies: Vec<(DictionaryId, &'a dict::yomitan::Frequency)>,
    pitches: Vec<(DictionaryId, &'a dict::yomitan::Pitch)>,
    audio: Vec<(DictionaryId, Audio<'a>)>,
}

#[derive(Debug, Default, Deref, DerefMut)]
struct SourceGlossaries<'a>(Vec<&'a dict::yomitan::Glossary>);

#[derive(Debug)]
enum Audio<'a> {
    Forvo(&'a dict::yomichan_audio::Forvo),
    Jpod(&'a dict::yomichan_audio::Jpod),
    Nhk16(&'a dict::yomichan_audio::Nhk16),
    Shinmeikai8(&'a dict::yomichan_audio::Shinmeikai8),
}

fn render_term(term: &Term) -> Markup {
    html! {
        ruby {
            (term.headword().map(|s| s.as_str()).unwrap_or_default())

            rt {
                (term.reading().map(|s| s.as_str()).unwrap_or_default())
            }
        }
    }
}

fn render_pitch(_term: &Term, _pitch: &dict::yomitan::Pitch) -> Markup {
    html! { "TODO" }
}

fn render_frequency(
    dictionaries: &HashMap<DictionaryId, Dictionary>,
    source: DictionaryId,
    frequency: &dict::yomitan::Frequency,
) -> Markup {
    html! {
        span .source {
            (name_of(dictionaries, source))
        }

        span .value {
            (frequency
                .display
                .clone()
                .or_else(|| frequency.rank.map(|rank| format!("{}", rank.value())))
                .unwrap_or_else(|| "?".into())
            )
        }
    }
}

fn render_glossaries(
    dictionaries: &HashMap<DictionaryId, Dictionary>,
    source: DictionaryId,
    glossaries: &SourceGlossaries,
) -> Markup {
    html! {
        span .source-name {
            (name_of(dictionaries, source))
        }

        @for glossary in &glossaries.0 {
            .glossary {
                (render_glossary(glossary))
            }
        }
    }
}

fn render_glossary(glossary: &dict::yomitan::Glossary) -> Markup {
    let mut tags = glossary.tags.iter().collect::<Vec<_>>();
    tags.sort_by(|tag_a, tag_b| tag_a.order.cmp(&tag_b.order));

    html! {
        @for tag in tags {
            .tag title=(tag.description) {
                (tag.name)
            }
        }

        ul {
            @for content in &glossary.content {
                li {
                    (content)
                }
            }
        }
    }
}

fn name_of(dictionaries: &HashMap<DictionaryId, Dictionary>, dictionary_id: DictionaryId) -> &str {
    dictionaries
        .get(&dictionary_id)
        .map_or("?", |dict| dict.meta.name.as_str())
}
