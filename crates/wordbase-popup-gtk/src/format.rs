use derive_more::{Deref, DerefMut};
use foldhash::HashMap;
use gtk::prelude::{BoxExt, GridExt, WidgetExt};
use wordbase::{
    jp,
    schema::{Dictionary, DictionaryId, Frequency, Glossary, LookupInfo, Pitch, Term},
};

use crate::ui;

#[derive(Debug, Clone, Default, Deref, DerefMut)]
pub struct Terms(pub IndexMap<Term, TermInfo>);

type IndexMap<K, V> = indexmap::IndexMap<K, V, foldhash::fast::RandomState>;

#[derive(Debug, Clone, Default)]
pub struct TermInfo {
    pub glossaries: IndexMap<DictionaryId, (DictionaryTitle, Vec<Glossary>)>,
    pub frequencies: IndexMap<DictionaryId, (DictionaryTitle, Vec<Frequency>)>,
    pub pitches: IndexMap<DictionaryId, (DictionaryTitle, Vec<Pitch>)>,
}

pub type DictionaryTitle = String;

impl Terms {
    pub fn new(dictionaries: &HashMap<DictionaryId, Dictionary>, info: LookupInfo) -> Self {
        let title_of = |id: DictionaryId| -> DictionaryTitle {
            dictionaries
                .get(&id)
                .map_or_else(|| format!("{id:?}"), |dictionary| dictionary.title.clone())
        };

        let mut this = Self::default();

        for (source, term, glossary) in info.glossaries {
            this.entry(term)
                .or_default()
                .glossaries
                .entry(source)
                .or_insert_with(|| (title_of(source), Vec::new()))
                .1
                .push(glossary);
        }

        for (source, term, frequency) in info.frequencies {
            this.entry(term)
                .or_default()
                .frequencies
                .entry(source)
                .or_insert_with(|| (title_of(source), Vec::new()))
                .1
                .push(frequency);
        }

        for (source, term, pitch) in info.pitches {
            this.entry(term)
                .or_default()
                .pitches
                .entry(source)
                .or_insert_with(|| (title_of(source), Vec::new()))
                .1
                .push(pitch);
        }

        this
    }

    pub fn to_ui(&self) -> ui::Dictionary {
        let dictionary_ui = ui::Dictionary::new();

        for (row, (term, info)) in self.iter().enumerate() {
            let Ok(row) = i32::try_from(row) else {
                break;
            };

            // term meta (left)

            let meta_ui = ui::TermMeta::new();
            dictionary_ui.attach(&meta_ui, 0, row, 1, 1);

            meta_ui
                .reading()
                .set_text(term.reading.as_deref().unwrap_or_default());
            meta_ui.expression().set_text(&term.expression);

            for frequency_tag in info
                .frequencies
                .values()
                .map(|(dict_title, frequencies)| frequency_tag(dict_title, frequencies))
            {
                meta_ui.frequency_tags().append(&frequency_tag);
            }

            let reading = term.reading.as_ref().unwrap_or(&term.expression);
            for pitch_label in info
                .pitches
                .values()
                .flat_map(|(_, pitches)| pitches.iter().map(|pitch| pitch_label(reading, pitch)))
            {
                meta_ui.pitches().append(&pitch_label);
            }

            // glossaries (right)

            let page = ui::GlossaryPage::new();
            dictionary_ui.attach(&page, 1, row, 1, 1);

            for (dict_title, glossaries) in info.glossaries.values() {
                let group = ui::GlossaryGroup::new();
                page.append(&group);

                group.source().set_text(dict_title);

                for glossary in glossaries {
                    let row = ui::GlossaryRow::new();
                    group.append(&row);

                    let label = gtk::Label::new(Some(&glossary.text));
                    row.content().append(&label);
                    label.set_selectable(true);
                    label.set_wrap(true);
                    label.set_halign(gtk::Align::Start);
                }
            }
        }

        dictionary_ui
    }
}

fn frequency_tag(dict_title: &str, frequencies: &[Frequency]) -> ui::FrequencyTag {
    let tag = ui::FrequencyTag::new();

    tag.source().set_text(dict_title);

    let frequency = frequencies
        .iter()
        .map(|frequency| {
            frequency
                .display_rank
                .as_ref()
                .map_or_else(|| format!("{}", frequency.rank), ToOwned::to_owned)
        })
        .collect::<Vec<_>>()
        .join(" · ");
    tag.frequency().set_text(&frequency);
    tag
}

fn pitch_label(reading: &str, pitch: &Pitch) -> gtk::Box {
    let ui = gtk::Box::new(gtk::Orientation::Horizontal, 0);

    let downstep = usize::try_from(pitch.position).unwrap_or(usize::MAX);
    let mora = jp::mora(reading).collect::<Vec<_>>();

    let color_css_class = match downstep {
        0 => "heiban",
        1 => "atamadaka",
        n if n == mora.len() => "odaka",
        _ => "nakadaka",
    };

    for (position, mora) in mora.into_iter().enumerate() {
        let char_container = gtk::Overlay::new();
        ui.append(&char_container);

        let char_label = gtk::Label::new(Some(mora));
        char_container.set_child(Some(&char_label));
        char_label.add_css_class("mora");
        char_label.add_css_class(color_css_class);

        let pitch_line = gtk::Box::builder()
            .valign(gtk::Align::Start)
            .height_request(10)
            .css_classes(["pitch-line"])
            .build();
        char_container.add_overlay(&pitch_line);

        let is_high = jp::is_high(downstep, position);

        let css_class = if is_high { "high" } else { "low" };
        pitch_line.add_css_class(css_class);
        char_label.add_css_class(css_class);

        let is_next_high = jp::is_high(downstep, position + 1);
        let css_class = if is_next_high {
            "next-high"
        } else {
            "next-low"
        };
        pitch_line.add_css_class(css_class);
        char_label.add_css_class(css_class);
    }
    ui
}
