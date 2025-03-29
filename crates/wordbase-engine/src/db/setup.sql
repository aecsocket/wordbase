CREATE TABLE IF NOT EXISTS profile (
    id      INTEGER PRIMARY KEY AUTOINCREMENT,
    meta    TEXT    NOT NULL CHECK (json_valid(meta))
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

CREATE TABLE IF NOT EXISTS config (
    id                  INTEGER PRIMARY KEY CHECK (id = 1),
    current_profile     INTEGER NOT NULL DEFAULT 1 REFERENCES profile(id),
    texthooker_url      TEXT    NOT NULL DEFAULT 'ws://127.0.0.1:9001',
    ankiconnect_url     TEXT    NOT NULL DEFAULT 'http://127.0.0.1:8765',
    ankiconnect_api_key TEXT    NOT NULL DEFAULT ''
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

CREATE TABLE IF NOT EXISTS dictionary (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    position    INTEGER NOT NULL,
    meta        TEXT    NOT NULL CHECK (json_valid(meta))
);

CREATE TABLE IF NOT EXISTS profile_enabled_dictionary (
    profile     INTEGER NOT NULL REFERENCES profile(id)     ON DELETE CASCADE,
    dictionary  INTEGER NOT NULL REFERENCES dictionary(id)  ON DELETE CASCADE,
    UNIQUE      (profile, dictionary)
);
CREATE INDEX IF NOT EXISTS index_profile_enabled_dictionary ON profile_enabled_dictionary(profile, dictionary);

CREATE TABLE IF NOT EXISTS term (
    source      INTEGER NOT NULL REFERENCES dictionary(id) ON DELETE CASCADE,
    headword    TEXT,
    reading     TEXT,
    kind        INTEGER NOT NULL,
    data        BLOB    NOT NULL,
    CHECK       (headword IS NOT NULL OR reading IS NOT NULL)
);
CREATE INDEX IF NOT EXISTS term_source   ON term(source);
CREATE INDEX IF NOT EXISTS term_headword ON term(headword);
CREATE INDEX IF NOT EXISTS term_reading  ON term(reading);
CREATE INDEX IF NOT EXISTS term_kind     ON term(kind);
