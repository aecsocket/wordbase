use anyhow::Result;
use wordbase::{Profile, RecordKind};
use wordbase_engine::Engine;

pub fn deinflect(engine: &Engine, text: &str) {
    for lemma in engine.deinflect(text) {
        let scan_len = lemma.scan_len;
        let text_part = text.get(..scan_len).map_or_else(
            || format!("(invalid scan len {scan_len})"),
            ToOwned::to_owned,
        );
        let lemma = lemma.lemma;
        println!("{text_part:?} -> {:?}", &*lemma);
    }
}

pub async fn lookup(engine: &Engine, profile: &Profile, text: &str) -> Result<()> {
    for result in engine.lookup(profile.id, text, 0, RecordKind::ALL).await? {
        println!("{result:#?}");
    }
    Ok(())
}

pub async fn lookup_lemma(engine: &Engine, profile: &Profile, lemma: &str) -> Result<()> {
    for result in engine
        .lookup_lemma(profile.id, &lemma, RecordKind::ALL)
        .await?
    {
        println!("{result:#?}");
    }
    Ok(())
}
