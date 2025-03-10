//! Glossary structured content schema.
//!
//! See [`structured-content.d.ts`](https://github.com/yomidevs/yomitan/blob/master/types/ext/structured-content.d.ts).
#![expect(missing_docs, reason = "these are not our types")]

use {
    derive_more::{Deref, DerefMut, Display},
    foldhash::HashMap,
    serde::{Deserialize, Serialize},
};

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
#[serde(rename_all = "kebab-case")]
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LineBreak {
    pub data: Option<Data>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UnstyledElement {
    pub content: Option<Content>,
    pub data: Option<Data>,
    pub lang: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TableElement {
    pub content: Option<Content>,
    pub data: Option<Data>,
    pub col_span: Option<i64>,
    pub row_span: Option<i64>,
    pub style: Option<ContentStyle>,
    pub lang: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StyledElement {
    pub content: Option<Content>,
    pub data: Option<Data>,
    pub style: Option<ContentStyle>,
    pub title: Option<String>,
    pub open: Option<bool>,
    pub lang: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageElementBase {
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
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageElement {
    #[serde(flatten)]
    pub base: ImageElementBase,
    pub vertical_align: Option<VerticalAlign>,
    pub border: Option<String>,
    pub border_radius: Option<String>,
    pub size_units: Option<SizeUnits>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LinkElement {
    pub content: Option<Content>,
    pub href: String,
    pub lang: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "tag", rename_all = "kebab-case", deny_unknown_fields)]
#[expect(clippy::large_enum_variant, reason = "most variants will be large")]
pub enum Element {
    Br { data: Option<Data> },
    Ruby(UnstyledElement),
    Rt(UnstyledElement),
    Rp(UnstyledElement),
    Table(UnstyledElement),
    Thead(UnstyledElement),
    Tbody(UnstyledElement),
    Tfoot(UnstyledElement),
    Tr(UnstyledElement),
    Td(TableElement),
    Th(TableElement),
    Span(StyledElement),
    Div(StyledElement),
    Ol(StyledElement),
    Ul(StyledElement),
    Li(StyledElement),
    Details(StyledElement),
    Summary(StyledElement),
    Img(ImageElement),
    A(LinkElement),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Content {
    // todo doc: must not have \n's
    String(String),
    Element(Box<Element>),
    Content(Vec<Content>),
}
