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
    std::collections::HashMap,
};

#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(untagged)
)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize),
    rkyv(
        derive(Debug),
        serialize_bounds(
            __S: rkyv::ser::Writer + rkyv::ser::Allocator,
            __S::Error: rkyv::rancor::Source
        ),
        deserialize_bounds(__D::Error: rkyv::rancor::Source),
        bytecheck(
            bounds(
                __C: rkyv::validation::ArchiveContext
            )
        )
    )
)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub enum Content {
    String(String),
    Element(#[cfg_attr(feature = "rkyv", rkyv(omit_bounds))] Box<Element>),
    Content(#[cfg_attr(feature = "rkyv", rkyv(omit_bounds))] Vec<Content>),
}

#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(tag = "tag", rename_all = "kebab-case", deny_unknown_fields)
)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize),
    rkyv(derive(Debug))
)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
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

#[derive(Debug, Clone, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase", deny_unknown_fields)
)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize),
    rkyv(derive(Debug))
)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct LineBreakElement {
    pub data: Option<Data>,
}

#[derive(Debug, Clone, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase", deny_unknown_fields)
)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize),
    rkyv(derive(Debug))
)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct UnstyledElement {
    pub content: Option<Content>,
    pub data: Option<Data>,
    pub lang: Option<String>,
}

#[derive(Debug, Clone, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase", deny_unknown_fields)
)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize),
    rkyv(derive(Debug))
)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct TableElement {
    pub content: Option<Content>,
    pub data: Option<Data>,
    pub col_span: Option<i64>,
    pub row_span: Option<i64>,
    pub style: Option<ContentStyle>,
    pub lang: Option<String>,
}

#[derive(Debug, Clone, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase", deny_unknown_fields)
)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize),
    rkyv(derive(Debug))
)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct StyledElement {
    pub content: Option<Content>,
    pub data: Option<Data>,
    pub style: Option<ContentStyle>,
    pub title: Option<String>,
    pub open: Option<bool>,
    pub lang: Option<String>,
}

#[derive(Debug, Clone, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize),
    rkyv(derive(Debug))
)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct ImageElement {
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub base: ImageElementBase,
    pub vertical_align: Option<VerticalAlign>,
    pub border: Option<String>,
    pub border_radius: Option<String>,
    pub size_units: Option<SizeUnits>,
}

#[derive(Debug, Clone, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize),
    rkyv(derive(Debug))
)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
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

#[derive(Debug, Clone, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase", deny_unknown_fields)
)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize),
    rkyv(derive(Debug))
)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct LinkElement {
    pub content: Option<Content>,
    pub href: String,
    pub lang: Option<String>,
}

// styling

#[derive(Debug, Clone, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase", deny_unknown_fields)
)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize),
    rkyv(derive(Debug))
)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct ContentStyle {
    pub font_style: Option<FontStyle>,
    pub font_weight: Option<FontWeight>,
    pub font_size: Option<String>,
    pub color: Option<String>,
    pub background: Option<String>,
    pub background_color: Option<String>,
    #[cfg_attr(feature = "serde", serde(default))]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::EnumIter, strum::Display)]
#[strum(serialize_all = "kebab-case")]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize),
    rkyv(derive(Debug, Clone, Copy, PartialEq, Eq, Hash))
)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::EnumIter, strum::Display)]
#[strum(serialize_all = "kebab-case")]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize),
    rkyv(derive(Debug, Clone, Copy, PartialEq, Eq, Hash))
)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
pub enum TextDecorationLine {
    Underline,
    Overline,
    LineThrough,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::EnumIter, strum::Display)]
#[strum(serialize_all = "kebab-case")]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize),
    rkyv(derive(Debug, Clone, Copy, PartialEq, Eq, Hash))
)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
pub enum TextDecorationStyle {
    Solid,
    Double,
    Dotted,
    Dashed,
    Wavy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::EnumIter, strum::Display)]
#[strum(serialize_all = "kebab-case")]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize),
    rkyv(derive(Debug, Clone, Copy, PartialEq, Eq, Hash))
)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
pub enum FontStyle {
    Normal,
    Italic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::EnumIter, strum::Display)]
#[strum(serialize_all = "kebab-case")]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize),
    rkyv(derive(Debug, Clone, Copy, PartialEq, Eq, Hash))
)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
pub enum FontWeight {
    Normal,
    Bold,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::EnumIter, strum::Display)]
#[strum(serialize_all = "kebab-case")]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize),
    rkyv(derive(Debug, Clone, Copy, PartialEq, Eq, Hash))
)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
pub enum WordBreak {
    Normal,
    BreakAll,
    KeepAll,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::EnumIter, strum::Display)]
#[strum(serialize_all = "kebab-case")]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize),
    rkyv(derive(Debug, Clone, Copy, PartialEq, Eq, Hash))
)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
pub enum TextAlign {
    Start,
    End,
    Left,
    Right,
    Center,
    Justify,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::EnumIter, strum::Display)]
#[strum(serialize_all = "kebab-case")]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize),
    rkyv(derive(Debug, Clone, Copy, PartialEq, Eq, Hash))
)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
pub enum SizeUnits {
    Px,
    Em,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::EnumIter, strum::Display)]
#[strum(serialize_all = "kebab-case")]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize),
    rkyv(derive(Debug, Clone, Copy, PartialEq, Eq, Hash))
)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
pub enum ImageRendering {
    Auto,
    Pixelated,
    CrispEdges,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::EnumIter, strum::Display)]
#[strum(serialize_all = "kebab-case")]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize),
    rkyv(derive(Debug, Clone, Copy, PartialEq, Eq, Hash))
)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
pub enum ImageAppearance {
    Auto,
    Monochrome,
}

#[derive(Debug, Display, Clone)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(untagged)
)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize),
    rkyv(derive(Debug))
)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
pub enum NumberOrString {
    Number(f64),
    String(String),
}

#[derive(Debug, Clone, Default, Deref, DerefMut)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize),
    rkyv(derive(Debug, Deref, DerefMut))
)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct Data(pub HashMap<String, String>);

#[cfg(feature = "uniffi")]
uniffi::custom_newtype!(Data, HashMap<String, String>);

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

#[cfg(test)]
mod tests {
    use {
        crate::v1::yomitan::structured::{
            FontStyle, FontWeight, ImageAppearance, ImageRendering, SizeUnits, TextAlign,
            TextDecorationLine, TextDecorationStyle, VerticalAlign, WordBreak,
        },
        serde::Serialize,
        std::fmt::Display,
        strum::IntoEnumIterator,
    };

    fn assert_repr_eq<T: Clone + Display + Serialize>(t: T) {
        assert_eq!(
            t.clone().to_string(),
            serde_json::to_value(t).unwrap().as_str().unwrap()
        );
    }

    #[test]
    fn enum_display_eq_serialize() {
        VerticalAlign::iter().for_each(assert_repr_eq);
        TextDecorationLine::iter().for_each(assert_repr_eq);
        TextDecorationStyle::iter().for_each(assert_repr_eq);
        FontStyle::iter().for_each(assert_repr_eq);
        FontWeight::iter().for_each(assert_repr_eq);
        WordBreak::iter().for_each(assert_repr_eq);
        TextAlign::iter().for_each(assert_repr_eq);
        SizeUnits::iter().for_each(assert_repr_eq);
        ImageRendering::iter().for_each(assert_repr_eq);
        ImageAppearance::iter().for_each(assert_repr_eq);
    }
}
