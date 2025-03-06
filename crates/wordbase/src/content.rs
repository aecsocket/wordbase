//! Structured, renderable content element types in a [glossary].
//!
//! These types are based on the [Yomitan dictionary v3][yomitan] format, but
//! slightly modified to be normalized.
//!
//! [glossary]: crate::Glossary
//! [yomitan]: https://github.com/yomidevs/yomitan/

use std::{borrow::Cow, num::NonZero};

use derive_more::{Deref, DerefMut, Display, From};
use foldhash::HashMap;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

/// Top-level content element.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged, deny_unknown_fields)]
pub enum Content {
    /// Single line of plain text.
    ///
    /// If you want to create a value of this variant from a string, use
    /// [`Content::from_string`].
    Text(OneLineString),
    /// Compound element which may contain more [`Content`].
    Element(Box<Element>),
    /// Sequential list of [`Content`].
    List(Vec<Content>),
}

impl Default for Content {
    fn default() -> Self {
        Self::Text(OneLineString::EMPTY)
    }
}

impl Content {
    /// Creates either a [`Content::Text`] or [`Content::List`] from a string,
    /// converting [newlines] into [`Element::Br`] values.
    ///
    /// [newlines]: str::lines
    pub fn from_string<'a>(s: Cow<'a, String>) -> Self {
        let lines = s.lines().collect::<Vec<_>>();
        if lines.len() > 1 {
            #[expect(unstable_name_collisions, reason = "same functionality")]
            let children = lines
                .into_iter()
                .map(|line| Self::Text(OneLineString::new_unchecked(line.to_owned())))
                .intersperse(Self::Element(Box::new(LineBreakElement::default())))
                .collect::<Vec<_>>();
            Self::List(children)
        } else {
            Self::Text(OneLineString::new_unchecked(s.into_owned()))
        }
    }
}

impl<'de> Deserialize<'de> for Content {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // I know this is dogshit, there's no better way to do this though
        // "b-but youre using a private item thats not part of the publ-"
        // I know where you live. You have 48 hours.
        use serde::__private::de::{Content as Ast, ContentRefDeserializer};

        let content = Ast::deserialize(deserializer)?;
        let deserializer = ContentRefDeserializer::<D::Error>::new(&content);
        if let Ok(text) = String::deserialize(deserializer) {
            return Ok(Self::from_string(Cow::Owned(text)));
        }
        if let Ok(elem) = <Box<Element>>::deserialize(deserializer) {
            return Ok(Self::Element(elem));
        }
        if let Ok(children) = <Vec<Content>>::deserialize(deserializer) {
            return Ok(Self::List(children));
        }
        Err(serde::de::Error::custom(
            "data did not match any variant of untagged enum Content",
        ))
    }
}

/// [`String`] which is guaranteed to have no [newlines] inside of it.
///
/// [newlines]: str::lines
#[derive(Debug, Display, Clone, PartialEq, Eq, Hash, Deref, Serialize)]
pub struct OneLineString(String);

impl OneLineString {
    /// Empty string.
    const EMPTY: Self = OneLineString::new_unchecked(String::new());

    /// Creates a new value from a string without checking if it has newlines.
    #[must_use]
    pub const fn new_unchecked(s: String) -> Self {
        Self(s)
    }

    /// Gets a shared reference to the underlying string.
    #[must_use]
    pub fn get(&self) -> &str {
        &self.0
    }

    /// Takes the underlying string out of this value.
    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, From)]
#[serde(tag = "type")]
#[expect(clippy::large_enum_variant, reason = "most variants will be large")]
pub enum Element {
    LineBreak(LineBreakElement),
    Image(ImageElement),
    Link(LinkElement),
    Unstyled {
        kind: UnstyledKind,
        elem: UnstyledElement,
    },
    Styled {
        kind: StyledKind,
        elem: StyledElement,
    },
    Cell {
        kind: CellKind,
        elem: CellElement,
    },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LineBreakElement {
    pub data: Data,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnstyledElement {
    pub content: Content,
    pub data: Data,
    pub lang: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyledElement {
    pub content: Content,
    pub data: Data,
    pub style: ContentStyle,
    pub title: Option<String>,
    pub open: Option<bool>,
    pub lang: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CellElement {
    pub kind: CellKind,
    pub content: Content,
    pub data: Data,
    pub col_span: NonZero<u64>,
    pub row_span: NonZero<u64>,
    pub style: ContentStyle,
    pub lang: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LinkElement {
    pub content: Content,
    pub href: String,
    pub lang: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImageElement {
    pub data: Option<Data>,
    pub path: String,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub preferred_width: Option<f64>,
    pub preferred_height: Option<f64>,
    pub title: Option<String>,
    pub alt: Option<String>,
    pub description: Option<String>,
    pub pixelated: Option<bool>,
    pub image_rendering: Option<ImageRendering>,
    pub image_appearance: Option<ImageAppearance>,
    pub background: Option<bool>,
    pub collapsed: Option<bool>,
    pub collapsible: Option<bool>,
    pub vertical_align: Option<VerticalAlign>,
    pub border: Option<String>,
    pub border_radius: Option<String>,
    pub size_units: Option<SizeUnits>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UnstyledKind {
    Ruby,
    Rt,
    Rp,
    Table,
    Thead,
    Tbody,
    Tfoot,
    Tr,
}

crate::util::display_as_serialize!(UnstyledKind);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CellKind {
    Td,
    Th,
}

crate::util::display_as_serialize!(CellKind);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StyledKind {
    Span,
    Div,
    Ol,
    Ul,
    Li,
    Details,
    Summary,
}

crate::util::display_as_serialize!(StyledKind);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum VerticalAlign {
    Baseline,
    Sub,
    Super,
    TextTop,
    TextBottom,
    Middle,
    Top,
    Bottom,
}

crate::util::display_as_serialize!(VerticalAlign);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TextDecorationLine {
    Underline,
    Overline,
    LineThrough,
}

crate::util::display_as_serialize!(TextDecorationLine);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TextDecorationStyle {
    Solid,
    Double,
    Dotted,
    Dashed,
    Wavy,
}

crate::util::display_as_serialize!(TextDecorationStyle);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FontStyle {
    Normal,
    Italic,
}

crate::util::display_as_serialize!(FontStyle);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FontWeight {
    Normal,
    Bold,
}

crate::util::display_as_serialize!(FontWeight);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WordBreak {
    Normal,
    BreakAll,
    KeepAll,
}

crate::util::display_as_serialize!(WordBreak);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TextAlign {
    Start,
    End,
    Left,
    Right,
    Center,
    Justify,
}

crate::util::display_as_serialize!(TextAlign);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SizeUnits {
    Px,
    Em,
}

crate::util::display_as_serialize!(SizeUnits);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ImageRendering {
    Auto,
    Pixelated,
    CrispEdges,
}

crate::util::display_as_serialize!(ImageRendering);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ImageAppearance {
    Auto,
    Monochrome,
}

crate::util::display_as_serialize!(ImageAppearance);

#[derive(Debug, Display, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum NumberOrString {
    Number(f64),
    String(String),
}

#[derive(Debug, Clone, Default, Deref, DerefMut, Serialize, Deserialize)]
pub struct Data(pub HashMap<String, String>);

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContentStyle {
    pub font_style: Option<FontStyle>,
    pub font_weight: Option<FontWeight>,
    pub font_size: Option<String>,
    pub color: Option<String>,
    pub background: Option<String>,
    pub background_color: Option<String>,
    #[serde(default)]
    pub text_decoration_line: Vec<TextDecorationLine>,
    pub text_decoration_style: Option<TextDecorationStyle>,
    pub text_decoration_color: Option<String>,
    pub border_color: Option<String>,
    pub border_style: Option<String>,
    pub border_radius: Option<String>,
    pub border_width: Option<String>,
    pub clip_path: Option<String>,
    pub vertical_align: Option<VerticalAlign>,
    pub text_align: Option<TextAlign>,
    pub text_emphasis: Option<String>,
    pub text_shadow: Option<String>,
    pub margin: Option<String>,
    pub margin_top: Option<NumberOrString>,
    pub margin_left: Option<NumberOrString>,
    pub margin_right: Option<NumberOrString>,
    pub margin_bottom: Option<NumberOrString>,
    pub padding: Option<String>,
    pub padding_top: Option<String>,
    pub padding_left: Option<String>,
    pub padding_right: Option<String>,
    pub padding_bottom: Option<String>,
    pub word_break: Option<WordBreak>,
    pub white_space: Option<String>,
    pub cursor: Option<String>,
    pub list_style_type: Option<String>,
}
