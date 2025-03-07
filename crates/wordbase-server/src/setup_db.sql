CREATE TABLE IF NOT EXISTS dictionary (
    id          INTEGER     PRIMARY KEY AUTOINCREMENT,
    name        TEXT        NOT NULL,
    version     TEXT        NOT NULL,
    position    INTEGER     NOT NULL UNIQUE,
    enabled     BOOLEAN     NOT NULL DEFAULT TRUE
);

CREATE TABLE IF NOT EXISTS record (
    source      INTEGER NOT NULL REFERENCES dictionary(id) ON DELETE CASCADE,
    headword    TEXT    NOT NULL,
    reading     TEXT,
    -- keep this up to date with `record.rs`
    kind        INTEGER NOT NULL CHECK (kind IN (1, 2, 3)),
    data        BLOB    NOT NULL
);
CREATE INDEX IF NOT EXISTS record_headword ON record(headword);
CREATE INDEX IF NOT EXISTS record_reading  ON record(reading);
CREATE INDEX IF NOT EXISTS record_term     ON record(headword, reading);
