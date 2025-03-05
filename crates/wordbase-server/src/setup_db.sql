CREATE TABLE dictionaries (
    id          INTEGER     PRIMARY KEY AUTOINCREMENT,
    title       TEXT        NOT NULL,
    revision    TEXT        NOT NULL
);

CREATE TABLE readings (
    source      INTEGER NOT NULL REFERENCES dictionaries(id) ON DELETE CASCADE,
    expression  TEXT    NOT NULL,
    reading     TEXT    NOT NULL,
    UNIQUE      (source, expression, reading)
);
CREATE INDEX readings_expression    ON readings(expression);
CREATE INDEX readings_reading       ON readings(reading);

CREATE TABLE frequencies (
    source          INTEGER NOT NULL REFERENCES dictionaries(id) ON DELETE CASCADE,
    expression      TEXT    NOT NULL,
    reading         TEXT    NOT NULL,
    data            BLOB    NOT NULL
);
CREATE INDEX frequencies_expression ON frequencies(expression);
CREATE INDEX frequencies_reading    ON frequencies(reading);

CREATE TABLE pitches (
    source          INTEGER NOT NULL REFERENCES dictionaries(id) ON DELETE CASCADE,
    expression      TEXT    NOT NULL,
    reading         TEXT    NOT NULL,
    data            BLOB    NOT NULL
);
CREATE INDEX pitches_expression ON pitches(expression);
CREATE INDEX pitches_reading    ON pitches(reading);

CREATE TABLE glossaries (
    source          INTEGER NOT NULL REFERENCES dictionaries(id) ON DELETE CASCADE,
    expression      TEXT    NOT NULL,
    reading         TEXT    NOT NULL,
    data            BLOB    NOT NULL
);
CREATE INDEX glossaries_expression ON glossaries(expression);
