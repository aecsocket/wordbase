use core::fmt::{self, Write as _};

use gtk::{gdk, pango, prelude::*};
use webkit::prelude::WebViewExt;
use wordbase::yomitan::structured::{
    Content, ContentStyle, Element, StyledElement, TextAlign, VerticalAlign,
};

pub fn to_ui(display: gdk::Display, content: &Content) -> gtk::Widget {
    let webview = webkit::WebView::new();
    webview.set_height_request(200);
    webview.set_background_color(&gdk::RGBA::new(0.0, 0.0, 0.0, 0.0));

    webview.load_html(
        r#"
            <!DOCTYPE html>
        <html lang="en">
        <head>
            <meta charset="UTF-8">
            <meta name="viewport" content="width=device-width, initial-scale=1.0">
            <title>Transparent WebKit</title>
            <style>
                body {
                    background-color: transparent !important;
                    color: white; /* Ensure text is visible */
                }
            </style>
        </head>
        <body>
            <h1>Hello, World!</h1>
            <p>This HTML is loaded in a WebKit WebView with a transparent background.</p>
        </body>
        </html>
    "#,
        None,
    );

    // let mut css = String::new();
    // let container = gtk::Box::new(gtk::Orientation::Vertical, 0);
    // make(&mut css, content, &mut |child| container.append(&child));

    // let css_provider = gtk::CssProvider::new();
    // css_provider.load_from_string(&css);
    // gtk::style_context_add_provider_for_display(&display, &css_provider, 0);
    // container.connect_destroy(move |_| {
    //     gtk::style_context_remove_provider_for_display(&display, &css_provider);
    // })

    webview.upcast()
}

// internal iteration is a bit icky, but we'd need generators otherwise
// we need dynamic dispatch, because otherwise when generating types,
// we'd recurse infinitely
fn make(css: &mut String, content: &Content, append: &mut dyn FnMut(gtk::Widget)) {
    match content {
        Content::String(text) => {
            let label = gtk::Label::new(Some(text));
            label.set_selectable(true);
            label.set_wrap(true);
            label.set_wrap_mode(pango::WrapMode::Word);
            label.set_halign(gtk::Align::Start);
            append(label.upcast());
        }
        Content::Content(children) => {
            for content in children {
                make(css, content, append);
            }
        }
        Content::Element(elem) => match &**elem {
            Element::Br { data: _ } => {}
            Element::Rt(e) => {} // Ruby text that appears on top of kanji
            Element::Table(e) => {
                // TODO
            }
            Element::Div(e) => {
                let container = gtk::Box::new(gtk::Orientation::Vertical, 0);
                make_styled(css, e, &mut |child| container.append(&child));
                append(container.upcast());
            }
            Element::Span(e) => {
                let container = gtk::Box::new(gtk::Orientation::Horizontal, 0);
                make_styled(css, e, &mut |child| container.append(&child));
                append(container.upcast());
            }
            Element::Ruby(e)
            | Element::Rt(e)
            | Element::Rp(e)
            | Element::Table(e)
            | Element::Thead(e)
            | Element::Tbody(e)
            | Element::Tfoot(e)
            | Element::Tr(e) => make_opt(css, &e.content, append),
            Element::Td(e) | Element::Th(e) => make_opt(css, &e.content, append),
            Element::Span(e)
            | Element::Div(e)
            | Element::Ol(e)
            | Element::Ul(e)
            | Element::Li(e)
            | Element::Details(e)
            | Element::Summary(e) => {
                make_styled(css, e, append);
            }
            Element::Img(_) => {
                // TODO
            }
            Element::A(e) => {
                make_opt(css, &e.content, &mut |child| {
                    let container = gtk::LinkButton::new(&e.href);
                    container.set_child(Some(&child));
                    append(container.upcast());
                });
            }
        },
    }
}

fn make_opt(css: &mut String, content: &Option<Content>, append: &mut dyn FnMut(gtk::Widget)) {
    if let Some(content) = content {
        make(css, content, append);
    }
}

// fn make_into_grid(css: &mut String, grid: &gtk::Grid, row: &mut i32, content: &Content) {
//     match content {
//         Content::Content(children) => {
//             for content in children {
//                 make_into_grid(css, grid, row, content);
//                 *row = row.saturating_add(1);
//             }
//         }
//         Content::Element(elem) => {
//             if let Element::Tr(e) = &**elem {
//                 if let Some(content) = &e.content {
//                     let mut col = 0i32;
//                     make_into_table_row(css, grid, *row, &mut col, content);
//                 }
//             }
//         }
//         Content::String(_) => {}
//     }
// }

// fn make_into_table_row(
//     css: &mut String,
//     grid: &gtk::Grid,
//     row: i32,
//     col: &mut i32,
//     content: &Content,
// ) {
//     match content {
//         Content::Content(children) => {
//             for content in children {
//                 make_into_table_row(css, grid, row, col, content);
//                 *col = col.saturating_add(1);
//             }
//         }
//         Content::Element(elem) => {
//             if let Element::Th(e) | Element::Td(e) = &**elem {
//                 if let Some(child) = e.content.as_ref().and_then(|e| make(css, e)) {
//                     if let (Ok(width), Ok(height)) = (
//                         i32::try_from(e.col_span.unwrap_or(1)),
//                         i32::try_from(e.row_span.unwrap_or(1)),
//                     ) {
//                         grid.attach(&child, *col, row, width, height);
//                     }
//                 }
//             }
//         }
//         Content::String(_) => {}
//     }
// }

// fn make_into_box(css: &mut String, container: &gtk::Box, content: &Content) {
//     match content {
//         Content::Content(children) => {
//             for content in children {
//                 if let Some(child) = make(css, content) {
//                     container.append(&child);
//                 }
//             }
//         }
//         _ => {
//             if let Some(child) = make(css, content) {
//                 container.append(&child);
//             }
//         }
//     }
// }

fn make_styled(css: &mut String, elem: &StyledElement, append: &mut dyn FnMut(gtk::Widget)) {
    let css_class = elem.style.as_ref().map(|style| {
        let css_class = format!("glossary-{}", random_css_class());
        _ = write!(&mut *css, ".{css_class}{{");
        _ = to_css(style, &mut *css);
        _ = write!(&mut *css, "}}");
        css_class
    });

    make_opt(css, &elem.content, &mut |child| {
        if let Some(value) = &elem.title {
            child.set_tooltip_text(Some(value));
        }

        if let Some(css_class) = &css_class {
            child.add_css_class(css_class);
        }

        if let Some(style) = &elem.style {
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

        append(child);
    });
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
    // forward_to_css!(w, s, vertical_align, "vertical-align"); // implemented in code
    // forward_to_css!(w, s, text_align, "text-align"); // implemented in code
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
    // forward_to_css!(w, s, word_break, "word-break"); // unsupported
    // forward_to_css!(w, s, white_space, "white-space"); // unsupported
    // forward_to_css!(w, s, cursor, "cursor"); // unsupported
    // forward_to_css!(w, s, list_style_type, "list-style-type"); // unsupported

    Ok(())
}
