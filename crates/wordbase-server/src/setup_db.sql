CREATE TABLE dictionaries (
    id          INTEGER     PRIMARY KEY AUTOINCREMENT,
    title       TEXT        NOT NULL,
    revision    TEXT        NOT NULL
);

CREATE TABLE terms (
    dictionary  INTEGER REFERENCES dictionaries(id) ON DELETE CASCADE,
    expression  TEXT NOT NULL,
    reading     TEXT NOT NULL,
    data        BLOB
);
CREATE INDEX terms_expression    ON terms(expression);
CREATE INDEX terms_reading       ON terms(reading);

CREATE TABLE term_meta (
    dictionary  INTEGER REFERENCES dictionary(id),
    expression  TEXT NOT NULL,
    data        BLOB
);
CREATE INDEX term_meta_expression ON term_meta(expression);
