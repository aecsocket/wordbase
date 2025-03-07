use std::{
    convert::Infallible,
    io::Cursor,
    iter,
    path::Path,
    sync::atomic::{self, AtomicUsize},
};

use anyhow::{Context as _, Result};
use sqlx::{Pool, Sqlite, Transaction};
use tokio::{fs, sync::Mutex};
use tracing::info;
use wordbase::{
    DictionaryId, Frequency, Glossary, TagCategory, TermTag,
    lang::jp,
    yomitan::{self, structured},
};

use crate::db::{data_kind, serialize};

pub async fn from_yomitan(db: Pool<Sqlite>, path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    let archive = fs::read(path)
        .await
        .context("failed to read file into memory")?;

    let (parser, index) = yomitan::Parse::new(|| Ok::<_, Infallible>(Cursor::new(&archive)))
        .context("failed to parse")?;

    let name = index.title;
    let already_present = sqlx::query_scalar!(
        "SELECT EXISTS(
            SELECT 1
            FROM dictionaries
            WHERE name = $1
        )",
        name
    )
    .fetch_one(&db)
    .await
    .context("failed to check if dictionary is already present")?;
    if already_present > 0 {
        info!("{name} is already present, skipping...");
        return Ok(());
    }

    let tag_banks_left = AtomicUsize::new(parser.tag_banks().len());
    let term_banks_left = AtomicUsize::new(parser.term_banks().len());
    let term_meta_banks_left = AtomicUsize::new(parser.term_meta_banks().len());
    info!("Parsing: {name}");

    let tag_bank = Mutex::new(yomitan::TagBank::default());
    let term_bank = Mutex::new(yomitan::TermBank::default());
    let term_meta_bank = Mutex::new(yomitan::TermMetaBank::default());

    info!("Parsing...");
    let span = tracing::Span::current();
    parser
        .run(
            |_, bank| {
                let _span = span.enter();
                tag_bank.blocking_lock().extend_from_slice(&bank);
                let left = tag_banks_left.fetch_sub(1, atomic::Ordering::SeqCst) - 1;
                _ = left;
            },
            |_, bank| {
                let _span = span.enter();
                term_bank.blocking_lock().extend_from_slice(&bank);

                let left = term_banks_left.fetch_sub(1, atomic::Ordering::SeqCst) - 1;
                if left % 10 == 0 {
                    info!("{left} term banks left");
                }
            },
            |_, bank| {
                let _span = span.enter();
                term_meta_bank.blocking_lock().extend_from_slice(&bank);

                let left = term_meta_banks_left.fetch_sub(1, atomic::Ordering::SeqCst) - 1;
                if left % 10 == 0 {
                    info!("{left} term meta banks left");
                }
            },
            |_, _| {},
            |_, _| {},
        )
        .context("failed to parse banks")?;
    info!("Parse complete, starting transaction...");
    let mut tx = db.begin().await.context("failed to begin transaction")?;

    let dictionary_id = {
        info!("Writing dictionary record...");
        let result = sqlx::query!(
            "INSERT INTO dictionaries (name, version)
            VALUES ($1, $2)",
            name,
            index.revision,
        )
        .execute(&mut *tx)
        .await
        .context("failed to insert dictionary")?;

        DictionaryId(result.last_insert_rowid())
    };

    let mut all_tags = tag_bank
        .into_inner()
        .into_iter()
        .map(to_term_tag)
        .collect::<Vec<_>>();
    all_tags.sort_by(|tag_a, tag_b| tag_b.name.len().cmp(&tag_a.name.len()));

    info!("Importing term bank");
    import_term_bank(dictionary_id, &mut tx, term_bank.into_inner(), &all_tags)
        .await
        .context("failed to import term bank")?;

    info!("Importing term meta bank");
    import_term_meta_bank(dictionary_id, &mut tx, term_meta_bank.into_inner())
        .await
        .context("failed to import term meta bank")?;

    info!("Committing...");
    tx.commit().await.context("failed to commit transaction")?;

    info!("*=* COMPLETE *=*");
    Ok(())
}

fn none_if_empty(s: String) -> Option<String> {
    if s.trim().is_empty() { None } else { Some(s) }
}

async fn import_term_bank(
    DictionaryId(source): DictionaryId,
    tx: &mut Transaction<'_, Sqlite>,
    bank: yomitan::TermBank,
    all_tags: &[TermTag],
) -> Result<()> {
    let mut scratch = Vec::<u8>::new();
    let mut records_left = bank.len();
    for term in bank {
        let headword = term.expression.clone();
        let reading = none_if_empty(term.reading.clone());

        let tags = match_tags(
            all_tags,
            term.definition_tags.as_deref().unwrap_or_default(),
        )
        .cloned()
        .collect::<Vec<_>>();

        let content = term
            .glossary
            .into_iter()
            .filter_map(to_glossary_content)
            .collect();

        let mut glossary = Glossary::default();
        glossary.tags = tags;
        glossary.html = todo!();

        async {
            scratch.clear();
            serialize(&glossary, &mut scratch).context("failed to serialize data")?;
            let data = &scratch[..];

            sqlx::query!(
                "INSERT INTO terms (source, headword, reading, data_kind, data)
                VALUES ($1, $2, $3, $4, $5)",
                source,
                headword,
                reading,
                data_kind::GLOSSARY,
                data
            )
            .execute(&mut **tx)
            .await
            .context("failed to insert record")?;

            records_left -= 1;
            if records_left % 10000 == 0 {
                info!("IMPORT: {records_left} term records left");
            }

            anyhow::Ok(())
        }
        .await
        .with_context(|| format!("failed to import term {headword:?} ({reading:?})"))?;
    }
    Ok(())
}

// `all_tags` must be sorted longest-first
fn match_tags<'a>(
    all_tags: &'a [TermTag],
    mut definition_tags: &str,
) -> impl Iterator<Item = &'a TermTag> {
    iter::from_fn(move || {
        definition_tags = definition_tags.trim();
        for tag in all_tags {
            if let Some(stripped) = definition_tags.strip_prefix(&tag.name) {
                definition_tags = stripped;
                return Some(tag);
            }
        }
        None
    })
}

async fn import_term_meta_bank(
    DictionaryId(source): DictionaryId,
    tx: &mut Transaction<'_, Sqlite>,
    bank: yomitan::TermMetaBank,
) -> Result<()> {
    let mut scratch = Vec::<u8>::new();
    let mut records_left = bank.len();
    for term_meta in bank {
        let headword = term_meta.expression.clone();
        async {
            match term_meta.data {
                yomitan::TermMetaData::Frequency(frequency) => {
                    for (reading, frequency) in to_frequencies(frequency) {
                        scratch.clear();
                        serialize(&frequency, &mut scratch).context("failed to serialize data")?;
                        let data = &scratch[..];

                        sqlx::query!(
                            "INSERT INTO terms (source, headword, reading, data_kind, data)
                            VALUES ($1, $2, $3, $4, $5)",
                            source,
                            headword,
                            reading,
                            data_kind::FREQUENCY,
                            data,
                        )
                        .execute(&mut **tx)
                        .await
                        .context("failed to insert record")?;
                    }
                }
                yomitan::TermMetaData::Pitch(pitch) => {
                    for (reading, pitch) in to_pitch(pitch) {
                        scratch.clear();
                        serialize(&pitch, &mut scratch).context("failed to serialize data")?;
                        let data = &scratch[..];

                        sqlx::query!(
                            "INSERT INTO terms (source, headword, reading, data_kind, data)
                            VALUES ($1, $2, $3, $4, $5)",
                            source,
                            headword,
                            reading,
                            data_kind::JP_PITCH,
                            data,
                        )
                        .execute(&mut **tx)
                        .await
                        .context("failed to insert record")?;
                    }
                }
                yomitan::TermMetaData::Phonetic(_) => {}
            }

            records_left -= 1;
            if records_left % 10000 == 0 {
                info!("IMPORT: {records_left} term meta records left");
            }

            anyhow::Ok(())
        }
        .await
        .with_context(|| format!("failed to import term meta {headword:?}"))?;
    }
    Ok(())
}

fn to_term_tag(raw: yomitan::Tag) -> TermTag {
    TermTag {
        name: raw.name,
        category: match raw.category.as_str() {
            "name" => Some(TagCategory::Name),
            "expression" => Some(TagCategory::Expression),
            "popular" => Some(TagCategory::Popular),
            "frequent" => Some(TagCategory::Frequent),
            "archaism" => Some(TagCategory::Archaism),
            "dictionary" => Some(TagCategory::Dictionary),
            "frequency" => Some(TagCategory::Frequency),
            "partOfSpeech" => Some(TagCategory::PartOfSpeech),
            "search" => Some(TagCategory::Search),
            "pronunciation-dictionary" => Some(TagCategory::PronunciationDictionary),
            _ => None,
        },
        description: raw.notes,
        order: raw.order,
    }
}

fn to_glossary_content(raw: yomitan::Glossary) -> Option<structured::Content> {
    match raw {
        yomitan::Glossary::Deinflection(_) => None,
        yomitan::Glossary::String(text)
        | yomitan::Glossary::Content(yomitan::GlossaryContent::Text { text }) => {
            Some(structured::Content::String(text))
        }
        yomitan::Glossary::Content(yomitan::GlossaryContent::Image(base)) => {
            Some(structured::Content::Element(Box::new(
                structured::Element::Img(structured::ImageElement {
                    base,
                    ..Default::default()
                }),
            )))
        }
        yomitan::Glossary::Content(yomitan::GlossaryContent::StructuredContent { content }) => {
            Some(content)
        }
    }
}

fn to_frequencies(
    raw: yomitan::TermMetaFrequency,
) -> impl Iterator<Item = (Option<String>, Frequency)> {
    let (reading, generic) = match raw {
        yomitan::TermMetaFrequency::Generic(generic) => (None, generic),
        yomitan::TermMetaFrequency::WithReading { reading, frequency } => {
            (Some(reading), frequency)
        }
    };

    let frequency = match generic {
        yomitan::GenericFrequencyData::Number(rank) => Some(Frequency::new(rank)),
        yomitan::GenericFrequencyData::String(rank) => {
            // best-effort attempt
            rank.trim().parse::<u64>().map(Frequency::new).ok()
        }
        yomitan::GenericFrequencyData::Complex {
            value: rank,
            display_value: display_rank,
        } => Some(Frequency { rank, display_rank }),
    };

    frequency.map(|new| (reading, new)).into_iter()
}

fn to_pitch(raw: yomitan::TermMetaPitch) -> impl Iterator<Item = (String, jp::Pitch)> {
    raw.pitches.into_iter().map(move |pitch| {
        (
            raw.reading.clone(),
            jp::Pitch {
                position: pitch.position,
                nasal: to_pitch_positions(pitch.nasal),
                devoice: to_pitch_positions(pitch.devoice),
            },
        )
    })
}

fn to_pitch_positions(raw: Option<yomitan::PitchPosition>) -> Vec<u64> {
    match raw {
        None => vec![],
        Some(yomitan::PitchPosition::One(position)) => vec![position],
        Some(yomitan::PitchPosition::Many(positions)) => positions,
    }
}
