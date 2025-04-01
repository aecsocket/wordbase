CREATE TABLE IF NOT EXISTS dictionary (
    id          INTEGER PRIMARY KEY,
    meta        TEXT    NOT NULL CHECK (json_valid(meta)),
    position    INTEGER NOT NULL
);

--

CREATE TABLE IF NOT EXISTS profile (
    id                  INTEGER PRIMARY KEY,
    meta                TEXT    NOT NULL CHECK (json_valid(meta)),
    sorting_dictionary  INTEGER REFERENCES dictionary(id)
);

INSERT INTO profile (meta)
SELECT '{}'
WHERE NOT EXISTS (SELECT 1 FROM profile);

CREATE TRIGGER IF NOT EXISTS assert_at_least_one_profile
BEFORE DELETE ON profile
BEGIN
    SELECT CASE
        WHEN (SELECT COUNT(*) FROM profile) <= 1
        THEN RAISE(ABORT, 'cannot delete last profile')
    END;
END;

CREATE TRIGGER IF NOT EXISTS reset_profile_sorting_dictionary
AFTER DELETE ON dictionary
BEGIN
    UPDATE profile
    SET sorting_dictionary = NULL
    WHERE sorting_dictionary = OLD.id;
END;

--

CREATE TABLE IF NOT EXISTS profile_enabled_dictionary (
    profile     INTEGER NOT NULL REFERENCES profile(id)     ON DELETE CASCADE,
    dictionary  INTEGER NOT NULL REFERENCES dictionary(id)  ON DELETE CASCADE,
    UNIQUE      (profile, dictionary)
);
CREATE INDEX IF NOT EXISTS profile_enabled_dictionary_idx ON profile_enabled_dictionary(dictionary);

--

CREATE TABLE IF NOT EXISTS config (
    id                      INTEGER PRIMARY KEY CHECK (id = 1),
    max_db_connections      INTEGER NOT NULL DEFAULT 8,
    max_concurrent_imports  INTEGER NOT NULL DEFAULT 4,
    current_profile         INTEGER NOT NULL DEFAULT 1 REFERENCES profile(id),
    texthooker_url          TEXT    NOT NULL DEFAULT 'ws://127.0.0.1:9001',
    ankiconnect_url         TEXT    NOT NULL DEFAULT 'http://127.0.0.1:8765',
    ankiconnect_api_key     TEXT    NOT NULL DEFAULT ''
);

INSERT OR IGNORE INTO config DEFAULT VALUES;

CREATE TRIGGER IF NOT EXISTS prevent_config_delete
BEFORE DELETE ON config
BEGIN
    SELECT RAISE(ABORT, 'cannot delete config row');
END;

CREATE TRIGGER IF NOT EXISTS reset_current_profile
AFTER DELETE ON profile
WHEN OLD.id = (SELECT current_profile FROM config)
BEGIN
    UPDATE config SET current_profile = (SELECT MIN(id) FROM profile);
END;

--

CREATE TABLE IF NOT EXISTS record (
    id          INTEGER PRIMARY KEY,
    source      INTEGER NOT NULL REFERENCES dictionary(id) ON DELETE CASCADE,
    kind        INTEGER NOT NULL,
    data        BLOB    NOT NULL
);

--

CREATE TABLE IF NOT EXISTS term_record (
    source      INTEGER NOT NULL REFERENCES dictionary(id) ON DELETE CASCADE,
    record      INTEGER NOT NULL REFERENCES record(id) ON DELETE CASCADE,
    headword    TEXT,
    reading     TEXT,
    UNIQUE (record, headword, reading),
    CHECK (headword IS NOT NULL OR reading IS NOT NULL)
);
CREATE INDEX IF NOT EXISTS term_record_idx1 ON term_record(reading, record);
CREATE INDEX IF NOT EXISTS term_record_idx2 ON term_record(headword);

--

CREATE TABLE IF NOT EXISTS frequency (
    source      INTEGER NOT NULL REFERENCES dictionary(id) ON DELETE CASCADE,
    headword    TEXT,
    reading     TEXT,
    mode        INTEGER NOT NULL CHECK (mode IN (0, 1)),
    value       INTEGER NOT NULL,
    UNIQUE (source, headword, reading)
    CHECK (headword IS NOT NULL OR reading IS NOT NULL)
);
