CREATE TABLE dictionary (
    id          INTEGER NOT NULL PRIMARY KEY,
    meta        TEXT    NOT NULL CHECK (json_valid(meta)),
    position    INTEGER NOT NULL
);

CREATE TABLE profile (
    id                  INTEGER NOT NULL PRIMARY KEY,
    name                TEXT,
    sorting_dictionary  INTEGER REFERENCES dictionary(id),
    font_family         TEXT,
    anki_deck           TEXT,
    anki_note_type      TEXT
);
INSERT INTO profile (name) SELECT NULL;
CREATE TRIGGER assert_at_least_one_profile
BEFORE DELETE ON profile
BEGIN
    SELECT CASE
        WHEN (SELECT COUNT(*) FROM profile) <= 1
        THEN RAISE(ABORT, 'cannot delete last profile')
    END;
END;
CREATE TRIGGER reset_profile_sorting_dictionary
AFTER DELETE ON dictionary
BEGIN
    UPDATE profile
    SET sorting_dictionary = NULL
    WHERE sorting_dictionary = OLD.id;
END;

CREATE TABLE profile_enabled_dictionary (
    profile     INTEGER NOT NULL REFERENCES profile(id)     ON DELETE CASCADE,
    dictionary  INTEGER NOT NULL REFERENCES dictionary(id)  ON DELETE CASCADE,
    UNIQUE      (profile, dictionary)
);
-- CREATE INDEX profile_enabled_dictionary_idx ON profile_enabled_dictionary(dictionary);

CREATE TABLE config (
    id                      INTEGER NOT NULL PRIMARY KEY CHECK (id = 1),
    ankiconnect_url         TEXT    NOT NULL DEFAULT 'http://127.0.0.1:8765',
    ankiconnect_api_key     TEXT    NOT NULL DEFAULT '',
    texthooker_url          TEXT    NOT NULL DEFAULT 'ws://127.0.0.1:9001'
);
INSERT INTO config DEFAULT VALUES;
CREATE TRIGGER prevent_config_delete
BEFORE DELETE ON config
BEGIN
    SELECT RAISE(ABORT, 'cannot delete config row');
END;

CREATE TABLE record (
    id          INTEGER NOT NULL PRIMARY KEY,
    source      INTEGER NOT NULL REFERENCES dictionary(id) ON DELETE CASCADE,
    kind        INTEGER NOT NULL,
    data        BLOB    NOT NULL
);

CREATE TABLE term_record (
    source      INTEGER NOT NULL REFERENCES dictionary(id) ON DELETE CASCADE,
    record      INTEGER NOT NULL REFERENCES record(id) ON DELETE CASCADE,
    headword    TEXT,
    reading     TEXT,
    UNIQUE (record, headword, reading),
    CHECK (headword IS NOT NULL OR reading IS NOT NULL)
);
-- CREATE INDEX term_record_idx1 ON term_record(reading, record);
-- CREATE INDEX term_record_idx2 ON term_record(headword);

CREATE TABLE frequency (
    source      INTEGER NOT NULL REFERENCES dictionary(id) ON DELETE CASCADE,
    headword    TEXT,
    reading     TEXT,
    mode        INTEGER NOT NULL CHECK (mode IN (0, 1)),
    value       INTEGER NOT NULL,
    UNIQUE (source, headword, reading)
    CHECK (headword IS NOT NULL OR reading IS NOT NULL)
);
