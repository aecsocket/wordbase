# Notes

how to perform a lookup:
- kanji/expression
  - find the lemma (mecab)
    - 食べなかった -> 食べる
    - たべなかった -> 食べる
  - find database records where `expression = lemma`

schema design decisions:
- we are working off of the yomitan format closely
- for meta stuff (dictionary, frequency, pitch etc.) we can make our own format
  which is convertible from yomitans
  - try to retain as much "machine-readable" info as possible, i.e. all frequencies
    have an integer value
- for glossary/rendering stuff, we'll use yomitan's own structured content format
  - easy to convert
  - easy to turn into html
  - not necessarily machine-readable, i.e. you can't read the font size as an integer,
    it's just a CSS `text-size` string

reference:
- https://github.com/sunwxg/gnome-shell-extension-arrangeWindows/blob/master/arrangeWindows%40sun.wxg%40gmail.com/extension.js
