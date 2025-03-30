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
    text    TEXT    NOT NULL CHECK (text <> ''),
    kind    INTEGER NOT NULL CHECK (kind IN (0, 1)),
    record  INTEGER NOT NULL REFERENCES record(id),
    UNIQUE  (text, kind, record)
);
CREATE INDEX IF NOT EXISTS index_term_text ON term(text);

CREATE TABLE IF NOT EXISTS record (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    source      INTEGER NOT NULL REFERENCES dictionary(id) ON DELETE CASCADE,
    kind        INTEGER NOT NULL,
    data        BLOB    NOT NULL
);
CREATE INDEX IF NOT EXISTS record_source    ON record(source);
CREATE INDEX IF NOT EXISTS record_kind      ON record(kind);
CREATE TRIGGER IF NOT EXISTS delete_orphaned_terms
AFTER DELETE ON record
BEGIN
    DELETE FROM term
    WHERE record = OLD.id
    AND NOT EXISTS (
        SELECT 1 FROM record
        WHERE record.id = term.record
    );
END
