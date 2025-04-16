//! Glossary structured content schema.
//!
//! See [`structured-content.d.ts`](https://github.com/yomidevs/yomitan/blob/master/types/ext/structured-content.d.ts).
#![expect(missing_docs, reason = "these are not our types")]

use {
    derive_more::{Deref, DerefMut, Display},
    foldhash::HashMap,
    serde::{Deserialize, Serialize},
    std::fmt,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Content {
    String(String),
    Element(Box<Element>),
    Content(Vec<Content>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "tag", rename_all = "kebab-case", deny_unknown_fields)]
#[expect(clippy::large_enum_variant, reason = "most variants will be large")]
pub enum Element {
    Br(LineBreakElement),
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LineBreakElement {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Data>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UnstyledElement {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Data>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TableElement {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Data>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub col_span: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub row_span: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<ContentStyle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StyledElement {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Data>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<ContentStyle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub open: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageElement {
    #[serde(flatten)]
    pub base: ImageElementBase,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vertical_align: Option<VerticalAlign>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border_radius: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_units: Option<SizeUnits>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageElementBase {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Data>,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preferred_width: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preferred_height: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pixelated: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_rendering: Option<ImageRendering>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_appearance: Option<ImageAppearance>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collapsed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collapsible: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LinkElement {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<Content>,
    pub href: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
}

// styling

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContentStyle {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_style: Option<FontStyle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_weight: Option<FontWeight>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_size: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background_color: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub text_decoration_line: Vec<TextDecorationLine>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_decoration_style: Option<TextDecorationStyle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_decoration_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border_style: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border_radius: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border_width: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clip_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vertical_align: Option<VerticalAlign>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_align: Option<TextAlign>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_emphasis: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_shadow: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub margin: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub margin_top: Option<NumberOrString>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub margin_left: Option<NumberOrString>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub margin_right: Option<NumberOrString>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub margin_bottom: Option<NumberOrString>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub padding: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub padding_top: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub padding_left: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub padding_right: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub padding_bottom: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub word_break: Option<WordBreak>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub white_space: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_style_type: Option<String>,
}

macro_rules! display_as_serialize {
    ($T:ty) => {
        const _: () = {
            use std::fmt;

            impl fmt::Display for $T {
                fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                    let serializer = FormatterSerializer { f };
                    self.serialize(serializer)
                }
            }
        };
    };
}

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

display_as_serialize!(VerticalAlign);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TextDecorationLine {
    Underline,
    Overline,
    LineThrough,
}

display_as_serialize!(TextDecorationLine);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TextDecorationStyle {
    Solid,
    Double,
    Dotted,
    Dashed,
    Wavy,
}

display_as_serialize!(TextDecorationStyle);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FontStyle {
    Normal,
    Italic,
}

display_as_serialize!(FontStyle);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FontWeight {
    Normal,
    Bold,
}

display_as_serialize!(FontWeight);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WordBreak {
    Normal,
    BreakAll,
    KeepAll,
}

display_as_serialize!(WordBreak);

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

display_as_serialize!(TextAlign);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SizeUnits {
    Px,
    Em,
}

display_as_serialize!(SizeUnits);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ImageRendering {
    Auto,
    Pixelated,
    CrispEdges,
}

display_as_serialize!(ImageRendering);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ImageAppearance {
    Auto,
    Monochrome,
}

display_as_serialize!(ImageAppearance);

#[derive(Debug, Display, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum NumberOrString {
    Number(f64),
    String(String),
}

#[derive(Debug, Clone, Default, Deref, DerefMut, Serialize, Deserialize)]
pub struct Data(pub HashMap<String, String>);

// utils

struct FormatterSerializer<'a, 'b> {
    pub f: &'a mut fmt::Formatter<'b>,
}

impl serde::Serializer for FormatterSerializer<'_, '_> {
    type Ok = ();
    type Error = fmt::Error;

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        write!(self.f, "{variant}")
    }

    serde::__serialize_unimplemented! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str bytes none some
        unit unit_struct newtype_struct newtype_variant
        seq tuple tuple_struct tuple_variant map struct struct_variant
    }
}
