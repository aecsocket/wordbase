use {
    crate::lang,
    base64::{Engine, prelude::BASE64_STANDARD},
    derive_more::{Deref, DerefMut},
    maud::{Markup, PreEscaped, html},
    std::fmt::Write as _,
    wordbase::{
        Dictionary, DictionaryId, Record, RecordLookup, Term,
        dict::{self, yomichan_audio::AudioFormat},
    },
};

pub fn render_records<'a>(
    dictionary_by_id: &impl Fn(DictionaryId) -> Option<&'a Dictionary>,
    records: &[RecordLookup],
) -> Markup {
    let terms = make_terms(records);

    html! {
        svg style="display: none;" {
            symbol id="speakers-symbolic" viewBox="0 0 16 16" {
                (r##"<path d="m 12.039062 0.00390625 c -0.257812 -0.01171875 -0.523437 0.07421875 -0.726562 0.28124975 l -3.3125 3.292969 v 1.421875 h 1.390625 l 3.304687 -3.296875 c 0.40625 -0.40625 0.363282 -1.042969 0.03125 -1.394531 c -0.175781 -0.183594 -0.429687 -0.292969 -0.6875 -0.30468775 z m -5.039062 1.00390575 c -0.296875 -0.003906 -0.578125 0.125 -0.765625 0.351563 l -3.234375 3.640625 h -1 c -1.09375 0 -2 0.84375 -2 2 v 2 c 0 1.089844 0.910156 2 2 2 h 1 l 3.234375 3.640625 c 0.207031 0.253906 0.488281 0.363281 0.765625 0.359375 z m 1 5.992188 v 2 h 6 c 0.75 0 1 -0.5 1 -1 s -0.25 -1 -1 -1 z m 0 4 v 1.421875 l 3.324219 3.292969 c 0.402343 0.410156 1.0625 0.347656 1.414062 -0.023438 c 0.332031 -0.351562 0.371094 -0.988281 -0.03125 -1.390625 l -3.316406 -3.300781 z m 0 0" fill="#222222"/>"##)
            }
        }

        @for (term, info) in &terms.0 {
            .term-group {
                .meta-group {
                    .meta {
                        .term {
                            (render_term(term))
                        }
                    }

                    .actions {
                        button {
                            "TODO: anki buttons"
                        }
                    }
                }

                .misc-group {
                    @for (_, audio) in &info.audio {
                        (render_audio(audio))
                    }

                    @for pitch in &info.pitches {
                        span .tag .pitch {
                            (render_pitch(term, pitch))
                        }
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
        let normalized_term = match &record.term {
            Term::Full { headword, reading } if headword == reading => Term::Headword {
                headword: headword.clone(),
            },
            Term::Reading { reading } => Term::Headword {
                headword: reading.clone(),
            },
            term => term.clone(),
        };
        let info = terms.entry(normalized_term).or_default();

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
pub struct Terms<'a>(IndexMap<Term, TermInfo<'a>>);

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
            .or_else(|| frequency.rank.map(|rank| format!("{}", rank.value())))
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
        Audio::Nhk16(dict::yomichan_audio::Nhk16 { audio }) => ("NHK".into(), audio),
        Audio::Shinmeikai8(dict::yomichan_audio::Shinmeikai8 { audio, .. }) => {
            ("Shinmeikai".into(), audio)
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
