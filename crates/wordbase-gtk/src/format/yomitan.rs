use wordbase::{Term, format::yomitan};

use crate::{AddToTermInfo, GlossaryInfo, MetaInfo, RecordContext};

impl AddToTermInfo for yomitan::Glossary {
    fn add_to_term_info(self, cx: RecordContext) {}
}
