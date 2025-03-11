use {
    super::structured::{Content, ContentStyle, Element, UnstyledElement},
    maud::{Markup, Render, html},
    std::fmt,
};

impl Render for Content {
    fn render(&self) -> Markup {
        html! {
            @match self {
                Self::String(text) => (text),
                Self::Content(children) => {
                    @for child in children {
                        (child)
                    }
                }
                Self::Element(elem) => (elem),
            }
        }
    }
}

macro_rules! unstyled {
    ($elem:expr, $tag:ident) => {
        html! {
            $tag lang=[&($elem.lang)] {
                @if let Some(c) = &($elem.content) { (c) }
            }
        }
    };
}

macro_rules! table {
    ($elem:expr, $tag:ident) => {{
        let style = ($elem.style).as_ref().map(style_css);
        html! {
            $tag style=[style] col-span=[$elem.col_span] row_span=[$elem.row_span] lang=[&($elem.lang)] {
                @if let Some(c) = &($elem.content) { (c) }
            }
        }
    }};
}

macro_rules! styled {
    ($elem:expr, $tag:ident) => {{
        let style = ($elem.style).as_ref().map(style_css);
        html! {
            $tag style=[style] title=[&($elem.title)] open=[$elem.open] lang=[&($elem.lang)] {
                @if let Some(c) = &($elem.content) { (c) }
            }
        }
    }};
}

impl Render for Element {
    fn render(&self) -> Markup {
        match self {
            Self::Br(elem) => html! { br; },
            Self::Ruby(elem) => unstyled!(elem, ruby),
            Self::Rt(elem) => unstyled!(elem, rt),
            Self::Rp(elem) => unstyled!(elem, rp),
            Self::Table(elem) => unstyled!(elem, table),
            Self::Thead(elem) => unstyled!(elem, thead),
            Self::Tbody(elem) => unstyled!(elem, tbody),
            Self::Tfoot(elem) => unstyled!(elem, tfoot),
            Self::Tr(elem) => unstyled!(elem, tr),
            Self::Td(elem) => table!(elem, td),
            Self::Th(elem) => table!(elem, th),
            Self::Span(elem) => styled!(elem, span),
            Self::Div(elem) => styled!(elem, div),
            Self::Ol(elem) => styled!(elem, ol),
            Self::Ul(elem) => styled!(elem, ul),
            Self::Li(elem) => styled!(elem, li),
            Self::Details(elem) => styled!(elem, details),
            Self::Summary(elem) => styled!(elem, summary),
            Self::Img(elem) => html! {
                img
                    path=(elem.base.path)
                    width=[elem.base.width]
                    height=[elem.base.height]
                    preferred-width=[elem.base.preferred_width]
                    preferred-height=[elem.base.preferred_height]
                    title=[&elem.base.title]
                    alt=[&elem.base.alt]
                    description=[&elem.base.description]
                    pixelated=[elem.base.pixelated]
                    image-rendering=[elem.base.image_rendering]
                    image-appearance=[elem.base.image_appearance]
                    background=[elem.base.background]
                    collapsed=[elem.base.collapsed]
                    collapsible=[elem.base.collapsible];
            },
            Self::A(elem) => html! {
                a href=(elem.href) lang=[&elem.lang] {
                    @if let Some(c) = &elem.content { (c) }
                }
            },
        }
    }
}

impl Render for UnstyledElement {
    fn render(&self) -> Markup {
        html! {}
    }
}

// #[rustfmt::skip]
// fn element(w: &mut impl fmt::Write, elem: &Element) -> fmt::Result {
//     match elem {
//         Element::Br { data } => line_break(w, data.as_ref()),
//         //
//         Element::Ruby(e)  => unstyled(w, e, "ruby"),
//         Element::Rt(e)    => unstyled(w, e, "rt"),
//         Element::Rp(e)    => unstyled(w, e, "rp"),
//         Element::Table(e) => unstyled(w, e, "table"),
//         Element::Thead(e) => unstyled(w, e, "thead"),
//         Element::Tbody(e) => unstyled(w, e, "tbody"),
//         Element::Tfoot(e) => unstyled(w, e, "tfoot"),
//         Element::Tr(e)    => unstyled(w, e, "tr"),
//         //
//         Element::Td(e) => table(w, e, "td"),
//         Element::Th(e) => table(w, e, "th"),
//         //
//         Element::Span(e)    => styled(w, e, "span"),
//         Element::Div(e)     => styled(w, e, "div"),
//         Element::Ol(e)      => styled(w, e, "ol"),
//         Element::Ul(e)      => styled(w, e, "ul"),
//         Element::Li(e)      => styled(w, e, "li"),
//         Element::Details(e) => styled(w, e, "details"),
//         Element::Summary(e) => styled(w, e, "summary"),
//         //
//         Element::Img(e) => img(w, e),
//         //
//         Element::A(e) => link(w, e),
//     }
// }

// macro_rules! forward_to_tag {
//     ($w:expr, $field:expr, $prop:expr) => {
//         if let Some(value) = &($field) {
//             write!($w, " ")?;
//             write!($w, $prop)?;
//             let value = format!("{value}");
//             write!($w, "=\"{}\"", html_escape::encode_safe(&value))?;
//         }
//     };
// }

// macro_rules! forward_to_tag_fn {
//     ($w:expr, $field:expr, $prop:expr, $f:ident) => {
//         if let Some(value) = &($field) {
//             write!($w, " ")?;
//             write!($w, $prop)?;
//             write!($w, "=\"")?;
//             $f($w, value)?;
//             write!($w, "\"")?;
//         }
//     };
// }

// fn forward_data(w: &mut impl fmt::Write, data: Option<&Data>) -> fmt::Result {
//     let Some(data) = data else {
//         return Ok(());
//     };
//     for (key, value) in data.iter() {
//         let value = html_escape::encode_safe(value);
//         write!(w, " data-{key}=\"{value}\"")?;
//     }
//     Ok(())
// }

// fn line_break(w: &mut impl fmt::Write, data: Option<&Data>) -> fmt::Result {
//     write!(w, "<br")?;
//     forward_data(w, data)?;
//     write!(w, ">")
// }

// fn unstyled(w: &mut impl fmt::Write, elem: &UnstyledElement, tag: &str) -> fmt::Result {
//     write!(w, "<{tag}")?;
//     forward_data(w, elem.data.as_ref())?;
//     forward_to_tag!(w, elem.lang, "lang");
//     write!(w, ">")?;

//     if let Some(content) = &elem.content {
//         any(w, content)?;
//     }

//     write!(w, "</{tag}>")
// }

// fn table(w: &mut impl fmt::Write, elem: &TableElement, tag: &str) -> fmt::Result {
//     write!(w, "<{tag}")?;
//     forward_data(w, elem.data.as_ref())?;
//     forward_to_tag!(w, elem.col_span, "col-span");
//     forward_to_tag!(w, elem.row_span, "row-span");
//     forward_to_tag_fn!(w, elem.style, "style", style_css);
//     forward_to_tag!(w, elem.lang, "lang");
//     write!(w, ">")?;

//     if let Some(content) = &elem.content {
//         any(w, content)?;
//     }

//     write!(w, "</{tag}>")
// }

// fn styled(w: &mut impl fmt::Write, elem: &StyledElement, tag: &str) -> fmt::Result {
//     write!(w, "<{tag}")?;
//     forward_data(w, elem.data.as_ref())?;
//     forward_to_tag_fn!(w, elem.style, "style", style_css);
//     forward_to_tag!(w, elem.title, "title");
//     forward_to_tag!(w, elem.open, "open");
//     forward_to_tag!(w, elem.lang, "lang");
//     write!(w, ">")?;

//     if let Some(content) = &elem.content {
//         any(w, content)?;
//     }

//     write!(w, "</{tag}>")
// }

fn style_css(s: &ContentStyle) -> String {
    let mut css = String::new();
    _ = write_style_css(&mut css, s);
    css
}

#[expect(
    clippy::cognitive_complexity,
    reason = "macro invocations lead to internal cognitive complexity"
)]
fn write_style_css(w: &mut impl fmt::Write, s: &ContentStyle) -> fmt::Result {
    macro_rules! forward_to_css {
        ($field:expr, $prop:expr) => {
            if let Some(value) = &($field) {
                write!(w, $prop)?;
                write!(w, ":")?;
                let value = format!("{value}");
                write!(w, "{}", html_escape::encode_safe(&value))?;
                write!(w, ";")?;
            }
        };
    }

    forward_to_css!(s.font_style, "font-style");
    forward_to_css!(s.font_weight, "font-weight");
    forward_to_css!(s.font_size, "font-size");
    forward_to_css!(s.color, "color");
    forward_to_css!(s.background, "background");
    forward_to_css!(s.background_color, "background-color");
    // forward_to_css!(s.text_decoration_line, "text-decoration-line"); // TODO
    forward_to_css!(s.text_decoration_style, "text-decoration-style");
    forward_to_css!(s.text_decoration_color, "text-decoration-color");
    forward_to_css!(s.border_color, "border-color");
    forward_to_css!(s.border_style, "border-style");
    forward_to_css!(s.border_radius, "border-radius");
    forward_to_css!(s.border_width, "border-width");
    forward_to_css!(s.clip_path, "clip-path");
    forward_to_css!(s.vertical_align, "vertical-align");
    forward_to_css!(s.text_align, "text-align");
    forward_to_css!(s.text_emphasis, "text-emphasis");
    forward_to_css!(s.text_shadow, "text-shadow");
    forward_to_css!(s.margin, "margin");
    forward_to_css!(s.margin_top, "margin-top");
    forward_to_css!(s.margin_left, "margin-left");
    forward_to_css!(s.margin_right, "margin-right");
    forward_to_css!(s.margin_bottom, "margin-bottom");
    forward_to_css!(s.padding, "padding");
    forward_to_css!(s.padding_top, "padding-top");
    forward_to_css!(s.padding_left, "padding-left");
    forward_to_css!(s.padding_right, "padding-right");
    forward_to_css!(s.padding_bottom, "padding-bottom");
    forward_to_css!(s.word_break, "word-break");
    forward_to_css!(s.white_space, "white-space");
    forward_to_css!(s.cursor, "cursor");
    forward_to_css!(s.list_style_type, "list-style-type");

    Ok(())
}

// fn img(w: &mut impl fmt::Write, elem: &ImageElement) -> fmt::Result {
//     write!(w, "<img src={}", elem.base.path)?;
//     // TODO
//     write!(w, ">")
// }

// fn link(w: &mut impl fmt::Write, elem: &LinkElement) -> fmt::Result {
//     write!(w, "<a href={}>", elem.href)?;

//     if let Some(content) = &elem.content {
//         any(w, content)?;
//     }

//     write!(w, "</a>")
// }
