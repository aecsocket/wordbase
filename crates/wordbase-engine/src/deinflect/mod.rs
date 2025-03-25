mod lindera;

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Deinflectors {
    pub lindera: lindera::Deinflector,
}
