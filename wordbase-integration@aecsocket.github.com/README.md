# Workflow

## From the user's perspective

- Configuration
  - Install dictionaries
  - Configure AnkiConnect settings
- Running a visual novel
  - Run your VN
  - Run your texthooker
  - When you advance a line, a pop-up appears to show you what your texthooker read
  - You can move and resize this pop-up freely
  - You can click and highlight the text in this pop-up
  - Hovering over a word looks it up
  - You can press an "Add to Anki" button
  - When adding to Anki, we add the sentence, as well as any context e.g. window screenshot
  - If we can't connect to Anki, we auto-launch the Anki app

## From a technical perspective

> Configuration

Part of the GNOME extension settings. You can:

- add, remove, reorder dictionaries (in bulk)
- specify AnkiConnect settings
- specify fields to put into Anki card

> When you advance a line...

We use the [exSTATic](https://github.com/KamWithK/exSTATic/) WebSocket protocol to read input from
Textractor or any other texthooker which supports its protocol. When we read a new line, we know
that the user has just advanced their VN. Their currently focused window is probably their VN, and
we mark it as our "target window" - we can then use this window info to identify what VN they're
running, and store persistent info for that.

Each VN has its own persistent configuration - this includes:

- pop-up position and size
- full text log
- time spent with window opened
- time spent with window focused (track inactivity somehow?)

> ...a pop-up appears to show you what your texthooker read

We use the GNOME extension API to draw some kind of window in the shell on top of everything else,
and allow you to click into it. We should probably use the same technique that Memento uses for
non-web-view dictionary dialogs (or is it secretly a web-view? I need to look into this).

> When adding to Anki...

We have the sentence context from the WebSocket, and we can take a screenshot of the target window
(make sure that the pop-up isn't visible though). Then we send this via AnkiConnect over to Anki,
and make a new card.

# Improvements

Broad high-level improvements which might be possible:

- Have a central, desktop-wide database for dictionaries
  - Integrate with this GNOME extension, Yomitan browser extensions, Memento, etc.
  - How would we read it/send requests to it? I'm thinking a dictionary HTTP REST API.
    - More platform-agnostic than e.g. dbus
    - Conflicts with sandboxing (browser/Flatpak/...)?
- Have a central format for Japanese sentence mining cards
  - No need for the user to manually configure what templates map to which fields, standardise it
  - I'd use Lapis as the base
  - How different are JPMN and Lapis? Lapis vs. other mining note types in the ecosystem?
