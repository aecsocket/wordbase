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
