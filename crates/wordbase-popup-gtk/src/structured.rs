use core::fmt::{self, Write as _};

use gtk::{gdk, pango, prelude::*};
use wordbase::yomitan::structured::{
    self, ContentStyle, FontStyle, FontWeight, StyledElement, TextAlign, VerticalAlign,
};

pub fn to_ui(display: gdk::Display, content: &structured::Content) -> Option<gtk::Widget> {
    let mut css = String::new();
    let widget = make(&mut css, content)?;

    let css_provider = gtk::CssProvider::new();
    css_provider.load_from_string(&css);
    gtk::style_context_add_provider_for_display(&display, &css_provider, 0);
    widget.connect_destroy(move |_| {
        gtk::style_context_remove_provider_for_display(&display, &css_provider);
    });

    Some(widget)
}

fn make(css: &mut String, content: &structured::Content) -> Option<gtk::Widget> {
    match content {
        structured::Content::String(text) => {
            let label = gtk::Label::new(Some(text));
            label.set_selectable(true);
            label.set_wrap(true);
            label.set_wrap_mode(pango::WrapMode::Word);
            label.set_halign(gtk::Align::Start);
            Some(label.upcast())
        }
        structured::Content::Content(children) => {
            let container = gtk::Box::new(gtk::Orientation::Vertical, 4);
            for child in children {
                if let Some(child) = make(css, child) {
                    container.append(&child);
                }
            }
            Some(container.upcast())
        }
        structured::Content::Element(elem) => match &**elem {
            structured::Element::Br { data: _ } => None,
            structured::Element::Table(e) => {
                let grid = gtk::Grid::new();
                if let Some(content) = &e.content {
                    let mut row = 0i32;
                    make_table(css, &grid, &mut row, content);
                }
                Some(grid.upcast())
            }
            structured::Element::Ruby(e)
            | structured::Element::Rt(e)
            | structured::Element::Rp(e)
            | structured::Element::Table(e)
            | structured::Element::Thead(e)
            | structured::Element::Tbody(e)
            | structured::Element::Tfoot(e)
            | structured::Element::Tr(e) => e.content.as_ref().and_then(|e| make(css, e)),
            structured::Element::Td(e) | structured::Element::Th(e) => {
                e.content.as_ref().and_then(|e| make(css, e))
            }
            structured::Element::Span(e)
            | structured::Element::Div(e)
            | structured::Element::Ol(e)
            | structured::Element::Ul(e)
            | structured::Element::Li(e)
            | structured::Element::Details(e)
            | structured::Element::Summary(e) => make_styled(css, e),
            structured::Element::Img(_) => {
                None // TODO
            }
            structured::Element::A(e) => {
                e.content.as_ref().and_then(|e| make(css, e)).map(|child| {
                    let button = gtk::LinkButton::new(&e.href);
                    button.set_child(Some(&child));
                    button.upcast()
                })
            }
        }
        .map(Cast::upcast),
    }
}

fn make_table(css: &mut String, grid: &gtk::Grid, row: &mut i32, content: &structured::Content) {
    match content {
        structured::Content::Content(children) => {
            for child in children {
                make_table(css, grid, row, child);
                *row = row.saturating_add(1);
            }
        }
        structured::Content::Element(elem) => {
            if let structured::Element::Tr(e) = &**elem {
                if let Some(content) = &e.content {
                    let mut col = 0i32;
                    make_table_row(css, grid, *row, &mut col, content);
                }
            }
        }
        structured::Content::String(_) => {}
    }
}

fn make_table_row(
    css: &mut String,
    grid: &gtk::Grid,
    row: i32,
    col: &mut i32,
    content: &structured::Content,
) {
    match content {
        structured::Content::Content(children) => {
            for child in children {
                make_table_row(css, grid, row, col, child);
                *col = col.saturating_add(1);
            }
        }
        structured::Content::Element(elem) => {
            if let structured::Element::Th(e) | structured::Element::Td(e) = &**elem {
                if let Some(child) = e.content.as_ref().and_then(|e| make(css, e)) {
                    if let (Ok(width), Ok(height)) = (
                        i32::try_from(e.col_span.unwrap_or(1)),
                        i32::try_from(e.row_span.unwrap_or(1)),
                    ) {
                        grid.attach(&child, *col, row, width, height);
                    }
                }
            }
        }
        structured::Content::String(_) => {}
    }
}

fn make_styled(css: &mut String, elem: &StyledElement) -> Option<gtk::Widget> {
    let child = elem.content.as_ref().and_then(|e| make(css, e))?;

    if let Some(value) = &elem.title {
        child.set_tooltip_text(Some(value));
    }

    if let Some(style) = &elem.style {
        let css_class = format!("glossary-{}", random_css_class());
        _ = write!(&mut *css, ".{css_class}{{");
        _ = to_css(style, &mut *css);
        _ = write!(&mut *css, "}}");
        child.add_css_class(&css_class);

        match style.vertical_align {
            Some(VerticalAlign::Top) => child.set_valign(gtk::Align::Start),
            Some(VerticalAlign::Middle) => child.set_valign(gtk::Align::Center),
            Some(VerticalAlign::Bottom) => child.set_valign(gtk::Align::End),
            _ => {}
        }

        let direction = gtk::Widget::default_direction();
        match style.text_align {
            Some(TextAlign::Start | TextAlign::Justify) => child.set_halign(gtk::Align::Start),
            Some(TextAlign::End) => child.set_halign(gtk::Align::End),
            Some(TextAlign::Left) => child.set_halign(match direction {
                gtk::TextDirection::Rtl => gtk::Align::End,
                _ => gtk::Align::Start,
            }),
            Some(TextAlign::Right) => child.set_halign(match direction {
                gtk::TextDirection::Rtl => gtk::Align::Start,
                _ => gtk::Align::End,
            }),
            Some(TextAlign::Center) => child.set_halign(gtk::Align::Center),
            None => {}
        }
    }

    Some(child)
}

fn random_css_class() -> String {
    let [b0, b1, b2, b3, b4, b5, b6, b7] = rand::random::<[u8; 8]>();
    format!("{b0:02x}{b1:02x}{b2:02x}{b3:02x}{b4:02x}{b5:02x}{b6:02x}{b7:02x}")
}

fn to_css(s: &ContentStyle, mut w: impl fmt::Write) -> Result<(), fmt::Error> {
    macro_rules! forward_to_css {
        ($writer:expr, $style:expr, $field:ident, $css_prop:expr) => {{
            if let Some(value) = &($style.$field) {
                write!($writer, $css_prop)?;
                write!($writer, ":{value};")?;
            }
        }};
    }

    forward_to_css!(w, s, font_style, "font-style");
    forward_to_css!(w, s, font_weight, "font-weight");
    forward_to_css!(w, s, font_size, "font-size");
    forward_to_css!(w, s, color, "color");
    forward_to_css!(w, s, background, "background");
    forward_to_css!(w, s, background_color, "background-color");

    // forward_to_css!(w, s, text_decoration_line, "text-decoration-line"); // implemented manually
    let text_decoration_line = s
        .text_decoration_line
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(" ");
    if !text_decoration_line.is_empty() {
        write!(w, "text-decoration-line:{text_decoration_line};")?;
    }

    forward_to_css!(w, s, text_decoration_style, "text-decoration-style");
    forward_to_css!(w, s, text_decoration_color, "text-decoration-color");
    forward_to_css!(w, s, border_color, "border-color");
    forward_to_css!(w, s, border_style, "border-style");
    forward_to_css!(w, s, border_radius, "border-radius");
    forward_to_css!(w, s, border_width, "border-width");
    // forward_to_css!(w, s, clip_path, "clip-path"); // unsupported
    // forward_to_css!(w, s, vertical_align, "vertical-align"); // implemented via code
    // forward_to_css!(w, s, text_align, "text-align"); // implemented via code
    // forward_to_css!(w, s, text_emphasis, "text-emphasis"); // unsupported
    forward_to_css!(w, s, text_shadow, "text-shadow");
    forward_to_css!(w, s, margin, "margin");
    forward_to_css!(w, s, margin_top, "margin-top");
    forward_to_css!(w, s, margin_left, "margin-left");
    forward_to_css!(w, s, margin_right, "margin-right");
    forward_to_css!(w, s, margin_bottom, "margin-bottom");
    forward_to_css!(w, s, padding, "padding");
    forward_to_css!(w, s, padding_top, "padding-top");
    forward_to_css!(w, s, padding_left, "padding-left");
    forward_to_css!(w, s, padding_right, "padding-right");
    forward_to_css!(w, s, padding_bottom, "padding-bottom");
    // forward_to_css!(w, s, word_break, "word-break"); // implemented via code
    // forward_to_css!(w, s, white_space, "white-space"); // unsupported
    // forward_to_css!(w, s, cursor, "cursor"); // unsupported
    // forward_to_css!(w, s, list_style_type, "list-style-type"); // unsupported

    Ok(())
}
