use std::{fmt::Write, sync::LazyLock, time::Duration};

use derive_more::{Deref, DerefMut};
use foldhash::HashMap;
use gtk::{gdk, gio, prelude::*};
use log::{info, warn};
use webkit::prelude::{PolicyDecisionExt, WebViewExt};
use wordbase::{
    jp,
    schema::{Dictionary, DictionaryId, Frequency, Glossary, LookupInfo, Pitch, Term},
    yomitan::{self, structured},
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

                    for tag in &glossary.tags {
                        let tag_ui = ui::GlossaryTag::new();
                        row.tags().append(&tag_ui);

                        tag_ui.set_label(&tag.name);
                        tag_ui.set_tooltip_text(Some(&tag.description));
                        if let Some(category) = tag.category {
                            tag_ui.add_css_class(ui::GlossaryTag::css_class_of(category));
                        }
                    }

                    row.content().append(&glossary_webview(&glossary.content));
                }
            }
        }

        dictionary_ui
    }
}

fn frequency_tag(dict_title: &str, frequencies: &[Frequency]) -> gtk::Widget {
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
    tag.upcast()
}

fn pitch_label(reading: &str, pitch: &Pitch) -> gtk::Widget {
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
        let container = gtk::Overlay::builder()
            .css_classes(["mora-container"])
            .build();
        ui.append(&container);

        let label = gtk::Label::new(Some(mora));
        container.set_child(Some(&label));
        label.add_css_class("mora");
        label.add_css_class(color_css_class);

        let pitch_line = gtk::Box::builder()
            .valign(gtk::Align::Start)
            .height_request(10) // TODO un-hardcode
            .css_classes(["pitch-line"])
            .build();
        container.add_overlay(&pitch_line);

        let is_high = jp::is_high(downstep, position);
        let base_css_class = if is_high { "high" } else { "low" };

        let is_next_high = jp::is_high(downstep, position + 1);
        let next_css_class = if is_next_high {
            "next-high"
        } else {
            "next-low"
        };

        for widget in [
            container.upcast_ref::<gtk::Widget>(),
            label.upcast_ref(),
            pitch_line.upcast_ref(),
        ] {
            widget.add_css_class(base_css_class);
            widget.add_css_class(next_css_class);
        }
    }
    ui.upcast()
}

static GLOSSARY_HTML: LazyLock<String> = LazyLock::new(|| {
    // TODO: is there a way to fetch this?
    const ADWAITA_DARK_FG_COLOR: &str = "#ffffff";

    let css = include_str!("glossary.css").replace("var(--t-dark-fg)", ADWAITA_DARK_FG_COLOR);
    format!("<style>{css}</style>")
});

fn glossary_webview(contents: &[structured::Content]) -> gtk::Widget {
    // why are we wrapping the webview in a container?
    // it's a surprise tool for later ;)
    let container = gtk::Box::new(gtk::Orientation::Vertical, 0);

    let view = webkit::WebView::new();
    container.append(&view);

    view.set_hexpand(true);
    view.set_vexpand(true);
    // avoid errors about allocating GBM buffer of size WIDTHx0
    // we'll resize the view once we have an actual height
    // also, if we can't allocate a buffer now, it will be empty forever
    view.set_height_request(1);
    view.set_background_color(&gdk::RGBA::new(0.0, 0.0, 0.0, 0.0));

    // generate the glossary HTML and start loading it
    let mut html = GLOSSARY_HTML.clone();
    _ = write!(
        &mut html,
        r#"<ul class="gloss-list" data-count="{}">"#,
        contents.len()
    );
    for content in contents {
        _ = write!(&mut html, "<li>");
        _ = yomitan::write_html(&mut html, content);
        _ = write!(&mut html, "</li>");
    }
    _ = write!(&mut html, "</ul>");
    view.load_html(&html, None);

    // when attempting to navigate to a URL, open in the user's browser instead
    view.connect_decide_policy(|_, decision, decision_type| {
        if decision_type != webkit::PolicyDecisionType::NavigationAction {
            return false;
        }
        let Some(decision) = decision.downcast_ref::<webkit::NavigationPolicyDecision>() else {
            return false;
        };
        let Some(mut nav_action) = decision.navigation_action() else {
            return false;
        };
        if !nav_action.is_user_gesture() {
            return false;
        }

        if let Some(request) = nav_action.request() {
            if let Some(uri) = request.uri() {
                open_uri(&uri);
            }
        }

        decision.ignore();
        true // inhibit request
    });

    // resize the view height to the content *when the webpage loads*
    view.connect_load_changed(move |view, _| resize_view(view.clone()));

    // resize the view height to the content *when the container changes size*
    // we use a DrawingArea to detect when the container size changes,
    // by hooking into `draw_func`. it's pretty stupid, I know.
    let height_change_proxy = gtk::DrawingArea::builder()
        .hexpand(true)
        .vexpand(false)
        .build();
    container.append(&height_change_proxy);
    height_change_proxy.set_draw_func(move |_, _, _, _| {
        resize_view(view.clone());
    });

    container.upcast()
}

fn resize_view(view: webkit::WebView) {
    glib::timeout_add_local_once(Duration::from_millis(100), move || {
        view.evaluate_javascript(
            // get the natural height of the content
            // `document.body.scrollHeight` and friends will stay tall
            // even if the natural height is reduced, so we need to do it like this
            "
[...document.body.children].reduce(
    (h, el) => Math.max(h, el.getBoundingClientRect().bottom),
    0
)",
            None,
            None,
            None::<&gio::Cancellable>,
            {
                let view = view.clone();
                move |result| {
                    if let Ok(value) = result {
                        let height = value.to_int32();
                        view.set_height_request(height.max(1));
                    };
                }
            },
        );
    });
}

fn open_uri(uri: &str) {
    if let Some(uri) = uri.strip_prefix('?') {
        if let Some((_, query)) =
            form_urlencoded::parse(uri.as_bytes()).find(|(key, _)| key == "query")
        {
            info!("Looking up {query:?}");
            warn!("TODO: unimplemented");
        } else {
            warn!("Attempted to open {uri:?} which does not contain `query`");
        }
    } else {
        info!("Opening {uri:?}");
        if let Err(err) = open::that(uri) {
            warn!(
                "Failed to open link to {uri:?}: {:?}",
                anyhow::Error::new(err)
            );
        }
    }
}
