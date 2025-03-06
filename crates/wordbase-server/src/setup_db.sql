CREATE TABLE dictionaries (
    id          INTEGER     PRIMARY KEY AUTOINCREMENT,
    title       TEXT        NOT NULL,
    revision    TEXT        NOT NULL,
    enabled     BOOLEAN     NOT NULL DEFAULT TRUE
);

CREATE TABLE terms (
    source      INTEGER NOT NULL REFERENCES dictionaries(id) ON DELETE CASCADE,
    expression  TEXT    NOT NULL,
    reading     TEXT,
    -- keep this up to date with `db.rs`
    -- 1: glossary
    -- 2: frequency
    -- 3: pitch
    data_kind   INTEGER NOT NULL CHECK (data_kind IN (1, 2, 3)),
    data        BLOB    NOT NULL
);
CREATE INDEX terms_expression    ON terms(expression);
CREATE INDEX terms_reading       ON terms(reading);
