use {
    crate::{Engine, IndexMap, lang},
    anyhow::{Context, Result},
    arc_swap::ArcSwap,
    data_encoding::BASE64,
    foldhash::HashSet,
    serde::Serialize,
    tera::Tera,
    wordbase_api::{DictionaryId, Record, RecordEntry, RecordKind, Term, dict},
};

#[derive(Debug)]
pub struct Renderer {
    tera: ArcSwap<Tera>,
}

impl Renderer {
    pub fn new() -> Result<Self> {
        let mut tera = Tera::default();
        tera.add_raw_template("records.html", include_str!("records.html"))
            .context("failed to add record render template")?;
        Ok(Self {
            tera: ArcSwap::from_pointee(tera),
        })
    }
}

impl Engine {
    /// Renders the results of [`Engine::lookup`] to the `<body>` contents of
    /// an HTML document, so you can display it to the user in a web view or
    /// similar.
    ///
    /// To get a valid HTML document, you will need to add:
    /// - `<!doctype html>`
    /// - `<html></html>`
    /// - `<body></body>`
    ///
    /// You can also include your own HTML to further customize the output,
    /// such as adding your own `<style>` block.
    ///
    /// # Errors
    ///
    /// Errors if the HTML template cannot be rendered by [`tera`]. This should
    /// not happen normally, but if you are modifying the template and
    /// hot-reloading it, then this may error. It is usually safe to just
    /// `expect` this to be [`Ok`].
    pub fn render_html_body(
        &self,
        entries: &[RecordEntry],
        config: &RenderConfig,
    ) -> Result<String> {
        let terms = group_terms(entries);

        let mut context = tera::Context::new();
        context.insert("dictionaries", &self.dictionaries().0);
        context.insert("terms", &terms);
        context.insert("config", config);
        let body = self.renderer.tera.load().render("records.html", &context)?;

        Ok(body)
    }
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct RenderConfig {
    /// Translated text string "Add Note".
    pub s_add_note: String,
    pub s_view_note: String,
    pub s_add_duplicate_note: String,
    /// Template for calling a JS function to get how many notes in Anki already
    /// exist for a given term.
    ///
    /// # Examples
    ///
    /// ```text
    /// const num_notes = Wordbase.num_existing_notes(<js_headword>, <js_reading>);
    /// <js_callback>(num_notes);
    /// ```
    pub fn_num_existing_notes: String,
    /// Template for calling a JS function to add an Anki note for a given term.
    ///
    /// # Examples
    ///
    /// ```text
    /// Wordbase.add_note(<js_headword>, <js_reading>);
    /// <js_callback>();
    /// ```
    ///
    /// ```text
    /// window.wordbase.callNative(
    ///     'add_note',
    ///     { headword: <js_headword>, reading: <js_reading> },
    ///     result => <js_callback>(JSON.parse(result)),
    /// )
    /// ```
    pub fn_add_new_note: String,
    pub fn_add_duplicate_note: String,
    pub fn_view_note: String,
}

pub fn group_terms(entries: &[RecordEntry]) -> Vec<RecordTerm> {
    // note on ordering:
    // by default, tera will not preserve the order of IndexMap entries,
    // because serde_json doesn't either.
    // we enable `tera/preserve_order` to make sure that order stays.
    let mut groups = IndexMap::<Term, TermInfo>::default();
    for record in entries {
        let source = record.source;
        let term = &record.term;
        let info = groups.entry(term.clone()).or_insert_with(|| TermInfo {
            furigana_parts: match term {
                Term::Full(headword, reading) => lang::jpn::furigana_parts(headword, reading)
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
                            .map(|content| dict::yomitan::render_html(content).0)
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
                    kind: RecordKind::YomichanAudioForvo,
                    blob: audio_blob(&audio.audio),
                });
            }
            Record::YomichanAudioJpod(audio) => {
                info.audio_no_pitch.entry(source).or_default().push(Audio {
                    kind: RecordKind::YomichanAudioJpod,
                    blob: audio_blob(&audio.audio),
                });
            }
            Record::YomichanAudioNhk16(audio) => {
                let conv = Audio {
                    kind: RecordKind::YomichanAudioNhk16,
                    blob: audio_blob(&audio.audio),
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
                    kind: RecordKind::YomichanAudioShinmeikai8,
                    blob: audio_blob(&audio.audio),
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
pub struct RecordTerm<'a> {
    pub term: Term,
    #[serde(flatten)]
    pub info: TermInfo<'a>,
}

#[derive(Debug, Default, Serialize)]
pub struct TermInfo<'a> {
    pub furigana_parts: Vec<(String, String)>,
    pub morae: Vec<String>,
    pub glossary_groups: IndexMap<DictionaryId, Vec<Glossary<'a>>>,
    pub frequencies: IndexMap<DictionaryId, Vec<&'a dict::yomitan::Frequency>>,
    pub pitches: IndexMap<dict::jpn::PitchPosition, Pitch<'a>>,
    pub audio_no_pitch: IndexMap<DictionaryId, Vec<Audio>>,
}

#[derive(Debug, Serialize)]
pub struct Glossary<'a> {
    pub tags: &'a Vec<dict::yomitan::GlossaryTag>,
    pub content: Vec<String>,
}

#[derive(Debug, Default, Serialize)]
pub struct Pitch<'a> {
    pub category: Option<dict::jpn::PitchCategory>,
    pub high: Vec<bool>,
    pub info: Option<&'a dict::yomitan::Pitch>,
    pub audio: Vec<Audio>,
}

#[must_use]
pub fn base_pitch<'a>(term: &Term, downstep: dict::jpn::PitchPosition) -> Pitch<'a> {
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
pub struct Audio {
    pub kind: RecordKind,
    pub blob: String,
}

fn audio_mime_type(audio: &dict::yomichan_audio::Audio) -> &'static str {
    match audio.format {
        dict::yomichan_audio::AudioFormat::Opus => "audio/opus",
        dict::yomichan_audio::AudioFormat::Mp3 => "audio/mp3",
    }
}

fn audio_blob(audio: &dict::yomichan_audio::Audio) -> String {
    let mime_type = audio_mime_type(audio);
    let data = BASE64.encode(&audio.data);
    format!("data:{mime_type};base64,{data}")
}

#[cfg(feature = "uniffi")]
const _: () = {
    use crate::{FfiResult, Wordbase};

    #[uniffi::export]
    impl Wordbase {
        pub fn render_html_body(
            &self,
            entries: &[RecordEntry],
            config: &RenderConfig,
        ) -> FfiResult<String> {
            Ok(self.0.render_html_body(entries, config)?)
        }
    }
};
