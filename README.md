# ⚠️ ⚠️ WARNING ⚠️ ⚠️

This project is nowhere near ready for serious use! While some core functionality is implemented and basically complete, other features are still highly subject to change. Database data loss MAY occur, and databases may be incompatible between versions. You have been warned.

Also, I haven't decided on a license yet.

# Features

- Engine (`wordbase-engine`)
  - Importing and managing dictionaries in various formats
    - [x] [Yomitan]
    - [x] [Yomichan Local Audio]
    - more to come...
  - Dictionary sorting
    - [x] User-defined dictionary ordering
    - [x] Via a frequency dictionary
    - [x] Sorting within terms in a single dictionary
  - Language support
    - [x] Japanese
      - Deinflection powered by [Lindera] using UniDic
      - Furigana generation
    - more to come...
  - [x] Creating and managing profiles
  - [x] Connecting to texthookers via the [exSTATic]/[TextractorSender] protocol
  - [ ] AnkiConnect integration

# Usage

At this point, the only thing you can really use is the core engine functionality, which is intended for developers*. There's no real user-facing app or functionality yet. All of the instructions in this section will change in the future to be more user-friendly. But for now:

- Clone the repo
- Run `cargo run -p wordbase-engine-cli` to interact with the database (import a dictionary, do a lookup, etc.)
- If you can program in Rust, you can write a crate which imports `wordbase-engine` and interact with the engine through that

I plan on writing a server (`wordbase-server`) to allow you to interact with the engine via a WebSocket, although I'm not sure if that will be a binary or embedded into the desktop app. Maybe both?

*yes the desktop app crate exists. I do NOT recommend you use it for anything right now. It sucks. I won't even give you instructions to run it.

# Goals

## Be a *language learning platform*

There exist many tools for language learning, offering features like popup dictionaries, pronunciation audio, etc. These exist in various forms, like desktop apps, mobile apps, and browser extensions. But due to the fragmented nature of the ecosystem, there's almost no consistency in any of these - one app may use a hardcoded dictionary, another may fetch data from a server online, whereas another may require you to import dictionaries manually - but those dictionaries aren't shared with any other apps, even though they use the same format.

As a real-life example, [Yomitan] is a browser extension providing a popup dictionary specialized for Japanese language learning. The Yomitan dictionary format is used by various tools - however, it's impossible for an external app (like [Memento], a Japanese language learning video player) to actually use the dictionaries stored inside Yomitan itself, since it's a browser extension. This means duplicate effort on writing logic for importing and rendering Yomitan dictionaries, performing deinflection, etc. - not a simple task! Despite this, there are still inconsistences between the two - e.g., Memento struggles to render more complicated glossary contents due to the Yomitan dictionary format's structured content format.

There's great value in having a single shared database of dictionaries, and a single consistent way to fetch data for some text. This is ultimately the goal of Wordbase - make it stupid simple to integrate a dictionary, and any other language learning tools, into any app.

## Be *simple to use*

Following the GNOME philosophy of "every preference has a cost"[^1][^2][^3], Wordbase *the engine* focuses on providing a solid foundation to build language learning tools on top of, favoring robustness over features. Wordbase *the app* focuses on being easy to pick up and use immediately by anyone, rather than being as flexible as possible. Features will be added if they benefit the 90% of users, and don't significantly increase the cognitive complexity of the app.

Of course, this doesn't mean that Wordbase *the platform* is inflexible. On the contrary, developers should be able (and encouraged!) to write their own software which hooks into Wordbase's API to solve problems in areas which Wordbase itself won't tackle. This includes software like browser extensions, video players, and OCR tools.

Perhaps this is a niche goal, considering the typical audience of apps like this are the people who are already more technically inclined - for many people, even getting Anki set up in the first place is a hurdle - but in my opinion, the more accurate viewpoint is that the average language learner doesn't use these tools *because* they're designed for the more technically inclined. Lowering the barrier to entry for language learning is a noble goal in of itself, and one worth pursuing.

TODO: I'm reconsidering the simplicity goal for the engine itself. Perhaps move the complexity into the engine and server, and make the app itself simple (via min-config etc.)

[^1]: https://www.omgubuntu.co.uk/2021/07/why-gnome-does-what-gnome-does-by-tobias-bernard
[^2]: https://blogs.gnome.org/tbernard/2021/07/13/community-power-4/
[^3]: https://ometer.com/preferences.html

## First-class *Linux/Wayland/Flatpak support*

Out of the existing language learning tools available, a small number have first-class Linux support, and an even smaller number have support for the modern Linux desktop with Wayland and Flatpak. This number drops even further when delving into niches like visual novel texthookers and related tools ([Textractor], [JL]). This is understandable, as Linux is still a small fraction of market share - but it's a steadily growing fraction, and its users deserve something nicer.

# Architecture

## Overview

### Terms, records, and lookups

We start with the concept of a *term*, which is the key for what you would typically consider a "dictionary entry". For example, the term "rust" would be a key for:
- the definitions of the word "rust"
  1. coating of iron oxide on iron formed by oxidation
  2. a programming language
  3. a video game developed by Facepunch Studios
- the pronunciation information for the word "rust"
  - /rəst/
- how often the word "rust" appears in a corpus
  - occurs about 3 times per million words according to the [Oxford English Dictionary][oed]

[oed]: https://www.oed.com/dictionary/rust_n1

Each of these pieces of information is called a *record*, and each record is of a specific *kind*. A term can have any number of records, and can have multiple records of the same kind. A term may also have both a *headword* and a *reading*, to support languages where the same word can have multiple forms. For example, for the term "錆", the headword is "錆" and the reading is "さび". Querying for either of these will result in the same records being returned.

However, terms may not always appear in the exact headword or reading form. For example, searching for "eating" will not give the records for "eat", as they are two separate terms. This is where *deinflection* comes in, which will transform "eating" into "eat". Users don't need to worry about how this happens - this is the responsibility of the Wordbase engine.

The combination of "deinflect this term into its canonical form" and "query the database for records for this term" is called a "lookup", and is at the heart of Wordbase.

### Dictionaries

To get records, we import them from a *dictionary*. There are many formats of dictionaries that exist in the wild, and Wordbase aims to support as many as possible (or at least, makes it easy to add new ones). Dictionaries can be imported via the Wordbase app, and once added, they can be reordered, disabled, or removed. Placing a dictionary higher in the ordering makes its records appear earlier when looking up a term, giving it a greater priority.

You can also mark a single dictionary as a *sorting dictionary*, which will be used to sort records in a lookup. The records are sorted by what *frequency* that term has in the sorting dictionary. If a term appears more often - i.e. it has a lower ranking, or a higher occurrence - it will appear earlier in the results.

### Profiles

You may want different settings for different situations, e.g. use a different set of dictionaries when studying Japanese versus when studying Mandarin. For this, you can create different *profiles* and switch between them in the app or when performing lookups. Each profile stores its own settings, and tracks which dictionaries are enabled separately.

Not all state is separate between profiles - for example, the actual dictionaries you have imported, and their ordering, is common across all profiles. App-level settings such as the selected theme are also shared across profiles.

### API and the popup dictionary

External apps may want to use Wordbase's dictionaries and lookup functions without having to include Wordbase in their app themselves - for example, a video player or web browser extension where you can click on words to see their definitions. To support this, Wordbase apps expose an API for developers to perform lookups.

Currently, the API is exposed to 3rd party apps as an HTTP service running at `http://127.0.0.1:9518`. This is meant to be used only locally, and **never exposed** to other machines - **there is no authentication mechanism**. In the future, we may add a WebSocket API alongside the HTTP API, and possibly platform-specific APIs like DBus, Unix sockets, or Windows named pipes.

Even though external apps can perform lookups, they don't necessarily have to implement all of the logic for rendering those lookup results. Instead, if it is supported on the current platform, they can request the server to spawn a dictionary popup at a specific location relative to the app's own window. This will automatically handle scanning the text, performing the record lookup, creating the popup window, and positioning it. Wordbase's goal is to make it as simple as possible for 3rd party developers to integrate with the dictionary.

### Texthooker overlay

Japanese learners who use visual novels to study are likely familiar with the concept of a texthooker like Textractor. This is an app that hooks into the memory of the visual novel you're playing, and extracts the text from the current dialog box to be displayed in another app where you can perform lookups. The traditional approach of using a texthooker is:
- you open your visual novel
- you open your texthooker and attach it to the game
- you open a web browser with Yomitan and a clipboard inserter extension
- you open a blank texthooking page, which continuously pastes the contents of your clipboard into the page
- when a new sentence appears, the texthooker reads it from memory
- texthooker copies it to clipboard
- browser extension copies clipboard contents to webpage
- scan the text in the webpage

Wordbase simplifies this:
- you open your visual novel
- you open your texthooker and attach it to the game
- when a new sentence appears, you see it appear as scannable text in an overlay above the game window
- scan the text in the overlay

This is achieved by Wordbase connecting to your texthooker via [TextractorSender], receiving sentences, and pushing those sentences into an overlay window which sits on top of your VN. This window then integrates with the rest of Wordbase by triggering a popup dictionary when selecting a word.

In the future this may be moved into a separate app, although this is TBD.

[TextractorSender]: https://github.com/KamWithK/TextractorSender

### Anki integration

After looking up a word, you may want to add it to your Anki deck to study it later. Wordbase allows you to connect to an [AnkiConnect] server, which adds a button to the popup dictionary allowing you to add the word as a note to your specified deck. This will also ask the app which originally requested the popup to provide an image and sentence audio for the note, so you can get the most context-specific information for your note.

[AnkiConnect]: https://ankiweb.net/shared/info/2055492159

## Projects

### 📦 `wordbase`

Provides the core API types, and defines the protocol for communicating with the engine (which actually performs most of the logic). This includes all of the kinds of records that the engine may store and provide.

### 📦 `wordbase-engine`

This is the heart of Wordbase, which implements the logic for:
- managing and selecting profiles
- importing, managing, and deleting dictionaries
- storing dictionary records in a database
- performing deinflection
- performing lookups

The engine is a library, not a binary - it is intended to be packaged inside of an app. This is because the engine only implements the platform-agnostic logic, and cannot perform platform-dependent actions like spawning a popup dictionary. The app may also choose to not support some functions (i.e. on a mobile platform, you may not be able to spawn a popup on top of the currently active app).

The engine also does not handle allowing clients to communicate with the engine, and leaves this up to the app (via e.g. a WebSocket server, DBus, or some other form of IPC).

### ⚙️ `wordbase-desktop`

This is a GTK/Adwaita app which runs on the desktop, and runs `wordbase-engine` plus exposes ways for clients to talk to the engine via a WebSocket server and DBus. This app targets the Linux GNOME desktop as the first-class target, and aims to follow modern Linux desktop app standards, including using Wayland and being compatible with Flatpak sandboxing. However, it also aims to be as cross-platform as possible (though not necessarily as native as possible on other platforms). This implements the logic for:
- running and persisting an engine instance
- rendering dictionary contents, and allowing users to search all dictionaries
- showing a user-friendly GUI to manage the engine
- spawning an overlay window when a texthooker is connected
- importing dictionary files
- running a WebSocket server (requires extra Flatpak permissions)
- on Linux, providing the DBus service over the app's known name

Currently, this app uses Relm4 on top of GTK/Adwaita, but we might switch this to raw GTK/Adwaita in the future. Relm4 doesn't help us much, considering that state is mostly stored outside of memory (in SQLite, the filesystem, etc.), and its component model adds complexity.

### ⚙️ `wordbase-mobile`

TODO: could we make a mobile app? Have it use accessibility APIs to render on top of other app content, and show a popup?

### 📦 `wordbase-client-tokio`

Provides API for interacting with a Wordbase engine through a WebSocket server. Uses Tokio and Tungstenite for the WebSocket implementation.

### 🧩 `wordbase-integration`

GNOME extension to integrate Wordbase into Mutter, by allowing the app to request the window manager to perform window manager-specific actions, e.g. set the position of the popup window.

## Popup dictionary

Wordbase clients may request the server to spawn a pop-up dictionary to query for some client-provided text at a client-specified position (relative to its own surface). This makes it stupid simple for clients to integrate pop-up dictionary functionality, as they don't need to handle performing a lookup or rendering contents; they just request the server to handle it for them.

This pop-up is shown as a window which is placed above all other windows on the desktop, and is integrated into the server itself. It is not a standalone binary which can be launched outside of the server. It uses a single WebView which covers the entire surface, and renders all dictionary content into there.
However, this is a fairly platform-specific feature, and comes with some challenges to solve.

### Linux - Wayland

Wayland is the most challenging window manager to target for pop-ups due to its security features, which disallow the client from having much control over its own window. However, it also means that if we design for Wayland first, then we design for the most difficult scenario first, and supporting all other platforms will be easier.

Under Wayland, there is no way for a client to read its own window's X, Y coordinates in screen space, and there is no standard protocol to request the compositor to move your window to a specific coordinate. While this is great for security, and it means other windows can't easily snoop on what your window is doing or mess with the user experience, it puts apps like the pop-up dictionary in a tricky situation.

To solve this, we write compositor-specific extensions which integrate with the compositor itself and have the server request the extension (via DBus, not via WebSockets like other clients) to move the pop-up window to the desired coordinates. Note that this makes the pop-up dictionary basically desktop-specific; however, we aim to support the most commonly used desktops, and it's not particularly hard to make an extension for other compositors which handle this functionality.

### Linux - X11

X11 is a much less restricting (and much less secure) protocol than Wayland in comparison, which means we don't need to hook into the compositor to perform what we want.

TODO: so what do we do? I don't use X11 so...

### Windows

TODO

### MacOS

TODO

### Android + iOS

Unsupported due to platform limitations. Apps can't spawn arbitrary windows on top of other apps.

TODO: this might not be entirely true...

[Yomitan]: https://github.com/yomidevs/yomitan
[Yomichan Local Audio]: https://github.com/yomidevs/local-audio-yomichan
[Memento]: https://github.com/ripose-jp/memento
[Lindera]: https://github.com/lindera/lindera
[Textractor]: https://github.com/Artikash/Textractor
[JL]: https://github.com/rampaa/JL
[exSTATic]: https://github.com/KamWithK/exSTATic
[TextractorSender]: https://github.com/KamWithK/TextractorSender
