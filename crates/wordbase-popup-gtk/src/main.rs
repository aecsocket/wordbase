#![allow(
    clippy::wildcard_imports,
    reason = "in `mod imp`s, we often use `super::*`"
)]
#![allow(
    clippy::new_without_default,
    reason = "`gtk` types do not follow this pattern, so neither do we"
)]

extern crate gtk4 as gtk;
extern crate libadwaita as adw;
extern crate webkit6 as webkit;

mod ui;

use adw::prelude::*;
use gtk::gdk;
use wordbase::dict::{
    ExpressionEntry, Frequency, FrequencySet, Glossary, GlossarySet, Pitch, PitchSet, Reading,
};

fn main() {
    let app = adw::Application::builder()
        .application_id("com.github.aecsocket.WordbasePopup")
        .build();

    app.connect_startup(|_| {
        let css_provider = gtk::CssProvider::new();
        css_provider.load_from_string(include_str!("style.css"));

        gtk::style_context_add_provider_for_display(
            &gdk::Display::default().expect("failed to get display"),
            &css_provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    });

    app.connect_activate(|app| {
        let dictionary = ui::Dictionary::from(&entries());
        let dictionary_popup = ui::DictionaryPopup::new(&dictionary);

        adw::ApplicationWindow::builder()
            .application(app)
            .title("Dictionary")
            .content(&dictionary_popup)
            .build()
            .present();
    });

    app.run();
}

#[allow(clippy::too_many_lines)]
fn entries() -> Vec<ExpressionEntry> {
    vec![
        ExpressionEntry {
            reading: Reading::from_pairs([("協", "きょう"), ("力", "りょく")]),
            frequency_sets: vec![
                FrequencySet {
                    dictionary: "JPDB".into(),
                    frequencies: vec![
                        Frequency {
                            value: 954,
                            display_value: None,
                        },
                        Frequency {
                            value: 131_342,
                            display_value: Some("131342㋕".into()),
                        },
                    ],
                },
                FrequencySet {
                    dictionary: "VN Freq".into(),
                    frequencies: vec![Frequency {
                        value: 948,
                        display_value: None,
                    }],
                },
                FrequencySet {
                    dictionary: "Novels".into(),
                    frequencies: vec![Frequency {
                        value: 1377,
                        display_value: None,
                    }],
                },
                FrequencySet {
                    dictionary: "Anime & J-drama".into(),
                    frequencies: vec![Frequency {
                        value: 1042,
                        display_value: None,
                    }],
                },
                FrequencySet {
                    dictionary: "Youtube".into(),
                    frequencies: vec![Frequency {
                        value: 722,
                        display_value: None,
                    }],
                },
                FrequencySet {
                    dictionary: "Wikipedia".into(),
                    frequencies: vec![Frequency {
                        value: 705,
                        display_value: None,
                    }],
                },
                FrequencySet {
                    dictionary: "BCCWJ".into(),
                    frequencies: vec![
                        Frequency {
                            value: 597,
                            display_value: None,
                        },
                        Frequency {
                            value: 1395,
                            display_value: None,
                        },
                    ],
                },
                FrequencySet {
                    dictionary: "CC100".into(),
                    frequencies: vec![Frequency {
                        value: 741,
                        display_value: None,
                    }],
                },
                FrequencySet {
                    dictionary: "Innocent Ranked".into(),
                    frequencies: vec![Frequency {
                        value: 2343,
                        display_value: None,
                    }],
                },
                FrequencySet {
                    dictionary: "Narou Freq".into(),
                    frequencies: vec![Frequency {
                        value: 845,
                        display_value: None,
                    }],
                },
            ],
            pitch_sets: vec![PitchSet {
                dictionary: "NHK".into(),
                pitches: vec![Pitch { position: 1 }],
            }],
            glossary_sets: vec![
                GlossarySet {
                    dictionary: "Jitendex [2025-02-11]".into(),
                    glossaries: vec![Glossary {
                        todo: "TODO".into(),
                    }],
                },
                GlossarySet {
                    dictionary: "三省堂国語辞典　第八版".into(),
                    glossaries: vec![Glossary {
                        todo: "TODO".into(),
                    }],
                },
                GlossarySet {
                    dictionary: "明鏡国語辞典　第二版".into(),
                    glossaries: vec![Glossary {
                        todo: "TODO".into(),
                    }],
                },
                GlossarySet {
                    dictionary: "デジタル大辞泉".into(),
                    glossaries: vec![Glossary {
                        todo: "TODO".into(),
                    }],
                },
                GlossarySet {
                    dictionary: "PixivLight [2023-11-24]".into(),
                    glossaries: vec![Glossary {
                        todo: "TODO".into(),
                    }],
                },
            ],
        },
        ExpressionEntry {
            reading: Reading::from_no_pairs("協", ""),
            frequency_sets: vec![
                FrequencySet {
                    dictionary: "Novels".into(),
                    frequencies: vec![Frequency {
                        value: 29289,
                        display_value: None,
                    }],
                },
                FrequencySet {
                    dictionary: "Anime & J-drama".into(),
                    frequencies: vec![Frequency {
                        value: 26197,
                        display_value: None,
                    }],
                },
                FrequencySet {
                    dictionary: "Youtube".into(),
                    frequencies: vec![Frequency {
                        value: 23714,
                        display_value: None,
                    }],
                },
                FrequencySet {
                    dictionary: "Wikipedia".into(),
                    frequencies: vec![Frequency {
                        value: 6162,
                        display_value: None,
                    }],
                },
                FrequencySet {
                    dictionary: "Innocent Ranked".into(),
                    frequencies: vec![Frequency {
                        value: 18957,
                        display_value: None,
                    }],
                },
            ],
            pitch_sets: vec![],
            glossary_sets: vec![GlossarySet {
                dictionary: "JMnedict [2025-02-18]".into(),
                glossaries: vec![],
            }],
        },
    ]
}
