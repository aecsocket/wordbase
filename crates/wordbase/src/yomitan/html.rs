use core::fmt;

use crate::yomitan::structured::{StyledElement, UnstyledElement};

use super::structured::{Content, ContentStyle, Element, ImageElement, LinkElement, TableElement};

#[must_use]
pub fn to_html(content: &Content) -> String {
    let mut html = String::new();
    _ = write_html(&mut html, content);
    html
}

pub fn write_html(mut w: impl fmt::Write, content: &Content) -> fmt::Result {
    any(&mut w, content)
}

fn any(w: &mut impl fmt::Write, content: &Content) -> fmt::Result {
    match content {
        Content::String(s) => write!(w, "{s}"),
        Content::Content(children) => {
            for content in children {
                any(w, content)?;
            }
            Ok(())

            // write!(w, "<ul>")?;
            // for content in children {
            //     write!(w, "<li>")?;
            //     any(w, content)?;
            //     write!(w, "</li>")?;
            // }
            // write!(*w, "</ul>")
        }
        Content::Element(elem) => element(w, elem),
    }
}

#[rustfmt::skip]
fn element(w: &mut impl fmt::Write, elem: &Element) -> fmt::Result {
    match elem {
        Element::Br { data: _ } => write!(w, "</br>"),
        //
        Element::Ruby(e)  => unstyled(w, e, "ruby"),
        Element::Rt(e)    => unstyled(w, e, "rt"),
        Element::Rp(e)    => unstyled(w, e, "rp"),
        Element::Table(e) => unstyled(w, e, "table"),
        Element::Thead(e) => unstyled(w, e, "thead"),
        Element::Tbody(e) => unstyled(w, e, "tbody"),
        Element::Tfoot(e) => unstyled(w, e, "tfoot"),
        Element::Tr(e)    => unstyled(w, e, "tr"),
        //
        Element::Td(e) => table(w, e, "td"),
        Element::Th(e) => table(w, e, "th"),
        //
        Element::Span(e)    => styled(w, e, "span"),
        Element::Div(e)     => styled(w, e, "div"),
        Element::Ol(e)      => styled(w, e, "ol"),
        Element::Ul(e)      => styled(w, e, "ul"),
        Element::Li(e)      => styled(w, e, "li"),
        Element::Details(e) => styled(w, e, "details"),
        Element::Summary(e) => styled(w, e, "summary"),
        //
        Element::Img(e) => img(w, e),
        //
        Element::A(e) => link(w, e),
    }
}

macro_rules! forward_to_tag {
    ($w:expr, $field:expr, $prop:expr) => {
        if let Some(value) = &($field) {
            write!($w, " ")?;
            write!($w, $prop)?;
            write!($w, r#"="{value}""#)?;
        }
    };
}

macro_rules! forward_to_tag_fn {
    ($w:expr, $field:expr, $prop:expr, $f:ident) => {
        if let Some(value) = &($field) {
            write!($w, " ")?;
            write!($w, $prop)?;
            write!($w, "=\"")?;
            $f($w, value)?;
            write!($w, "\"")?;
        }
    };
}

fn unstyled(w: &mut impl fmt::Write, elem: &UnstyledElement, tag: &str) -> fmt::Result {
    write!(w, "<{tag}")?;
    forward_to_tag!(w, elem.lang, "lang");
    write!(w, ">")?;

    if let Some(content) = &elem.content {
        any(w, content)?;
    }

    write!(w, "</{tag}>")
}

fn table(w: &mut impl fmt::Write, elem: &TableElement, tag: &str) -> fmt::Result {
    write!(w, "<{tag}")?;
    forward_to_tag!(w, elem.col_span, "col-span");
    forward_to_tag!(w, elem.row_span, "row-span");
    forward_to_tag_fn!(w, elem.style, "style", style_css);
    forward_to_tag!(w, elem.lang, "lang");
    write!(w, ">")?;

    if let Some(content) = &elem.content {
        any(w, content)?;
    }

    write!(w, "</{tag}>")
}

fn styled(w: &mut impl fmt::Write, elem: &StyledElement, tag: &str) -> fmt::Result {
    write!(w, "<{tag}")?;
    forward_to_tag_fn!(w, elem.style, "style", style_css);
    forward_to_tag!(w, elem.title, "title");
    forward_to_tag!(w, elem.open, "open");
    forward_to_tag!(w, elem.lang, "lang");
    write!(w, ">")?;

    if let Some(content) = &elem.content {
        any(w, content)?;
    }

    write!(w, "</{tag}>")
}

#[expect(
    clippy::cognitive_complexity,
    reason = "macro invocations lead to internal cognitive complexity"
)]
fn style_css(w: &mut impl fmt::Write, s: &ContentStyle) -> fmt::Result {
    macro_rules! forward_to_css {
        ($field:expr, $prop:expr) => {
            if let Some(value) = &($field) {
                write!(w, $prop)?;
                write!(w, ":{value};")?;
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

fn img(w: &mut impl fmt::Write, elem: &ImageElement) -> fmt::Result {
    write!(w, "<img src={}", elem.base.path)?;
    // TODO
    write!(w, ">")
}

fn link(w: &mut impl fmt::Write, elem: &LinkElement) -> fmt::Result {
    write!(w, "<a href={}>", elem.href)?;

    if let Some(content) = &elem.content {
        any(w, content)?;
    }

    write!(w, "</a>")
}
