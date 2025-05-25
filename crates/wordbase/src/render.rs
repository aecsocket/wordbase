use anyhow::Result;
use data_encoding::BASE64;
use foldhash::HashSet;
use maud::Render as _;
use serde::Serialize;
use wordbase_api::{DictionaryId, Record, RecordKind, RecordLookup, Term, dict};

use crate::{Engine, IndexMap};

impl Engine {
    pub fn render_to_html(
        &self,
        records: &[RecordLookup],
        config: &RenderConfig,
    ) -> Result<String> {
        let terms = group_terms(records);

        let mut context = tera::Context::new();
        context.insert("dictionaries", &self.dictionaries().0);
        context.insert("terms", &terms);
        context.insert("config", config);

        let html = self.renderer.render("records.html", &context)?;
        Ok(html)
    }
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct RenderConfig {
    pub add_card_text: String,
    pub add_card_js_fn: String,
}

fn group_terms(records: &[RecordLookup]) -> Vec<RecordTerm> {
    // note on ordering:
    // by default, tera will not preserve the order of IndexMap entries,
    // because serde_json doesn't either.
    // we enable `tera/preserve_order` to make sure that order stays.
    let mut groups = IndexMap::<Term, TermInfo>::default();
    for record in records {
        let source = record.source;
        let term = &record.term;
        let info = groups.entry(term.clone()).or_insert_with(|| TermInfo {
            furigana_parts: match term {
                Term::Full(headword, reading) => dict::jpn::furigana_parts(headword, reading)
                    .into_iter()
                    .map(|(a, b)| (a.to_owned(), b.to_owned()))
                    .collect::<Vec<_>>(),
                Term::Headword(text) | Term::Reading(text) => {
                    vec![(text.to_string(), String::new())]
                }
            },
            morae: term.reading().map_or(Vec::new(), |reading| {
                dict::jpn::morae(reading).map(ToOwned::to_owned).collect()
            }),
            ..Default::default()
        });

        match &record.record {
            Record::YomitanGlossary(glossary) => {
                info.glossary_groups
                    .entry(source)
                    .or_default()
                    .push(Glossary {
                        tags: &glossary.tags,
                        content: glossary
                            .content
                            .iter()
                            .map(|content| content.render().0)
                            .collect(),
                    });
            }
            Record::YomitanFrequency(frequency) => {
                info.frequencies.entry(source).or_default().push(frequency);
            }
            Record::YomitanPitch(pitch) => {
                info.pitches
                    .entry(pitch.position)
                    .or_insert_with(|| base_pitch(term, pitch.position))
                    .info = Some(pitch);
            }
            Record::YomichanAudioForvo(audio) => {
                info.audio_no_pitch.entry(source).or_default().push(Audio {
                    blob: audio_blob(&audio.audio),
                    kind: RecordKind::YomichanAudioForvo,
                });
            }
            Record::YomichanAudioJpod(audio) => {
                info.audio_no_pitch.entry(source).or_default().push(Audio {
                    blob: audio_blob(&audio.audio),
                    kind: RecordKind::YomichanAudioJpod,
                });
            }
            Record::YomichanAudioNhk16(audio) => {
                let conv = Audio {
                    blob: audio_blob(&audio.audio),
                    kind: RecordKind::YomichanAudioNhk16,
                };
                if audio.pitch_positions.is_empty() {
                    info.audio_no_pitch.entry(source).or_default().push(conv);
                } else {
                    for &pos in &audio
                        .pitch_positions
                        .iter()
                        .copied()
                        // deduplicate pitches
                        .collect::<HashSet<_>>()
                    {
                        info.pitches
                            .entry(pos)
                            .or_insert_with(|| base_pitch(term, pos))
                            .audio
                            .push(conv.clone());
                    }
                }
            }
            Record::YomichanAudioShinmeikai8(audio) => {
                let conv = Audio {
                    blob: audio_blob(&audio.audio),
                    kind: RecordKind::YomichanAudioShinmeikai8,
                };
                if let Some(pos) = audio.pitch_number {
                    info.pitches
                        .entry(pos)
                        .or_insert_with(|| base_pitch(term, pos))
                        .audio
                        .push(conv);
                } else {
                    info.audio_no_pitch.entry(source).or_default().push(conv);
                }
            }
            _ => {}
        }
    }

    groups
        .into_iter()
        .map(|(term, info)| RecordTerm { term, info })
        .collect()
}

#[derive(Debug, Serialize)]
struct RecordTerm<'a> {
    term: Term,
    #[serde(flatten)]
    info: TermInfo<'a>,
}

#[derive(Debug, Default, Serialize)]
struct TermInfo<'a> {
    furigana_parts: Vec<(String, String)>,
    morae: Vec<String>,
    glossary_groups: IndexMap<DictionaryId, Vec<Glossary<'a>>>,
    frequencies: IndexMap<DictionaryId, Vec<&'a dict::yomitan::Frequency>>,
    pitches: IndexMap<dict::jpn::PitchPosition, Pitch<'a>>,
    audio_no_pitch: IndexMap<DictionaryId, Vec<Audio>>,
}

#[derive(Debug, Serialize)]
struct Glossary<'a> {
    tags: &'a Vec<dict::yomitan::GlossaryTag>,
    content: Vec<String>,
}

#[derive(Debug, Default, Serialize)]
struct Pitch<'a> {
    category: Option<dict::jpn::PitchCategory>,
    high: Vec<bool>,
    info: Option<&'a dict::yomitan::Pitch>,
    audio: Vec<Audio>,
}

fn base_pitch<'a>(term: &Term, downstep: dict::jpn::PitchPosition) -> Pitch<'a> {
    let Some(reading) = term.reading() else {
        return Pitch::default();
    };

    let downstep = usize::try_from(downstep.0).unwrap_or(usize::MAX);
    let n_morae = dict::jpn::morae(reading).count();
    let category = dict::jpn::pitch_category_of(n_morae, downstep);
    Pitch {
        category: Some(category),
        high: (0..=n_morae)
            .map(|pos| dict::jpn::is_high(downstep, pos))
            .collect(),
        ..Default::default()
    }
}

#[derive(Debug, Clone, Serialize)]
struct Audio {
    blob: String,
    kind: RecordKind,
}

fn audio_blob(audio: &dict::yomichan_audio::Audio) -> String {
    let mime_type = match audio.format {
        dict::yomichan_audio::AudioFormat::Opus => "audio/opus",
        dict::yomichan_audio::AudioFormat::Mp3 => "audio/mp3",
    };
    let data = BASE64.encode(&audio.data);
    format!("data:{mime_type};base64,{data}")
}

#[cfg(feature = "uniffi")]
const _: () = {
    use crate::{FfiResult, Wordbase};

    #[uniffi::export]
    impl Wordbase {
        pub fn render_to_html(
            &self,
            records: &[RecordLookup],
            config: &RenderConfig,
        ) -> FfiResult<String> {
            Ok(self.0.render_to_html(records, config)?)
        }
    }
};
