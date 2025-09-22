CREATE TABLE profile (
    id                 BLOB    NOT NULL PRIMARY KEY,
    created_at         INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    name               TEXT    NOT NULL,
    sorting_dictionary BLOB
);

CREATE TABLE profile_dictionary (
    profile    BLOB NOT NULL REFERENCES profile(id) ON DELETE CASCADE,
    dictionary BLOB NOT NULL,
    UNIQUE (profile, dictionary)
);
CREATE TRIGGER prevent_last_profile_delete
BEFORE DELETE ON profile
FOR EACH ROW
WHEN (SELECT COUNT(*) FROM profile) = 1
BEGIN
    SELECT RAISE(ABORT, 'cannot delete last profile');
END;
