#[derive(Debug)]
enum AppMsg {
    ThemeInsert(CustomTheme),
    ThemeRemove(ThemeName),
    SetFont {
        font_family: Option<String>,
        font_face: Option<String>,
    },
}
