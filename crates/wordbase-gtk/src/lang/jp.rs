use gtk::prelude::*;
use wordbase::lang::jp;

use crate::{AddToTermInfo, RecordContext};

impl AddToTermInfo for jp::Pitch {
    fn add_to_term_info(self, cx: RecordContext) {
        let Some(reading) = cx.term.reading.as_ref() else {
            return;
        };

        let ui = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        cx.meta_info.pitches.push(ui.clone().upcast());

        let downstep = usize::try_from(self.position).unwrap_or(usize::MAX);
        let mora = jp::morae(reading).collect::<Vec<_>>();

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
    }
}
