Generate Japanese furigana for headword/reading pairs via JmdictFurigana

[![crates.io](https://img.shields.io/crates/v/jmdict-furigana.svg)](https://crates.io/crates/jmdict-furigana)
[![docs.rs](https://img.shields.io/docsrs/jmdict-furigana)](https://docs.rs/jmdict-furigana)

[JmdictFurigana] is a project which links Japanese kanji and kana readings with the right kana portions for each kanji in a dictionary word. This mapping lets you segment a kanji+kana reading pair into a list of kanji/kana pairs correctly. `jmdict-furigana` provides access to this mapping from Rust, by bundling the released JmdictFurigana archive and parsing it.

The current bundled archive is [2.3.1+2024-11-25](https://github.com/Doublevil/JmdictFurigana/releases/tag/2.3.1%2B2024-11-25)

# Features

- Readings for individual kanji are mapped correctly
  - `面白い[おもしろい]` becomes `面[おも]白[しろ]い`
  - `関係無い[かんけいない]` becomes `関[かん]係[けい]無[な]い`
- Special readings are mapped correctly
  - `大人[おとな]` becomes `大人[おとな]`

If a headword/reading pair is missing from the dictionary, `None` is returned. In this case, you either have no furigana reading available, or you will need to use some kind of fallback method like [`wordbase-api`].

```rust
async fn main() {
    // make sure to call `init` first to parse and load the dictionary
    // otherwise `get` calls will panic
    jmdict_furigana::init().await;

    let segments: &[(&str, &str)] = jmdict_furigana::get("関係無い", "かんけいない").unwrap();

    assert_eq!(segments, [("関", "かん"), ("係", "けい"), ("無", "な"), ("い", "")]);
}
```

# License

The source code is dual-licensed under MIT or Apache-2.0.

The JmdictFurigana archive is licensed under Creative Commons Attribution-ShareAlike Licence, same as JmdictFurigana and JMDict itself.

[JmdictFurigana]: https://github.com/Doublevil/JmdictFurigana?tab=readme-ov-file
[`wordbase-api`]: https://docs.rs/wordbase-api
