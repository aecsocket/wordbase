# notes - temp

## 12 Sep

Got redb + rkyv working. Some simple benchmarks:

- Import jitendex in 2 ways
- Make a `wordbase_rkyv.redb` with rkyv-encoded
- Make a `wordbase_rmp.redb` with messagepack-encoded
- Benchmark querying
  - `rkyv access`: reading from ReDB and accessing the `ArchivedRecord`
  - `rkyv deserialize`: reading the `ArchivedRecord`, deserializing to `Record`
  - `rmp deserialize`: reading from `rmp.redb` and deserializing via MessagePack to `Record`

Query perf:
```
INFO wordbase_cli: rkyv access: 776.637938ms
INFO wordbase_cli: rkyv deserialize: 2.338171616s
INFO wordbase_cli: rmp deserialize: 14.482389631s
```

Storage size:
```
⬢ [dev] ❯ la ~/.local/share/wordbase/
total 4.7G
-rw-r--r-- 1 boris boris 6.1G Sep 13 01:01 dictionary_rkyv.redb
-rw-r--r-- 1 boris boris 1.1G Sep 13 01:28 dictionary_rmp.redb
```

Ouch!

Record size:
- rkyv
```
INFO bank{path="term_bank_142.json"}: wordbase::import: record size = 12368
INFO bank{path="term_bank_142.json"}: wordbase::import: record size = 12368
INFO bank{path="term_bank_142.json"}: wordbase::import: record size = 4264
INFO bank{path="term_bank_142.json"}: wordbase::import: record size = 4776
INFO bank{path="term_bank_142.json"}: wordbase::import: record size = 3776
INFO bank{path="term_bank_142.json"}: wordbase::import: record size = 4888
INFO bank{path="term_bank_142.json"}: wordbase::import: record size = 3760
INFO bank{path="term_bank_142.json"}: wordbase::import: record size = 13688
INFO bank{path="term_bank_142.json"}: wordbase::import: record size = 4752
INFO bank{path="term_bank_142.json"}: wordbase::import: record size = 4752
INFO bank{path="term_bank_142.json"}: wordbase::import: record size = 4784
INFO bank{path="term_bank_142.json"}: wordbase::import: record size = 4280
INFO bank{path="term_bank_142.json"}: wordbase::import: record size = 3752
INFO bank{path="term_bank_142.json"}: wordbase::import: record size = 13688
INFO bank{path="term_bank_142.json"}: wordbase::import: record size = 9088
```
- rmp_serde
```
INFO bank{path="term_bank_68.json"}: wordbase::import: record size = 646
INFO bank{path="term_bank_68.json"}: wordbase::import: record size = 461
INFO bank{path="term_bank_68.json"}: wordbase::import: record size = 429
INFO bank{path="term_bank_68.json"}: wordbase::import: record size = 440
INFO bank{path="term_bank_68.json"}: wordbase::import: record size = 445
INFO bank{path="term_bank_68.json"}: wordbase::import: record size = 1764
INFO bank{path="term_bank_68.json"}: wordbase::import: record size = 432
INFO bank{path="term_bank_68.json"}: wordbase::import: record size = 433
INFO bank{path="term_bank_68.json"}: wordbase::import: record size = 438
INFO bank{path="term_bank_68.json"}: wordbase::import: record size = 428
INFO bank{path="term_bank_68.json"}: wordbase::import: record size = 447
INFO bank{path="term_bank_68.json"}: wordbase::import: record size = 414
INFO bank{path="term_bank_68.json"}: wordbase::import: record size = 459
INFO bank{path="term_bank_68.json"}: wordbase::import: record size = 435
INFO bank{path="term_bank_68.json"}: wordbase::import: record size = 464
INFO bank{path="term_bank_68.json"}: wordbase::import: record size = 452
INFO bank{path="term_bank_68.json"}: wordbase::import: record size = 455
INFO bank{path="term_bank_68.json"}: wordbase::import: record size = 610
INFO bank{path="term_bank_68.json"}: wordbase::import: record size = 243
INFO bank{path="term_bank_68.json"}: wordbase::import: record size = 1720
INFO bank{path="term_bank_68.json"}: wordbase::import: record size = 1720
INFO bank{path="term_bank_68.json"}: wordbase::import: record size = 419
```

Summary:
- `rkyv` is ~10x faster to deserialize, and ~20x faster to access
- `rmp` is ~6x smaller on disk

## 11 Sep

Been doing more research on database stuff. LMDB doesn't let me do multiple writes at once to the same Env but different DBs. (I mean LMDB supports it but not `heed`). So:
- `sled` - last update was 4 years ago, there's a rewrite `1.0.0-alpha` branch but that's alpha..
- `redb` - beta but fills a similar niche?

## 10 Sep

I want to redesign the data storage to support local-first and cross-device syncing. This will be important later when I add syncing to/from a remote, and keeping your multiple devices in sync.
- <https://automerge.org/docs/hello/>
- <https://www.inkandswitch.com/essay/local-first/>

Someone in the Rust discord mentioned making each dictionary its own SQLite DB and attaching them. Hmm..?

Probably not viable since querying many dictionaries would be O(n) and cross-database indices wouldn't work

LMDB?

I want to optimize for 🚀 blazing 🔥 fast 🚀 queries on a headword/reading, so maybe dictionaries are individual LMDB files and we do client-side sorting.

Our current lookup query is:
```sql
ORDER BY
    CASE
        -- prioritize results where both the headword and reading match the lemma
        -- e.g. if you typed あらゆる:
        -- - the first results would be for the kana あらゆる
        -- - then the kanji like 汎ゆる
        WHEN base.reading = $2 AND base.headword = $2 THEN 0
        -- then prioritize results where at least the reading or headword are an exact match
        -- e.g. in 念じる, usually 念ずる comes up first
        -- but this is obviously a different reading
        -- so we want to prioritize 念じる
        WHEN base.reading = $2 OR base.headword = $2 THEN 1
        -- all other results at the end
        ELSE 2
    END,
    -- user-specified dictionary sorting position always takes priority
    dictionary.position,
    -- put entries without an explicit frequency value last
    CASE
        WHEN profile_frequency.mode IS NULL THEN 1
        ELSE 0
    END,
    -- sort by profile-global frequency info
    CASE
        -- frequency rank
        WHEN profile_frequency.mode = 0 THEN  profile_frequency.value
        -- frequency occurrence
        WHEN profile_frequency.mode = 1 THEN -profile_frequency.value
        ELSE 0
    END,
    -- sort by source-specific frequency info
    CASE
        WHEN source_frequency.mode = 0 THEN  source_frequency.value
        WHEN source_frequency.mode = 1 THEN -source_frequency.value
        ELSE 0
    END
```

Could we replicate this with LMDB and client-side sorting?

## 26 Jun

I need a list of "user stories" or like "target scenarios", to focus on what I build next. Right now I can't decide on what to actually work on. Thoughts:
- reading wikipedia in chrome/firefox/brave
- reading a book in Moon Reader
- watching yt/netflix/local video
- playing a VN in an emulator (OCR)
- playing a VN with a texthooker

## 22 Jun

I really like Kotlin Compose, but I think I need to switch to something more universal to make porting to iOS easier in the future. Dioxus + some native Android elements like a dictionary activity?

Thoughts on architecture:
- `wordbase`
  - Core dictionary engine, platform-agnostic, uses SQLite
- `wordbase-app`
  - Uses Dioxus
  - Designed for web and mobile, but can theoretically run on desktop
  - IPC: platform-specific (Android intents, iOS share)
  - Features:
    - Dictionary lookup (main page)
    - Dictionary management
    - Conversation mode (see obsidian 2025-06-20)
    - Audio player (see obsidian 2025-06-20 and 21)
- `wordbase-gtk`
  - Uses Relm4 + GTK + Adwaita
  - Designed for desktop, and GNOME mobile I guess? lmao
  - IPC: HTTP/WS server on your desktop

## 18 Apr

Ok I've tried to get xdg desktop portals to work with this. I think it's fundamentally too limiting. Summary:
- if we use a Screenshot portal
  - it always makes a new file in the pictures folder
  - we can't select the window to screenshot by default
  - cumbersome for the user
- if we use a ScreenCast portal
  - We *could* actually, and take a screenshot of the pipewire node
  - But we can't get the window ID of the window that we're casting from the GNOME extension, which is super limiting
    - So the extension would have to guess what window you're casting, or the user would have to select it manually, which is really ass

## 17 Apr

Holy shit! How didn't I know about this!!

https://flatpak.github.io/xdg-desktop-portal/docs/window-identifiers.html
https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.impl.portal.Screenshot.html
https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.impl.portal.ScreenCast.html

New flow idea:
- when we receive a new texthooker sentence,
- request a screencast
- user selects the window to attach to
- we now have a way to screenshot that window (awesome)
- we get the `parent_window` identifier and forward that to our extension for positioning purposes

## 14 Apr

I think my goals are a bit conflicted here. I want *the app* to be as simple as possible and min-config (see the GNOME stuff), but I want *the platform* to be as flexible as possible. For this reason, I went with the "anti-schema" of record kinds, instead of trying to standardize a single glossary format or whatever. If I take this to its logical extreme, the platform should be able to support:
- any dictionary format (that's the point of the record kind stuff)
- texthooker aggregation
- AnkiConnect integration, and other flashcard apps potentially
- collecting statistics on words learned/learning

But despite all these features, the apps should stay as simple **to configure** as possible. I think that's the key here - I don't mind *more features*, but I do mind *more config*. It's fine if you want to add a feature to see what words you've looked up the most, but it *shouldn't require extra config*.

In summary: state is cheap, config is expensive. Convention over configuration.

This gives me a good guideline and goal to follow for the project.

What's the original reason I started this project? I got annoyed at how:
- my Memento and Yomitan dictionaries don't sync
  - by extension, how I have to configure 2 different tools whenever I set up a PC again (and a new browser)
- there are no good integrated sentence mining tools for Linux

I've ironed out a bunch of design issues, and AnkiConnect is like almost functioning. Most importantly, I've decided:
- `wordbase-engine` no longer tracks the current profile - this is left as a responsibility of the app
- I've figured out the scope of the different crates and the project as a whole - see paragraphs above.
- `wordbase` and `wordbase-engine` are solid foundations to build on, but `wordbase-desktop` is a lost cause. I've just been hacking on top of it instead of trying to clean anything up. I want to definitely rewrite this without Relm4 at some point.

- [ ] For some audios like NHK ones, we know the pitch position. We should move those audio buttons into the pitch reading pills.
- [ ] scanning ようやく gives 漸く as the top result. ffs, it should be the READING first! we have to prioritise it somehow. back to messing with the lookup query
- [ ] AI integration for understanding a sentence given context is insanely useful. This should 100% be a part of the normal workflow. (hmm..)
  - gemma3-1b seems to be useless for this. so is 4b
- [ ] ドアノブを引いたところで、俺は動きを止める。 - weird scanning fail with 引いたところで

## Test session 4 - 13 Apr

- I still haven't done the ankiconnect lmao. I've been procrastinating the stylesheet (but at least it's pretty now tho). This ACTUALLY has to be my priority now.
- The desktop app is a piece of shit and a mess
  - Where the fuck is all the state managed?!??!?!
  - I don't think Relm4 is the right fit for this app, since so much of the state is stored outside of the component model.
  - I want to rewrite this in raw GTK/Adwaita later, but for now I can keep hacking in features
- IMO the engine shouldn't keep track of a "current profile". Leave this up to the individual app. (But it should still keep track of profiles)
- If one of my goals is "keep it simple", then why did I add in texthooker support into the core app? This is a niche that only applies to Japanese visual novel language learners. I want to move this out to its own app later.
  - Move it out of `wordbase` and `wordbase-engine` into... somewhere else
  - Keep support for overlays in `wordbase-integration` - it's useful, and it means we don't need 2 separate GNOME extensions
  - Write texthooker listen logic directly in `wordbase-desktop` (and `wordbase-engine-cli`? ehh idk)
- [ ] Now that I have more dicts, lookups are starting to lag a bit. Currently, requesting a lookup "blocks" the async runtime. If we request a new lookup before the old one completes, it should stop the old lookup and no longer await it.
- [x] looking up 替える gives results for 変える first - wrong! I know why this happens tho. in the lookup, we sort by `reading matches AND headword matches`, then `reading matches OR headword matches`. this should be `hm AND rm`, `hm`, `rm`, everything else
  - fuck, this is wrong! if we looked up like "きびす" (踵), we'd get results for 踵<かかと> first. that's wrong
  - uuughhhh, maybe we detect if we have kana in the lemma and if so look up by reading first? and otherwise headword first?
  - can't reproduce anymore
- we should be able to spawn sub-popups from looking up in the popup. maybe? future goal.
- [ ] the audio should really be in the sticky header, not the meta tag flow box. When pitch accent readings are split onto multiple lines, it's a bit harder to read. They should be kept together. (Frequencies don't matter as much who cares)
- [x] "（ふぅ…騒がしかった…）" - deinflects as <騒がしかっ>た. could we make this <騒がしかった>?
- [x] there's a lot of Forvo audio records which have a headword but no reading e.g. 薄情. this makes the forvo audio appear as a separate entry below. I think we can unify this record with the main record by doing this:
  - if we have a record with a term which has a headword but no reading, it gets cloned into all the other "buckets" for the same headword
  - i.e. 見る, 観る would contain the same records for みる
  - this will probably suck if we do this to ALL record kinds. maybe only yomichan audio records?
- [ ] what would be really cool is, if you click on a sub-query lookup, it opens as a new navigation page in GTK with a new webview
  - recycle sub-webviews
  - you can swipe left to go back to the previous page seamlessly, like in epiphany
- [x] wrong furigana generation - 黄色い声 (きいろいこえ)
  - added test case - it's the same issue as before. I'll have to fix them together
- interesting dictionary quirk: ぶいぶいいわせる - we have records for ブイブイ言わす with reading ぶいぶいいわせる - this doesn't match! and even yomitan fails to generate furigana in this case.
  - [ ] its fallback is better than ours I think, we should use its strategy

## Test session 3 - 12 Apr

Now that I've done a bunch of generic bug fixing and improvements, I want to do more targeted high-impact improvements
- [ ] AnkiConnect!!!!!! (!!!)
- The first-hover popup experience MUST be perfect, since it's like 90% of the popup use case
  - [ ] it sometimes just doesn't show up. like, I *know* it scans, I can see the text highlight, but it doesn't appear on top for whatever reason. This has to work perfectly at all costs, even using hacks.
- [x] Terms like 此処等 are more common in kana as ここら, they should be displayed as such
  - How does Memento/Yomitan determine what becomes reading-only?
- [ ] When exiting the overlay, it presents for some reason? I think this is from a previous bugfix
- [x] "何かを念じるかのようだった。" - "念じる" has wrong order of lookup
- [x] "こんな図書室には似つかわしくないぐらい、専門的で高価そうな本ばかりだ。" - 高価 has wrong ordering. it's below 高い
- Future goal: can we get some way of highlighting known/unknown words? I'd like to use AnkiConnect for this tracking ideally, since that gives us the best estimate of how well the user knows a word. Then we can mark new words in a different color in the overlay or something.
- Future goal: I have a term like 履帯映像, I want to ask AI a question like "break down the kanji in this phrase". I should be able to do this integrated into the dictionary popup.

## Test session 2 - 11 Apr

- IPAex Gothic looks *really* nice
- [x] まじない -> deinflects as 呪い, and prioritises 呪い (のろい). can we make it prioritise まじない?
  - fixed with new deinflect algo
- [ ] my top priority HAS to be making anki notes from lookups
- [x] 頼りなさげな目を... - scans as <頼りな>さげな... we need extra lindera continuation rules for this case
  - <肩をおとし>て
  - <叩きつけ>ていた
  - <消えてたじゃない> -> <きえてた>じゃない
- [ ] つまらねぇ - lindera/unidic doesn't seem to be able to turn this into つまらない. do we hardcode some rules like ねぇ -> ない?
  - handwritten deinflector? 🤔
- [x] 関係ない furigana is wrong?
  - [ ] added a test case, but idk how to resolve it. failing test.
- [x] ともなると - in DOJG, the line breaks are done wrong. \n should be replaced with <br/>. let's do this in the renderer, not the importer.
- [x] popup should dynamically anchor itself to the topleft/topright/etc. shouldn't be up to the requester. e.g. if it's in the bottom 50% of the screen, anchor it to a top corner
  - improved positioning algorithm, similar to yomitan's (didn't copy any code tho)
- 大事 has a lot of pitch accents, will be good for testing PA rendering
- [ ] 当てられまくる授業だった - need better lookup for 当てられまくる. dicts?
- [x] I accidentally fullscreened the overlay. this should be impossible!
- [x] a button in the overlay to copy the sentence
- [x] a button in the overlay to go to the manager search field immediately, or an inline search? idk exactly

## Test session 1 - 10 Apr

- [ ] sometimes when hovering, the lookup is done, BUT the popup isn't focused maybe?
- [x] chinese fonts (force switch away from Inter?)
  - [x] allow switching overlay and dict font
  - [ ] set `lang=[bcp47 code]` based on current language (hardcoded to JP for now)
- [x] overlay opacity should update when you modify it
- [x] incorrect furigana
  - 聞き流す - ききながす
  - 言い争い - いいあらそい
  - 言い直す - いいなおす
- [x] why do some things get the wrong char length?
  - ショックでだろう -> should just be ショック
  - 日常だった -> should just be 日常
  - "共に" -> chars as 共, but should be 共に
- [x] add lindera tests for the above
- [ ] if clicking the mouse while sentence motion, it should NOT lookup
- [ ] this fails to lookup:
  - 居たたまれなくなって
    - because: root form is 居たたまれない
  - 仕えする
  - 向き合わせになる
- [x] click dragging on the overlay popup should let you drag it
- [ ] I really want a scrollback, but the current sentence goes to the bottom + there's enough padding at the bottom to push the scrollback up out of the way
- [x] dictionary popup settings button is literally invisible, need to add OSD class somehow
  - fixed by making the card background transparent
- [x] changing dictionaries from the popup window?
  - we don't need this, you have a button to open the manager anyway
- [ ] 停学処分 - we have lookups for:
  - (停学処分, ていがくしょぶん) -> jitendex
  - (停学処分, NULL) -> audio
  - can we merge the 2 somehow?
  - 体育会系 as well
  - won't fix
  - ok I lied about won't fix. I really do want to fix this somehow.
- [x] TONS of stack trace errors in journalctl. we need a way to only send them as error messages to the dbus client from the extension
- [x] イカす is improperly deinflected - lindera thinks it's 活かす/生かす
  - how about: when we deinflect e.g. イカす to 活かす, we ALSO generate a deinflection which maps to the same substring length as 活かす, BUT is a substring of the original query it self (イカす)?
  - [x] sorta fixed via a better deinflector + maintain deinflect ordering
- [ ] the "is ready" popup keeps coming up
- [ ] sometimes the dictionary IS spawned, but doesnt appear in front of the window. maybe `make_above` is not being applied?
  - yes, it's as I guessed. it's not being set as `make_above`. maybe related to below?
- [ ] or maybe it's to do with the window not found error from the extension. we should fix that

- overlay settings: font size, overlay opacity, lookup mode (hover, hold shift, hold ctrl, hold alt)
- good gnome integration - DONE
- Better stylesheet
- AnkiConnect
- Optional AI integration
  - Receives context of the looked up sentence (let's say 1 sentence before and after), and asks AI to generate an explanation
  - Can include extra metadata from the app e.g. video file name, browser tab title, so the AI has more context on what the user is doing
  - Local-first support via Ramalama

## Misc

TODO:
- local audio server - SORTA DONE
- ankiconnect

quick test problems:
- clicking off the overlay box, then it updates the sentence, and I start hovering back on - it doesn't immediately show popup box until I hover over a different word and back
- when the app goes full screen, the overlay should appear at the top
- dictionary popup should always appear above

audio server:
- i query たべなかった
- i get the term "食べる (たべる)"
- i get audio bytes as a record

FUTURE ROADMAP:

I don't like how `Term` is hardcoded to a headword and reading. It's totally possible to have multiple readings for the same headword, i.e. a hiragana katakana and reading for the same kanji. And things like NHK16 Yomichan local audio dict use katakana readings for pronunciation. It'd be really useful to support that, but we just don't right now.

However I also don't want a `reading_1`, `reading_2`, `reading_3` etc columns in the database. It's theoretically possible to do that, but that's really ugly.

I want to rewrite the Yomitan importer to take advantage of async. But how will it affect parsing performance? Since right now we can use multicore to parse all of the entries in parallel. But our bottleneck is inserting them into the database. Need to investigate more.

what happens we import a yomitan dictionary?

- we import (expr 日本語 reading にほんご)
  - we know for a fact that にほんご is a reading of 日本語
  - we don't know for certain whether 日本語 is a headword, but it probably is

- or: we import (expr 日本語 reading ())
  - we don't know what 日本語, it could be a reading or a headword
  - for now we'll treat it as a headword, but if we get evidence of the contrary (i.e. an entry where reading = 日本語), then we'll change our mind

- or: we import (expr () reading にほんご)
  - if we've already imported (日本語, にほんご) then this term already exists, skip
  - else, we don't know if this is a headword or a reading yet. assume it's a headword for now

querying:

CREATE TABLE IF NOT EXISTS term (
    text      TEXT    NOT NULL CHECK (text <> ''),
    record    INTEGER REFERENCES record(id),
    headword  INTEGER REFERENCES term(id),
    UNIQUE  (text, record)
);

- if we search for X, our goal is to find any terms where `term.text = X`
- if we find a bunch of terms, but all their `headword = NULL`, then we ASSUME that X is a headword, not a reading
- if we find at least 1 `term` for which `headword != NULL`, then X is a READING of 1 or more headwords
  - where `headword = Y AND headword != NULL`, pull in any terms where `term.text = Y` - and mark Y as the headword
- TODO: what if those terms pulled in also have `term.headword != NULL`? should that be allowed?

OR: have a headword and term tables

super basic yomitan async benchmark:
- async: 65.8 sec
- sync: 74.6 sec
