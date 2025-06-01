use sqlx::Either;
use wordbase_api::dict;

pub fn furigana_parts<'a>(
    headword: &'a str,
    reading: &'a str,
) -> impl Iterator<Item = (&'a str, &'a str)> {
    jmdict_furigana::get(headword, reading)
        .map_or_else(
            || Either::Left(dict::jpn::furigana_parts(headword, reading).into_iter()),
            |entry| Either::Right(entry.iter().copied()),
        )
        .into_iter()
}
