use core::fmt::{self, Write as _};

use gtk::{gdk, pango, prelude::*};
use wordbase::yomitan::structured::{self, ContentStyle, FontStyle, FontWeight, TextAlign};

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
        structured::Content::Element(element) => match &**element {
            structured::Element::Br { data: _ } => None,
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
            | structured::Element::Summary(e) => e
                .content
                .as_ref()
                .and_then(|e| make(css, e))
                .inspect(|child| {
                    if let Some(style) = &e.style {
                        let css_class = format!("glossary-{}", random_css_class());
                        _ = write!(&mut *css, ".{css_class}{{");
                        _ = to_css(style, &mut *css);
                        _ = write!(&mut *css, "}}");
                        child.add_css_class(&css_class);
                    }
                }),
            structured::Element::Img(e) => None,
            structured::Element::A(e) => {
                let button = gtk::LinkButton::new(&e.href);
                if let Some(child) = e.content.as_ref().and_then(|e| make(css, e)) {
                    button.set_child(Some(&child));
                }
                Some(button.upcast())
            }
        }
        .map(Cast::upcast),
    }
}

fn random_css_class() -> String {
    let [b0, b1, b2, b3, b4, b5, b6, b7] = rand::random::<[u8; 8]>();
    format!("{b0:02x}{b1:02x}{b2:02x}{b3:02x}{b4:02x}{b5:02x}{b6:02x}{b7:02x}")
}

fn to_css(style: &ContentStyle, mut w: impl fmt::Write) -> Result<(), fmt::Error> {
    match &style.font_style {
        Some(FontStyle::Normal) => write!(w, "font-style: normal;")?,
        Some(FontStyle::Italic) => write!(w, "font-style: italic;")?,
        None => {}
    }

    match &style.font_weight {
        Some(FontWeight::Normal) => write!(w, "font-weight: normal;")?,
        Some(FontWeight::Bold) => write!(w, "font-weight: bold;")?,
        None => {}
    }

    if let Some(value) = &style.font_size {
        write!(w, "font-size: {value};")?;
    }

    Ok(())
}
