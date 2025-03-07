CREATE TABLE IF NOT EXISTS dictionaries (
    id          INTEGER     PRIMARY KEY AUTOINCREMENT,
    name        TEXT        NOT NULL,
    version     TEXT        NOT NULL,
    position    INTEGER     NOT NULL UNIQUE,
    enabled     BOOLEAN     NOT NULL DEFAULT TRUE
);

CREATE TABLE IF NOT EXISTS terms (
    source      INTEGER NOT NULL REFERENCES dictionaries(id) ON DELETE CASCADE,
    headword    TEXT    NOT NULL,
    reading     TEXT,
    -- keep this up to date with `db.rs`
    -- 1: glossary
    -- 2: frequency
    -- 3: jp_pitch
    data_kind   INTEGER NOT NULL CHECK (data_kind IN (1, 2, 3)),
    data        BLOB    NOT NULL
);
CREATE INDEX IF NOT EXISTS terms_headword ON terms(headword);
CREATE INDEX IF NOT EXISTS terms_reading  ON terms(reading);
CREATE INDEX IF NOT EXISTS terms_term     ON terms(headword, reading);
