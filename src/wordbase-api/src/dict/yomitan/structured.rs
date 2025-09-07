//! Glossary structured content schema.
//!
//! See [`structured-content.d.ts`](https://github.com/yomidevs/yomitan/blob/master/types/ext/structured-content.d.ts).
// Implementation note: a lot of these fields are left as `null` or unspecified.
// However, we can't add #[serde(skip_serializing_if)], because:
// - we import from JSON and serialize into the database as MessagePack
//   - this is stored with no field names, the field index determines its role
// - when querying, we deserialize the MessagePack and reserialize as JSON
//   - we know the JSON field name based on the MP field index
//
// If we added #[serde(skip_serializing_if)], we wouldn't have consistent MP field indices.
#![expect(missing_docs, reason = "these are not our types")]

use {
    derive_more::{Deref, DerefMut, Display},
    serde::{Deserialize, Serialize},
    std::collections::HashMap,
    std::fmt,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Content {
    String(String),
    Element(Box<Element>),
    Content(Vec<Content>),
}

#[cfg(feature = "uniffi")]
const _: () = {
    pub struct ElementFfi(Element);

    uniffi::custom_newtype!(ElementFfi, Element);

    #[derive(uniffi::Enum)]
    pub enum ContentFfi {
        String(String),
        Element(Vec<ElementFfi>),
        Content(Vec<ContentFfi>),
    }

    impl From<Content> for ContentFfi {
        fn from(value: Content) -> Self {
            match value {
                Content::String(s) => Self::String(s),
                Content::Element(e) => Self::Element(vec![ElementFfi(*e)]),
                Content::Content(v) => Self::Content(v.into_iter().map(Self::from).collect()),
            }
        }
    }

    impl TryFrom<ContentFfi> for Content {
        type Error = InvalidElement;

        fn try_from(value: ContentFfi) -> Result<Self, Self::Error> {
            Ok(match value {
                ContentFfi::String(s) => Self::String(s),
                ContentFfi::Element(e) => {
                    let [elem] = <[ElementFfi; 1]>::try_from(e).map_err(|_| InvalidElement)?;
                    Self::Element(Box::new(elem.0))
                }
                ContentFfi::Content(v) => Self::Content(
                    v.into_iter()
                        .map(Self::try_from)
                        .collect::<Result<Vec<_>, _>>()?,
                ),
            })
        }
    }

    #[derive(Debug, Display, derive_more::Error)]
    #[display("`Content.Element` must contain exactly 1 element in list")]
    pub struct InvalidElement;

    uniffi::custom_type!(Content, ContentFfi);
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
#[serde(tag = "tag", rename_all = "kebab-case", deny_unknown_fields)]
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
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LineBreakElement {
    pub data: Option<Data>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UnstyledElement {
    pub content: Option<Content>,
    pub data: Option<Data>,
    pub lang: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
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
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
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
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
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
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
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
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LinkElement {
    pub content: Option<Content>,
    pub href: String,
    pub lang: Option<String>,
}

// styling

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
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
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
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
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
#[serde(rename_all = "kebab-case")]
pub enum TextDecorationLine {
    Underline,
    Overline,
    LineThrough,
}

display_as_serialize!(TextDecorationLine);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
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
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
#[serde(rename_all = "kebab-case")]
pub enum FontStyle {
    Normal,
    Italic,
}

display_as_serialize!(FontStyle);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
#[serde(rename_all = "kebab-case")]
pub enum FontWeight {
    Normal,
    Bold,
}

display_as_serialize!(FontWeight);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
#[serde(rename_all = "kebab-case")]
pub enum WordBreak {
    Normal,
    BreakAll,
    KeepAll,
}

display_as_serialize!(WordBreak);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
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
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
#[serde(rename_all = "kebab-case")]
pub enum SizeUnits {
    Px,
    Em,
}

display_as_serialize!(SizeUnits);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
#[serde(rename_all = "kebab-case")]
pub enum ImageRendering {
    Auto,
    Pixelated,
    CrispEdges,
}

display_as_serialize!(ImageRendering);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
#[serde(rename_all = "kebab-case")]
pub enum ImageAppearance {
    Auto,
    Monochrome,
}

display_as_serialize!(ImageAppearance);

#[derive(Debug, Display, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
#[serde(untagged)]
pub enum NumberOrString {
    Number(f64),
    String(String),
}

#[derive(Debug, Clone, Default, Deref, DerefMut, Serialize, Deserialize)]
pub struct Data(pub HashMap<String, String>);

#[cfg(feature = "uniffi")]
uniffi::custom_newtype!(Data, HashMap<String, String>);

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
