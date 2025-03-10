use gtk::prelude::*;
use std::fmt::Write as _;
use wordbase::format::yomitan;

use crate::{AddToTermInfo, GlossaryInfo, RecordContext, glossary, ui};

impl AddToTermInfo for yomitan::Glossary {
    fn add_to_term_info(mut self, cx: RecordContext) {
        let mut glossary_info = GlossaryInfo::default();

        self.tags
            .sort_by(|tag_a, tag_b| tag_a.order.cmp(&tag_b.order));

        glossary_info.tags.extend(self.tags.into_iter().map(|tag| {
            let ui = ui::GlossaryTag::new();
            ui.set_label(&tag.name);
            ui.set_tooltip_text(Some(&tag.description));
            if let Some(css_class) = css_class_of(&tag.category) {
                ui.add_css_class(css_class);
            }
            ui.upcast()
        }));

        // glossary_info.content.push(glossary::html(|mut s| {
        //     _ = write!(
        //         &mut s,
        //         r#"<ul class="gloss-list" data-count="{}">"#,
        //         self.content.len()
        //     );
        //     for content in self.content {
        //         _ = write!(&mut s, "<li>");
        //         _ = yomitan::render_to_html(&mut s, &content);
        //         _ = write!(&mut s, "</li>");
        //     }
        //     _ = write!(s, "</ul>");
        // }));

        cx.glossary_group.push(glossary_info);
    }
}

// https://github.com/yomidevs/yomitan/blob/48f1d012ad5045319d4e492dfbefa39da92817b2/ext/css/display.css#L136-L149
fn css_class_of(category: &str) -> Option<&'static str> {
    match category {
        "name" => Some("name"),
        "expression" => Some("expression"),
        "popular" => Some("popular"),
        "frequent" => Some("frequent"),
        "archaism" => Some("archaism"),
        "dictionary" => Some("dictionary"),
        "frequency" => Some("frequency"),
        "partOfSpeech" => Some("part-of-speech"),
        "search" => Some("search"),
        "pronunciation-dictionary" => Some("pronunciation-dictionary"),
        _ => None,
    }
}
