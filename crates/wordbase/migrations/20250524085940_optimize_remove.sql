CREATE INDEX profile_enabled_dictionary_profile ON profile_enabled_dictionary(profile);

DROP INDEX record_headword;
DROP INDEX record_reading;

-- improves removal performance, but if we're not careful,
-- SQLite will use these indexes for queries as well.
-- that will be VERY slow, since there may be 100,000s of entries
-- for a single `source`
CREATE INDEX record_source ON record(source);
CREATE INDEX frequency_source ON frequency(source);

-- ensure that SQLite uses proper word indexes for lookups
-- use 2 separate indexes since we search first by headword, then by reading,
-- done in 2 separate queries; we never search by both at the same time
CREATE INDEX record_query_headword ON record(headword, source);
CREATE INDEX record_query_reading ON record(reading, source);

-- we always search where `source` is the current sorting dict,
-- so `source` is first
-- we only search `frequency` by headword AND reading, never either,
-- so we include 1 (headword, reading) index instead of 2 separate ones
CREATE INDEX frequency_query ON frequency(source, headword, reading);
