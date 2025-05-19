use {
    crate::group::Grouping,
    data_encoding::BASE64,
    derive_more::{Deref, DerefMut},
    maud::{Markup, PreEscaped, html},
    wordbase::{
        Dictionary, DictionaryId, FrequencyValue, Record, RecordLookup, Term,
        dict::{self, yomichan_audio::AudioFormat},
    },
    wordbase_engine::lang,
};

pub fn render_records<'a>(
    dictionary_by_id: &impl Fn(DictionaryId) -> Option<&'a Dictionary>,
    records: &[RecordLookup],
) -> Markup {
    let terms = make_terms(records);

    html! {
        @for (term, info) in &terms.0 {
            @let add_note_payload = serde_json::to_string(&term.0)
                .expect("should be able to serialize term");

            .term-group {
                .meta-group {
                    .term {
                        (render_term(term))
                    }

                    .actions {
                        button
                            .add-note
                            onclick=(format!("window.webkit.messageHandlers.add_note.postMessage({add_note_payload})"))
                        {
                            "Add Note"
                        }
                    }
                }

                .misc-group {
                    @for pitch in &info.pitches {
                        span .tag .pitch {
                            (render_pitch(term, pitch))
                        }
                    }

                    @for (_, audio) in &info.audio {
                        (render_audio(audio))
                    }

                    @for (&source, frequencies) in &info.frequencies {
                        span .tag .frequencies {
                            (render_frequencies(dictionary_by_id, source, frequencies))
                        }
                    }
                }

                @if !info.glossaries.is_empty() {
                    .glossaries {
                        @for (&source, glossaries) in &info.glossaries {
                            .one-source {
                                (render_glossaries(dictionary_by_id, source, glossaries))
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn make_terms(records: &[RecordLookup]) -> Terms {
    let mut terms = Terms::default();
    for record in records {
        let source = record.source;
        let grouped_term = Grouping(record.term.clone());
        let info = terms.entry(grouped_term).or_default();

        match &record.record {
            Record::YomitanGlossary(glossary) => {
                info.glossaries.entry(source).or_default().push(glossary);
            }
            Record::YomitanFrequency(frequency) => {
                info.frequencies.entry(source).or_default().push(frequency);
            }
            Record::YomitanPitch(pitch) => {
                if !info.pitches.contains(&pitch) {
                    info.pitches.push(pitch);
                }
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
    terms
}

type IndexMap<K, V> = indexmap::IndexMap<K, V, foldhash::fast::RandomState>;

#[derive(Debug, Default, Deref, DerefMut)]
pub struct Terms<'a>(IndexMap<Grouping<Term>, TermInfo<'a>>);

#[derive(Debug, Default)]
pub struct TermInfo<'a> {
    pub glossaries: IndexMap<DictionaryId, Glossaries<'a>>,
    pub frequencies: IndexMap<DictionaryId, Vec<&'a dict::yomitan::Frequency>>,
    pub pitches: Vec<&'a dict::yomitan::Pitch>,
    pub audio: Vec<(DictionaryId, Audio<'a>)>,
}

#[derive(Debug, Default, Deref, DerefMut)]
pub struct Glossaries<'a>(pub Vec<&'a dict::yomitan::Glossary>);

#[derive(Debug)]
pub enum Audio<'a> {
    Forvo(&'a dict::yomichan_audio::Forvo),
    Jpod(&'a dict::yomichan_audio::Jpod),
    Nhk16(&'a dict::yomichan_audio::Nhk16),
    Shinmeikai8(&'a dict::yomichan_audio::Shinmeikai8),
}

#[must_use]
pub fn render_term(term: &Term) -> Markup {
    match term {
        Term::Headword { headword } => html! {
            ruby {
                (headword)
            }
        },
        Term::Reading { reading } => html! {
            ruby {
                rt {
                    (reading)
                }
            }
        },
        Term::Full { headword, reading } => {
            let parts = lang::jpn::furigana_parts(headword, reading);
            html! {
                ruby {
                    @for (headword_part, reading_part) in parts {
                        (headword_part)

                        rt {
                            (reading_part)
                        }
                    }
                }
            }
        }
    }
}

#[must_use]
pub fn render_pitch(term: &Term, pitch: &dict::yomitan::Pitch) -> Markup {
    let reading = match term {
        Term::Full {
            headword: _,
            reading,
        }
        | Term::Reading { reading } => reading,
        Term::Headword { headword } => headword,
    };

    let downstep = usize::try_from(pitch.position).unwrap_or(usize::MAX);
    let morae = lang::jpn::morae(reading).collect::<Vec<_>>();

    let pitch_css_class = match downstep {
        0 => "heiban",
        1 => "atamadaka",
        n if n == morae.len() => "odaka",
        _ => "nakadaka",
    };

    let morae = morae.into_iter().enumerate().map(|(position, mora)| {
        let this_css_class = if lang::jpn::is_high(downstep, position) {
            "high"
        } else {
            "low"
        };

        let next_css_class = if lang::jpn::is_high(downstep, position + 1) {
            "next-high"
        } else {
            "next-low"
        };

        html! {
            span .mora .(this_css_class) .(next_css_class) {
                @for ch in mora.chars() {
                    span .char {
                        (ch)
                    }
                }
            }
        }
    });

    html! {
        span .(pitch_css_class) {
            @for mora in morae {
                (mora)
            }
        }
    }
}

pub fn render_frequencies<'a>(
    dictionary_by_id: &impl Fn(DictionaryId) -> Option<&'a Dictionary>,
    source: DictionaryId,
    frequencies: &[&dict::yomitan::Frequency],
) -> Markup {
    html! {
        span .source {
            (name_of(dictionary_by_id, source))
        }

        .values {
            @for frequency in frequencies {
                span .value {
                    (render_frequency(frequency))
                }
            }
        }
    }
}

#[must_use]
pub fn render_frequency(frequency: &dict::yomitan::Frequency) -> Markup {
    html! {
        (frequency
            .display
            .clone()
            .or_else(|| frequency.value.and_then(|value| match value {
                FrequencyValue::Rank(rank) => Some(format!("{rank}")),
                FrequencyValue::Occurrence(_) => None,
            }))
            .unwrap_or_else(|| "?".into()))
    }
}

#[must_use]
pub fn render_audio(record: &Audio) -> Markup {
    let (name, audio) = match record {
        Audio::Forvo(dict::yomichan_audio::Forvo { audio, username }) => {
            (username.to_string(), audio)
        }
        Audio::Jpod(dict::yomichan_audio::Jpod { audio }) => ("JPod".into(), audio),
        Audio::Nhk16(dict::yomichan_audio::Nhk16 { audio, .. }) => ("NHK".into(), audio),
        Audio::Shinmeikai8(dict::yomichan_audio::Shinmeikai8 { audio, .. }) => {
            ("Shinmeikai".into(), audio)
        }
    };

    let mime_type = match audio.format {
        AudioFormat::Opus => "audio/opus",
        AudioFormat::Mp3 => "audio/mp3",
    };
    let data = BASE64.encode(&audio.data);
    let on_click = format!("new Audio('data:{mime_type};base64,{data}').play()");

    html! {
        button onclick=(on_click) {
            (PreEscaped(include_str!("../assets/speakers-symbolic.svg")))

            (name)
        }
    }
}

#[must_use]
pub fn render_glossaries<'a>(
    dictionary_by_id: &impl Fn(DictionaryId) -> Option<&'a Dictionary>,
    source: DictionaryId,
    glossaries: &Glossaries,
) -> Markup {
    html! {
        span .source-name {
            (name_of(dictionary_by_id, source))
        }

        @for glossary in &glossaries.0 {
            .glossary {
                (render_glossary(glossary))
            }
        }
    }
}

#[must_use]
pub fn render_glossary(glossary: &dict::yomitan::Glossary) -> Markup {
    let mut tags = glossary.tags.iter().collect::<Vec<_>>();
    tags.sort_by_key(|tag| tag.order);

    html! {
        @if !tags.is_empty() {
            .tag-group {
                @for tag in tags {
                    .tag title=(tag.description) {
                        (tag.name)
                    }
                }
            }
        }

        ul .content data-count=(glossary.content.len()) {
            @for content in &glossary.content {
                li {
                    (content)
                }
            }
        }
    }
}

fn name_of<'a>(
    dictionary_by_id: &impl Fn(DictionaryId) -> Option<&'a Dictionary>,
    dictionary_id: DictionaryId,
) -> &'a str {
    dictionary_by_id(dictionary_id).map_or("?", |dict| dict.meta.name.as_str())
}
