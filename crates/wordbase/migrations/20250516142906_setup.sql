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
CREATE INDEX profile_enabled_dictionary_profile    ON profile_enabled_dictionary(profile);
CREATE INDEX profile_enabled_dictionary_dictionary ON profile_enabled_dictionary(dictionary);

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
    headword    TEXT,
    reading     TEXT,
    record      INTEGER NOT NULL REFERENCES record(id) ON DELETE CASCADE,
    UNIQUE (source, headword, reading, record)
    CHECK (headword IS NOT NULL OR reading IS NOT NULL)
);

CREATE TABLE frequency (
    source      INTEGER NOT NULL REFERENCES dictionary(id) ON DELETE CASCADE,
    headword    TEXT,
    reading     TEXT,
    -- 0: rank
    -- 1: occurrence
    mode        INTEGER NOT NULL CHECK (mode IN (0, 1)),
    value       INTEGER NOT NULL,
    UNIQUE (source, headword, reading)
    CHECK (headword IS NOT NULL OR reading IS NOT NULL)
);

-- improves removal performance, but if we're not careful,
-- SQLite will use these indexes for queries as well.
-- that will be VERY slow, since there may be 100,000s of entries
-- for a single `source`
CREATE INDEX record_source ON record(source);
CREATE INDEX term_record_source ON term_record(source);
CREATE INDEX frequency_source ON frequency(source);

-- ensure that SQLite uses proper word indexes for lookups
-- use 2 separate indexes since we search first by headword, then by reading,
-- done in 2 separate queries; we never search by both at the same time
CREATE INDEX term_record_query_headword ON term_record(headword, source);
CREATE INDEX term_record_query_reading ON term_record(reading, source);

-- we always search where `source` is the current sorting dict,
-- so `source` is first
-- we only search `frequency` by headword AND reading, never either,
-- so we include 1 (headword, reading) index instead of 2 separate ones
CREATE INDEX frequency_query ON frequency(source, headword, reading);
