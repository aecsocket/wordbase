use {
    crate::{dictionaries::RecordEntry, storage::EngineStorage},
    eyre::{Context, Result, eyre},
    rayon::prelude::*,
    wordbase_core::deinflect::{Deinflection, Deinflector},
    wordbase_types::ProfileId,
};

pub struct Deinflectors {
    deinflectors: Vec<Box<dyn Deinflector>>,
}

impl Deinflectors {
    pub fn new(deinflectors: impl Into<Vec<Box<dyn Deinflector>>>) -> Self {
        Self {
            deinflectors: deinflectors.into(),
        }
    }

    pub fn deinflect<'t>(&self, sentence: &'t str, cursor: usize) -> Result<Vec<Deinflection<'t>>> {
        let mut deinflections = self
            .deinflectors
            .iter()
            .flat_map(
                |deinflector| match deinflector.deinflect(sentence, cursor) {
                    Ok(x) => x.into_iter().map(Ok).collect::<Vec<_>>(),
                    Err(err) => vec![
                        Err(err)
                            .wrap_err_with(|| eyre!("deinflector `{}` failed", deinflector.id())),
                    ],
                },
            )
            .collect::<Result<Vec<_>>>()?;
        // use a stable sort here because we want to preserve ordering
        // within each deinflector's result set
        deinflections.sort_by_key(|d| d.source_span.start);
        Ok(deinflections)
    }

    // sorting deferred to caller
    pub fn lookup(
        &self,
        storage: &EngineStorage,
        profile_id: ProfileId,
        sentence: &str,
        cursor: usize,
    ) -> Result<Vec<RecordEntry>> {
        self.deinflect(sentence, cursor)?
            .into_par_iter()
            .flat_map(
                |deinf| match storage.lookup_lemma(profile_id, &deinf.lemma) {
                    Ok(x) => x.into_iter().map(Ok).collect::<Vec<_>>(),
                    Err(err) => vec![
                        Err(err)
                            .wrap_err_with(|| eyre!("failed to look up lemma `{}`", deinf.lemma)),
                    ],
                },
            )
            .collect::<Result<Vec<_>>>()
    }
}
