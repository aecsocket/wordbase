use std::collections::HashMap;

use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TextDecorationLine {
    Underline,
    Overline,
    LineThrough,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TextDecorationStyle {
    Solid,
    Double,
    Dotted,
    Dashed,
    Wavy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FontStyle {
    Normal,
    Italic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FontWeight {
    Normal,
    Bold,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WordBreak {
    Normal,
    BreakAll,
    KeepAll,
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SizeUnits {
    Px,
    Em,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ImageRendering {
    Auto,
    Pixelated,
    CrispEdges,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ImageAppearance {
    Auto,
    Monochrome,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Data(pub HashMap<String, String>);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum NumberOrString {
    Number(f64),
    String(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LineBreak {
    pub data: Option<Data>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UnstyledElement {
    pub content: Option<Content>,
    pub data: Option<Data>,
    pub lang: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TableElement {
    content: Option<Content>,
    data: Option<Data>,
    col_span: Option<i64>,
    row_span: Option<i64>,
    style: Option<ContentStyle>,
    lang: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StyledElement {
    content: Option<Content>,
    data: Option<Data>,
    style: Option<ContentStyle>,
    title: Option<String>,
    open: Option<bool>,
    lang: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageElement {
    #[serde(flatten)]
    pub base: ImageElementBase,
    pub vertical_align: Option<VerticalAlign>,
    pub border: Option<String>,
    pub border_radius: Option<String>,
    pub size_units: Option<SizeUnits>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LinkElement {
    pub content: Option<Content>,
    pub href: String,
    pub lang: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "tag", deny_unknown_fields)]
#[expect(clippy::large_enum_variant, reason = "most variants will be large")]
pub enum Element {
    #[serde(rename = "br")]
    LineBreak { data: Option<Data> },
    #[serde(rename = "ruby")]
    UnstyledElementRuby(UnstyledElement),
    #[serde(rename = "rt")]
    UnstyledElementRt(UnstyledElement),
    #[serde(rename = "rp")]
    UnstyledElementRp(UnstyledElement),
    #[serde(rename = "table")]
    UnstyledElementTable(UnstyledElement),
    #[serde(rename = "thead")]
    UnstyledElementThead(UnstyledElement),
    #[serde(rename = "tbody")]
    UnstyledElementTbody(UnstyledElement),
    #[serde(rename = "tfoot")]
    UnstyledElementTfoot(UnstyledElement),
    #[serde(rename = "tr")]
    UnstyledElementTr(UnstyledElement),
    #[serde(rename = "td")]
    TableElementTd(TableElement),
    #[serde(rename = "th")]
    TableElementTh(TableElement),
    #[serde(rename = "span")]
    StyledElementSpan(StyledElement),
    #[serde(rename = "div")]
    StyledElementDiv(StyledElement),
    #[serde(rename = "ol")]
    StyledElementOl(StyledElement),
    #[serde(rename = "ul")]
    StyledElementUl(StyledElement),
    #[serde(rename = "li")]
    StyledElementLi(StyledElement),
    #[serde(rename = "details")]
    StyledElementDetails(StyledElement),
    #[serde(rename = "summary")]
    StyledElementSummary(StyledElement),
    #[serde(rename = "img")]
    ImageElement(ImageElement),
    #[serde(rename = "a")]
    LinkElement(LinkElement),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged, deny_unknown_fields)]
pub enum Content {
    String(String),
    Element(Box<Element>),
    Content(Vec<Content>),
}
