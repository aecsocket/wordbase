# Overview

Wordbase is a set of tools and services for using dictionaries, looking up words, and integrating
tightly with your desktop to provide a seamless experience for language learning.

# Features

- Dictionary management
  - [x] Import [Yomitan] dictionaries
  - [x] Enable, disable, reorder, delete dictionaries
  - [ ] Import dictionaries via app or command-line
- Lookups
  - [x] Japanese deconjugation via MeCab
    - TODO: needs to be improved
  - [x] Query by specific record kinds (i.e. frequency, glossary)
- Popup dictionary
  - [x] Full HTML support via [WebKit]
  - [ ] Custom CSS theming
  - [ ] Add Anki note
  - [ ] Play pronunciation
- Anki integration
  - [ ] Create notes
  - [ ] Add screenshot and sentence audio to notes
- API for external apps
  - WebSocket
    - [x] Manage dictionaries
    - [x] Perform lookups
    - [x] Request to show the popup dictionary
    - [ ] Add Anki note
  - HTTP
    - [ ] Import dictionaries
- Desktop integration
  - Windows
    - (not supported yet)
  - MacOS
    - (not supported yet)
  - Linux/GNOME
    - [ ] Control popup dictionary position and overlay visibility
    - [x] Do lookups from your [texthooker] in your visual novel
    - [ ] Add Anki notes with VN screenshots
    - [ ] Font selection
  - Linux/KDE
    - (not supported yet)
  - Linux/X11
    - (not supported yet)
  - Linux/any other Wayland compositor
    - (not supported; see [Architecture](#architecture))

[Yomitan]: https://github.com/yomidevs/yomitan/
[WebKit]: https://webkitgtk.org/
[texthooker]: https://github.com/Artikash/Textractor/

# Installation

TODO

# Architecture

Wordbase is a large project, spanning multiple different programming languages, environments, and
tech stacks. We attempt to keep the codebase as platform-agnostic as possible, however there are
certain situations which require a target-specific approach.

## Standalone programs

### [`wordbase-server`](./crates/wordbase-server)

The heart of Wordbase. The server is a user-level process that runs headless in the background,
listening for incoming WebSocket connections (on `localhost` by default), and allows clients to
interface with the server. The server is responsible for:
- dictionary management - importing, removing, enabling, etc.
- text lookup logic
- [lemmatisation](https://en.wikipedia.org/wiki/Lemmatization)
- connecting to [texthooker servers](https://github.com/KamWithK/TextractorSender)
- spawning dictionary popups

### [`wordbase-cli`](./crates/wordbase-cli)

Command-line tool to query and manage the Wordbase server. This is not designed to be user-facing,
but as a quick tool to test if parts of the server are working without requiring a GUI. It also
serves as a reference implementation of a Wordbase client.

### 🛠️ `wordbase-manager`

GUI app using Adwaita/GTK to query and manage the Wordbase server. This, in contrast to the CLI,
is designed to be as user-friendly as possible. It provides a UI for managing dictionaries and other
server settings. In addition, it allows you to type in a text query and get a lookup for that text.

## Pop-up dictionary

Wordbase clients may request the server to spawn a pop-up dictionary to query for some
client-provided text at a client-specified position (relative to its own surface). This makes it
stupid simple for clients to integrate pop-up dictionary functionality, as they don't need to handle
performing a lookup or rendering contents; they just request the server to handle it for them.

This pop-up is shown as a window which is placed above all other windows on the desktop, and is
integrated into the server itself. It is not a standalone binary which can be launched outside of
the server. It uses a single WebView which covers the entire surface, and renders all dictionary
content into there.

However, this is a fairly platform-specific feature, and comes with some challenges to solve.

### Linux - Wayland

Wayland is the most challenging window manager to target for pop-ups due to its security features,
which disallow the client from having much control over its own window. However, it also means that
if we design for Wayland first, then we design for the worst-case scenario first, and supporting
all other platforms will be easier.

Under Wayland, there is no way for a client to read its own window's X, Y coordinates in screen
space, and there is no standard protocol to request the compositor to move your window to a
specific coordinate. While this is great for security, and it means other windows can't easily snoop
on what your window is doing or mess with the user experience, it puts apps like the pop-up
dictionary in a tricky situation.

To solve this, we write compositor-specific extensions which integrate with the compositor itself
and have the server request the extension (via DBus, not via WebSockets like other clients) to move
the pop-up window to the desired coordinates. Note that this makes the pop-up dictionary basically
desktop-specific; however, we support the two most commonly used desktops, and it's not particularly
hard to make an extension for other compositors which handle this functionality.

#### [`wordbase-integration`](./integrations/wordbase-integration@aecsocket.github.com)

A GNOME shell extension which handles this window movement functionality. In addition, it also
integrates the texthooker functionality into your desktop - when you receive a new sentence from
your texthooker, it will appear as a widget above the app's window, letting you perform word lookups
without ever leaving the app.

#### 🛠️ (KDE-specific integration)

TODO: write a KWin script to handle this

### 🛠️ Linux - X11

X11 is a much less restricting (and much less secure) protocol than Wayland in comparison, which
means we don't need to hook into the compositor to perform what we want.

TODO: so what do we do? I don't use X11 so...

### 🛠️ Windows

### 🛠️ MacOS

### Android + iOS

Unsupported due to platform limitations. Apps can't spawn arbitrary windows on top of other apps.

## Libraries

### [`wordbase`](./crates/wordbase)

Core library defining the types used by Wordbase clients and servers, and defines the communication
protocol between the client and server.

### [`wordbase-client-tokio`](./crates/wordbase-client-tokio)

Client library written in Rust used to interface with a Wordbase server via WebSockets. This is a
reference implementation of a client library.

### [`wordbase-html`](./crates/wordbase-html)

Allows rendering dictionary elements to HTML, which can be further rendered by a browser engine like
WebKit. This is used internally by the manager app and the pop-up dictionary.

# Bugs

- [ ] No Windows, KDE, MacOS support
- [ ] GNOME extension doesn't work if VN is fullscreen
- [ ] After unfullscreening VN, extension breaks rendering
- [ ] On initial popup, the popup is not located in the right place
- [ ] Deconjugator sucks ass
- [ ] Popup dictionary stylesheet sucks ass
- [ ] No dictionary theming
- [ ] Need options for click-to-lookup
- [ ] Popup dictionary doesn't render pitch info
- [ ] No Add to Anki button
- [ ] No local audio server
- [ ] No "play pronunciation" button
- [ ] Popup should remember its set size
- [ ] Popup disappears if you start click resizing it
- [ ] No config persistence
- [ ] Extension doesn't autoconnect to server
- [ ] Change font setting
- [ ] Add auto-updating dictionaries
- [ ] HTTP API for importing dictionaries
- [ ] Admin web panel
- [ ] `data-*` attributes in yomitan glossaries
