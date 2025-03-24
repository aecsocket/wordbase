use sqlx::{Pool, Sqlite};

#[derive(Debug, Clone)]
pub struct Profiles {
    db: Pool<Sqlite>,
}

impl Profiles {
    pub(super) fn new(db: Pool<Sqlite>) -> Self {
        Self { db }
    }
}
