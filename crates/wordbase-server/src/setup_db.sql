CREATE TABLE IF NOT EXISTS dictionary (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name        TEXT    NOT NULL,
    version     TEXT    NOT NULL,
    position    INTEGER NOT NULL UNIQUE,
    enabled     BOOLEAN NOT NULL DEFAULT TRUE
);

CREATE TABLE IF NOT EXISTS term (
    source      INTEGER NOT NULL REFERENCES dictionary(id) ON DELETE CASCADE,
    headword    TEXT    NOT NULL,
    reading     TEXT,
    kind        INTEGER NOT NULL,
    data        BLOB    NOT NULL
);
CREATE INDEX IF NOT EXISTS term_headword ON term(headword);
CREATE INDEX IF NOT EXISTS term_reading  ON term(reading);
CREATE INDEX IF NOT EXISTS term_term     ON term(headword, reading);
