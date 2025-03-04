#![doc = include_str!("../README.md")]

use core::alloc;
use std::{
    collections::HashMap,
    convert::Infallible,
    fs::{self, File},
    io::Cursor,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

use anyhow::{Context, Result};
use tracing::info;
use wordbase::{DEFAULT_PORT, protocol::Lookup, yomitan};

#[derive(Debug, clap::Parser)]
struct Args {
    dictionary: PathBuf,
    lookup: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().init();
    let args = <Args as clap::Parser>::parse();

    let dictionary = fs::read(&args.dictionary).context("failed to read dictionary")?;

    let (parser, index) = yomitan::Parse::new(|| Ok::<_, Infallible>(Cursor::new(&dictionary)))
        .context("failed to parse dictionary index")?;
    let term_banks_left = AtomicUsize::new(parser.term_banks().len());

    info!("Parsing dictionary {:?}", index.title);

    struct Entry {
        reading: String,
        glossary: Vec<yomitan::Glossary>,
    }

    #[derive(Default)]
    struct State {
        expressions: HashMap<String, Vec<Entry>>,
        reading_to_expressions: HashMap<String, Vec<String>>,
    }

    let state = Mutex::new(State::default());

    parser
        .run(
            |_, _| {},
            |_, terms| {
                let mut state = state.lock().expect("poisoned");
                for term in terms.0 {
                    state
                        .expressions
                        .entry(term.expression.clone())
                        .or_default()
                        .push(Entry {
                            reading: term.reading.clone(),
                            glossary: term.glossary,
                        });

                    state
                        .reading_to_expressions
                        .entry(term.reading)
                        .or_default()
                        .push(term.expression);
                }
                drop(state);

                let term_banks_left = term_banks_left.fetch_sub(1, Ordering::SeqCst);
                info!("{term_banks_left} term banks left");
            },
            |_, _| {},
            |_, _| {},
            |_, _| {},
        )
        .context("failed to parse dictionary")?;

    let state = state.lock().expect("poisoned");
    info!(
        "{} expressions, {} readings",
        state.expressions.len(),
        state.reading_to_expressions.len()
    );

    let entries = state.expressions.get(&args.lookup).expect("no entries");
    for entry in entries {
        info!("=== {} ===", entry.reading);

        for glossary in &entry.glossary {
            match glossary {
                yomitan::Glossary::String(s) => info!("{s}"),
                yomitan::Glossary::Content(yomitan::GlossaryContent::Text { text }) => {
                    info!("{text}");
                }
                yomitan::Glossary::Content(yomitan::GlossaryContent::Image(image)) => {
                    info!("(image {})", image.path);
                }
                yomitan::Glossary::Content(yomitan::GlossaryContent::StructuredContent {
                    content,
                }) => {
                    print_strings(content);
                }
                yomitan::Glossary::Deinflection(_) => {}
            }
        }

        info!("{:#?}", entry.glossary);
    }

    drop(state);

    Ok(())
}

fn print_strings(c: &yomitan::structured::Content) {
    use yomitan::structured::Element;

    match c {
        yomitan::structured::Content::String(s) => info!("{s}"),
        yomitan::structured::Content::Element(elem) => match &**elem {
            Element::Br { .. } => {}
            Element::Ruby(e)
            | Element::Rt(e)
            | Element::Rp(e)
            | Element::Table(e)
            | Element::Thead(e)
            | Element::Tbody(e)
            | Element::Tfoot(e)
            | Element::Tr(e) => {
                if let Some(c) = &e.content {
                    print_strings(c);
                }
            }
            Element::Td(e) | Element::Th(e) => {
                if let Some(c) = &e.content {
                    print_strings(c);
                }
            }
            Element::Span(e)
            | Element::Div(e)
            | Element::Ol(e)
            | Element::Ul(e)
            | Element::Li(e)
            | Element::Details(e)
            | Element::Summary(e) => {
                if let Some(c) = &e.content {
                    print_strings(c);
                }
            }
            Element::Img(e) => {
                info!("(image {})", e.base.path);
            }
            Element::A(e) => {
                if let Some(c) = &e.content {
                    print_strings(c);
                }
            }
        },
        yomitan::structured::Content::Content(c) => {
            for c in c {
                print_strings(c);
            }
        }
    }
}
