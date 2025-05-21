#![expect(missing_docs, reason = "util crate")]

use std::time::Duration;

use anyhow::{Context, Result};
use data_encoding::BASE64;
use derive_more::{Deref, DerefMut};
use maud::Render;
use serde::Serialize;
use tera::Tera;
use tokio::fs;
use wordbase::{
    DictionaryId, Engine, IndexMap, ProfileId, Record, RecordKind, RecordLookup, Term,
    dict::{
        self,
        jpn::{PitchCategory, PitchPosition},
        yomichan_audio::AudioFormat,
    },
};

#[tokio::main]
async fn main() -> Result<()> {
    #[derive(Debug, Serialize)]
    struct TermWrapper<'a> {
        term: Term,
        #[serde(flatten)]
        info: TermInfo<'a>,
    }

    // let args = <Args as clap::Parser>::parse();
    let engine = Engine::new(wordbase::data_dir().context("failed to get data dir")?)
        .await
        .context("failed to create engine")?;

    let query = "見る";
    let records = engine
        .lookup(ProfileId(1), query, 0, RecordKind::ALL)
        .await
        .context("failed to perform lookup")?;
    let terms = group_terms(&records)
        .0
        .into_iter()
        .map(|(term, info)| TermWrapper { term, info })
        .collect::<Vec<_>>();

    let mut tera = Tera::new("record-templates/**/*").unwrap();

    loop {
        let mut context = tera::Context::new();
        context.insert("terms", &terms);
        context.insert("dictionaries", &engine.dictionaries().0);

        match tera.render("records.html", &context) {
            Ok(html) => {
                fs::write("records.html", &html)
                    .await
                    .context("failed to write HTML")?;
            }
            Err(err) => {
                eprintln!("render error: {err:?}");
            }
        }

        tokio::time::sleep(Duration::from_secs(1)).await;
        if let Err(err) = tera.full_reload() {
            eprintln!("failed to reload: {err:?}");
        }
    }
}

fn group_terms(records: &[RecordLookup]) -> RecordTerms {
    let mut groups = RecordTerms::default();
    for record in records {
        let source = record.source;
        let info = groups.entry(record.term.clone()).or_default();

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
                info.pitches.entry(pitch.position).or_default().info = Some(pitch);
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
                    for &pos in &audio.pitch_positions {
                        info.pitches
                            .entry(pos)
                            .or_default()
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
                    info.pitches.entry(pos).or_default().audio.push(conv);
                } else {
                    info.audio_no_pitch.entry(source).or_default().push(conv);
                }
            }
            _ => {}
        }
    }
    groups
}

#[derive(Debug, Default, Deref, DerefMut, Serialize)]
struct RecordTerms<'a>(IndexMap<Term, TermInfo<'a>>);

#[derive(Debug, Default, Serialize)]
struct TermInfo<'a> {
    glossary_groups: IndexMap<DictionaryId, Vec<Glossary<'a>>>,
    frequencies: IndexMap<DictionaryId, Vec<&'a dict::yomitan::Frequency>>,
    pitches: IndexMap<PitchPosition, Pitch<'a>>,
    audio_no_pitch: IndexMap<DictionaryId, Vec<Audio>>,
}

#[derive(Debug, Serialize)]
struct Glossary<'a> {
    tags: &'a Vec<dict::yomitan::GlossaryTag>,
    content: Vec<String>,
}

#[derive(Debug, Default, Serialize)]
struct Pitch<'a> {
    info: Option<&'a dict::yomitan::Pitch>,
    audio: Vec<Audio>,
}

#[derive(Debug, Clone, Serialize)]
struct Audio {
    blob: String,
    kind: RecordKind,
}

fn audio_blob(audio: &dict::yomichan_audio::Audio) -> String {
    let mime_type = match audio.format {
        AudioFormat::Opus => "audio/opus",
        AudioFormat::Mp3 => "audio/mp3",
    };
    let data = BASE64.encode(&audio.data);
    format!("data:{mime_type};base64,{data}")
}
