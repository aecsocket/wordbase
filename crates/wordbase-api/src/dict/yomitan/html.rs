use {
    super::structured::{Content, ContentStyle, Element},
    itertools::Itertools,
    maud::{Markup, PreEscaped, Render, html},
    std::fmt,
};

/// Renders structured content to HTML, also performing fixes to the HTML
/// structure.
#[must_use]
pub fn render_html(content: &Content) -> Markup {
    Rendering::new(content).render()
}

/// Wraps structured content types with extra context to correct some rendering
/// behavior.
///
/// Yomitan dictionary structured content is not guaranteed to be correct HTML.
/// In some cases, naively turning each content node into its corresponding
/// HTML node will result in invalid HTML, which the browser will try to
/// "fix", and break it in the process.
///
/// Yomitan, the extension, gets away with this because it uses JS to manipulate
/// the DOM directly (which will not be fixed by the browser), rather than
/// generating HTML. But since we generate HTML which is loaded by a browser or
/// a web view, we have to correct it first.
// TODO: if there is a good HTML DOM/XML document AST crate, we can generate an
// HTML AST first, then perform fixes, then render that to a string. It would be
// must cleaner than this approach we have now.
#[derive(Debug)]
struct Rendering<'t, T> {
    /// Element to render.
    t: &'t T,
    /// Whether we are currently inside an `<li>`.
    ///
    /// Some dictionaries, like [this Words.hk dictionary](https://github.com/MarvNC/wordshk-yomitan/)
    /// will place `<li>`s inside `<li>`s. This is illegal according to HTML, and
    /// the nested `<li>` must first be placed inside a `<ul>`.
    in_li: bool,
}

impl<'t, T> Rendering<'t, T> {
    fn new(t: &'t T) -> Self {
        Self { t, in_li: false }
    }

    fn wrap<'u, U>(&self, u: &'u U) -> Rendering<'u, U> {
        Rendering {
            t: u,
            in_li: self.in_li,
        }
    }

    fn in_li(self) -> Self {
        Self {
            in_li: true,
            ..self
        }
    }
}

impl Render for Rendering<'_, Content> {
    fn render(&self) -> Markup {
        html! {
            @match self.t {
                Content::String(text) => (newlines_to_brs(text)),
                Content::Content(children) => {
                    @for child in children {
                        (self.wrap(child))
                    }
                }
                Content::Element(elem) => (self.wrap(elem.as_ref())),
            }
        }
    }
}

fn newlines_to_brs(text: &str) -> Markup {
    PreEscaped(
        text.lines()
            .map(|line| html! { (line) }.0)
            .collect::<Vec<_>>()
            .join(&html! { br; }.0),
    )
}

impl Render for Rendering<'_, Element> {
    #[expect(
        clippy::cognitive_complexity,
        reason = "macros generate fake cognitive complexity"
    )]
    fn render(&self) -> Markup {
        // TODO: `data` support

        macro_rules! unstyled {
            ($elem:expr, $tag:ident) => {
                html! {
                    $tag  {
                        @if let Some(c) = &($elem.content) { (self.wrap(c)) }
                    }
                }
            };
        }

        macro_rules! table {
            ($elem:expr, $tag:ident) => {{
                let style = ($elem.style).as_ref().map(style_css);
                html! {
                    $tag style=[style] col-span=[$elem.col_span] row_span=[$elem.row_span] lang=[&($elem.lang)] {
                        @if let Some(c) = &($elem.content) { (self.wrap(c)) }
                    }
                }
            }};
        }

        macro_rules! styled {
            ($elem:expr, $tag:ident) => {{
                let style = ($elem.style).as_ref().map(style_css);
                html! {
                    $tag style=[style] title=[&($elem.title)] open=[$elem.open] lang=[&($elem.lang)] {
                        @if let Some(c) = &($elem.content) { (self.wrap(c)) }
                    }
                }
            }};
        }

        match self.t {
            Element::Br(_elem) => html! { br; },
            Element::Ruby(elem) => unstyled!(elem, ruby),
            Element::Rt(elem) => unstyled!(elem, rt),
            Element::Rp(elem) => unstyled!(elem, rp),
            Element::Table(elem) => unstyled!(elem, table),
            Element::Thead(elem) => unstyled!(elem, thead),
            Element::Tbody(elem) => unstyled!(elem, tbody),
            Element::Tfoot(elem) => unstyled!(elem, tfoot),
            Element::Tr(elem) => unstyled!(elem, tr),
            Element::Td(elem) => table!(elem, td),
            Element::Th(elem) => table!(elem, th),
            Element::Span(elem) => styled!(elem, span),
            Element::Div(elem) => styled!(elem, div),
            Element::Ol(elem) => styled!(elem, ol),
            Element::Ul(elem) => styled!(elem, ul),
            Element::Li(elem) => {
                let style = elem.style.as_ref().map(style_css);
                let body = html! {
                    li style=[style] title=[&(elem.title)] open=[elem.open] lang=[&(elem.lang)] {
                        @if let Some(c) = &(elem.content) { (self.wrap(c).in_li()) }
                    }
                };

                if self.in_li {
                    html! {
                        ul style="list-style-type: none; padding: 0; margin: 0" {
                            (body)
                        }
                    }
                } else {
                    html! { (body) }
                }
            }
            Element::Details(elem) => styled!(elem, details),
            Element::Summary(elem) => styled!(elem, summary),
            Element::Img(elem) => html! {
                img path=(elem.base.path)
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
                    collapsible=[elem.base.collapsible]
                    vertical-align=[elem.vertical_align]
                    border=[&(elem.border)]
                    border-radius=[&(elem.border_radius)]
                    size-units=[elem.size_units];
            },
            Element::A(elem) => html! {
                a href=(elem.href) lang=[&elem.lang] {
                    @if let Some(c) = &elem.content { (self.wrap(c)) }
                }
            },
        }
    }
}

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

    if !s.text_decoration_line.is_empty() {
        write!(
            w,
            "text-decoration-line:{};",
            s.text_decoration_line
                .iter()
                .map(ToString::to_string)
                .join(" ")
        )?;
    }

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

#[cfg(test)]
mod tests {
    use crate::dict::yomitan::structured::StyledElement;

    use super::*;

    #[test]
    fn style_escape() {
        let ul = Element::Ul(StyledElement {
            style: Some(ContentStyle {
                list_style_type: Some("\"X\"".into()),
                ..Default::default()
            }),
            ..Default::default()
        });
        let content = Content::Element(Box::new(ul));

        assert_eq!(
            r#"<ul style="list-style-type:&quot;X&quot;;"></ul>"#,
            render_html(&content).0
        );
    }

    #[test]
    fn li_in_li() {
        fn li(elem: StyledElement) -> Content {
            Content::Element(Box::new(Element::Li(elem)))
        }

        fn styled_with_content(content: Vec<Content>) -> StyledElement {
            StyledElement {
                content: Some(Content::Content(content)),
                ..Default::default()
            }
        }

        let ul = Element::Ul(styled_with_content(vec![
            li(styled_with_content(vec![li(StyledElement::default())])),
            li(StyledElement::default()),
        ]));
        let content = Content::Element(Box::new(ul));

        assert_eq!(
            r#"<ul><li><ul style="list-style-type: none; padding: 0; margin: 0"><li></li></ul></li><li></li></ul>"#,
            render_html(&content).0
        );
    }
}
