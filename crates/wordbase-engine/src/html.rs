use {
    crate::lang,
    base64::{Engine, prelude::BASE64_STANDARD},
    derive_more::{Deref, DerefMut},
    maud::{Markup, html},
    std::fmt::Write as _,
    wordbase::{
        Dictionary, DictionaryId, Record, RecordLookup, Term,
        dict::{self, yomichan_audio::AudioFormat},
    },
};

pub fn render_records(
    dictionaries: &IndexMap<DictionaryId, Dictionary>,
    records: &[RecordLookup],
) -> Markup {
    let terms = make_terms(records);

    html! {
        @for (term, info) in &terms.0 {
            .term-group {
                .term-meta {
                    .term {
                        (render_term(term))
                    }

                    .pitch-group {
                        @for (_, pitch) in &info.pitches {
                            .pitch {
                                (render_pitch(term, pitch))
                            }
                        }
                    }

                    .frequency-group {
                        @for &(source, frequency) in &info.frequencies {
                            .frequency {
                                (render_frequency(dictionaries, source, frequency))
                            }
                        }
                    }

                    .audio-group {
                        @for (_, audio) in &info.audio {
                            .audio {
                                (render_audio(audio))
                            }
                        }
                    }
                }

                .glossaries {
                    @for (&source, glossaries) in &info.glossaries {
                        .one-source {
                            (render_glossaries(dictionaries, source, glossaries))
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
    terms
}

type IndexMap<K, V> = indexmap::IndexMap<K, V, foldhash::fast::RandomState>;

#[derive(Debug, Default, Deref, DerefMut)]
pub struct Terms<'a>(IndexMap<Term, TermInfo<'a>>);

#[derive(Debug, Default)]
pub struct TermInfo<'a> {
    pub glossaries: IndexMap<DictionaryId, Glossaries<'a>>,
    pub frequencies: Vec<(DictionaryId, &'a dict::yomitan::Frequency)>,
    pub pitches: Vec<(DictionaryId, &'a dict::yomitan::Pitch)>,
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
    let Some(reading) = term.reading() else {
        return html! {};
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
        .(pitch_css_class) {
            @for mora in morae {
                (mora)
            }
        }
    }
}

#[must_use]
pub fn render_frequency(
    dictionaries: &IndexMap<DictionaryId, Dictionary>,
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

#[must_use]
pub fn render_audio(record: &Audio) -> Markup {
    let (name, audio) = match record {
        Audio::Forvo(dict::yomichan_audio::Forvo { audio, username }) => {
            (format!("Forvo {username}"), audio)
        }
        Audio::Jpod(dict::yomichan_audio::Jpod { audio }) => ("JPod".into(), audio),
        Audio::Nhk16(dict::yomichan_audio::Nhk16 { audio }) => ("Nhk16".into(), audio),
        Audio::Shinmeikai8(dict::yomichan_audio::Shinmeikai8 { audio, .. }) => {
            ("Shinmeikai8".into(), audio)
        }
    };

    let mime_type = match audio.format {
        AudioFormat::Opus => "audio/opus",
    };
    let mut on_click = format!("new Audio('data:{mime_type};base64,");
    BASE64_STANDARD.encode_string(&audio.data, &mut on_click);
    _ = write!(&mut on_click, "').play()");

    html! {
        button onclick=(on_click) {
            "Play Audio " (name)
        }
    }
}

#[must_use]
pub fn render_glossaries(
    dictionaries: &IndexMap<DictionaryId, Dictionary>,
    source: DictionaryId,
    glossaries: &Glossaries,
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

#[must_use]
pub fn render_glossary(glossary: &dict::yomitan::Glossary) -> Markup {
    let mut tags = glossary.tags.iter().collect::<Vec<_>>();
    tags.sort_by_key(|tag| tag.order);

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

fn name_of(dictionaries: &IndexMap<DictionaryId, Dictionary>, dictionary_id: DictionaryId) -> &str {
    dictionaries
        .get(&dictionary_id)
        .map_or("?", |dict| dict.meta.name.as_str())
}
