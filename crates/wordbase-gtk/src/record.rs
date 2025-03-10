use gtk::prelude::*;
use wordbase::record::Frequency;

use crate::{AddToTermInfo, RecordContext, ui};

impl AddToTermInfo for Frequency {
    fn add_to_term_info(self, cx: RecordContext) {
        let tag = ui::FrequencyTag::new();
        cx.meta_info.frequencies.push(tag.clone().upcast());

        tag.source().set_text(cx.source_name);
        tag.frequency()
            .set_text(&self.display.unwrap_or_else(|| format!("{}", self.rank)));
    }
}
