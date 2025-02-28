// <https://github.com/ripose-jp/Memento/blob/master/src/dict/deconjugator.h>

use std::sync::LazyLock;

pub struct Deconjugator;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ConjugationInfo {
    pub base: String,
    pub derivations: Vec<WordForm>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum WordForm {
    GodanVerb,
    IchidanVerb,
    SuruVerb,
    KuruVerb,
    IrregularVerb,
    Adjective,
    Negative,
    Past,
    Te,
    Conjunctive,
    Volitional,
    Passive,
    Causative,
    Imperative,
    Potential,
    PotentialPassive,
    Conditional,
    ImperativeNegative,
    Zaru,
    Zu,
    Nu,
    Neba,
    Tari,
    Shimau,
    Chau,
    Chimau,
    Polite,
    Tara,
    Tai,
    Nasai,
    Sugiru,
    Sou,
    E,
    Ba,
    Ki,
    Toku,
    ColloquialNegative,
    ProvisionalColloquialNegative,
    Continuous,
    Adverbial,
    Noun,
    Any,
    None,
}

impl WordForm {
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::GodanVerb | Self::IchidanVerb | Self::SuruVerb | Self::KuruVerb | Self::Adjective,
        )
    }
}

struct Rule {
    base: &'static str,
    conjugated: &'static str,
    base_form: WordForm,
    conjugated_form: WordForm,
}

macro_rules! add_rules {
    ($vec:expr,) => {};
    (
        $vec:expr,
        $base_form:ident $base:expr => $conjugated_form:ident $conjugated:expr
        $(, $($tail:tt)*)?
    ) => {
        $vec.push(Rule {
            base: $base,
            conjugated: $conjugated,
            base_form: WordForm::$base_form,
            conjugated_form: WordForm::$conjugated_form,
        });
        $(add_rules!($vec, $($tail)*))?
    };
    (
        $vec:expr,
        $base_form:ident override $override_base_form:ident $base:expr
        =>
        $conjugated_form:ident $conjugated:expr
        $(, $($tail:tt)*)?
    ) => {
        $vec.push(Rule {
            base: $base,
            conjugated: $conjugated,
            base_form: WordForm::$override_base_form,
            conjugated_form: WordForm::$conjugated_form,
        });
        $(add_rules!($vec, $($tail)*))?
    };
    (
        $vec:expr,
        $base_form:ident $base:expr
        =>
        $conjugated_form:ident override $override_conjugated_form:ident $conjugated:expr
        $(, $($tail:tt)*)?
    ) => {
        $vec.push(Rule {
            base: $base,
            conjugated: $conjugated,
            base_form: WordForm::$base_form,
            conjugated_form: WordForm::$override_conjugated_form,
        });
        $(add_rules!($vec, $($tail)*))?
    };
    (
        $vec:expr,
        $base_form:ident => * [
            $($(override $override_base_form:ident)? $base:expr => $conjugated_form:ident $conjugated:expr),* $(,)?
        ]
        $(, $($tail:tt)*)?
    ) => {
        $(add_rules!(
            $vec,
            $base_form $(override $override_base_form)? $base => $conjugated_form $conjugated
        );)*
        $(add_rules!($vec, $($tail)*))?
    };
    (
        $vec:expr,
        * => $conjugated_form:ident [
            $($base_form:ident $base:expr => $(override $override_conjugated_form:ident)? $conjugated:expr),* $(,)?
        ]
        $(, $($tail:tt)*)?
    ) => {
        $(add_rules!(
            $vec,
            $base_form $base => $conjugated_form $(override $override_conjugated_form)? $conjugated
        );)*
        $(add_rules!($vec, $($tail)*))?
    };
}

#[expect(
    clippy::vec_init_then_push,
    reason = "macros cannot generate individual elements in an array, \
              so we have to create and push into a `Vec` instead"
)]
static RULES: LazyLock<Vec<Rule>> = LazyLock::new(|| {
    let mut vec = Vec::new();
    add_rules! [
        vec,
        * => Negative [
            GodanVerb "る" => "らない",
            GodanVerb "う" => "わない",
            GodanVerb "つ" => "たない",
            GodanVerb "す" => "さない",
            GodanVerb "く" => "かない",
            GodanVerb "ぐ" => "がない",
            GodanVerb "ぶ" => "ばない",
            GodanVerb "む" => "まない",
            GodanVerb "ぬ" => "なない",
            IchidanVerb "る" => "ない",
            KuruVerb "くる" => "こない",
            KuruVerb "来る" => "来ない",
            SuruVerb "する" => "しない",
            SuruVerb "為る" => "為ない",
        ],
        * => Past [
            GodanVerb "る" => "った",
            GodanVerb "う" => "った",
            GodanVerb "つ" => "った",
            GodanVerb "す" => "した",
            GodanVerb "く" => "いた",
            GodanVerb "ぐ" => "いだ",
            GodanVerb "ぶ" => "んだ",
            GodanVerb "む" => "んだ",
            GodanVerb "ぬ" => "んだ",
            IchidanVerb "る" => "た",
            KuruVerb "くる" => "きた",
            KuruVerb "来る" => "来た",
            SuruVerb "する" => "した",
            SuruVerb "為る" => "為た",
            GodanVerb "行く" => "行った",
            GodanVerb "いく" => "いった",
            GodanVerb "問う" => "問うた",
            GodanVerb "とう" => "とうた",
            GodanVerb "請う" => "請うた",
            GodanVerb "こう" => "こうた",
        ],
        * => Te [
            GodanVerb "る" => "って",
            GodanVerb "う" => "って",
            GodanVerb "つ" => "って",
            GodanVerb "す" => "して",
            GodanVerb "く" => "いて",
            GodanVerb "ぐ" => "いで",
            GodanVerb "ぶ" => "んで",
            GodanVerb "ぬ" => "んで",
            GodanVerb "む" => "んで",
            IchidanVerb "る" => "て",
            KuruVerb "くる" => "きて",
            KuruVerb "来る" => "来て",
            SuruVerb "する" => "して",
            SuruVerb "為る" => "為て",
            GodanVerb "行く" => "行って",
            GodanVerb "いく" => "いって",
            GodanVerb "問う" => "問うて",
            GodanVerb "とう" => "とうて",
            GodanVerb "請う" => "請うて",
            GodanVerb "こう" => "こうて",
        ],
        * => Toku [
            GodanVerb "る" => "っとく",
            GodanVerb "う" => "っとく",
            GodanVerb "つ" => "っとく",
            GodanVerb "す" => "しとく",
            GodanVerb "く" => "いとく",
            GodanVerb "ぐ" => "いどく",
            GodanVerb "ぶ" => "んどく",
            GodanVerb "ぬ" => "んどく",
            GodanVerb "む" => "んどく",
            IchidanVerb "る" => "とく",
            KuruVerb "くる" => "きとく",
            KuruVerb "来る" => "来とく",
            SuruVerb "する" => "しとく",
            SuruVerb "為る" => "為とく",
            GodanVerb "行く" => "行っとく",
            GodanVerb "問う" => "問うとく",
            GodanVerb "請う" => "請うとく",
        ],
        * => Imperative [
            GodanVerb "る" => "れ",
            GodanVerb "う" => "え",
            GodanVerb "つ" => "て",
            GodanVerb "す" => "せ",
            GodanVerb "く" => "け",
            GodanVerb "ぐ" => "げ",
            GodanVerb "ぶ" => "べ",
            GodanVerb "む" => "め",
            GodanVerb "ぬ" => "ね",
            IchidanVerb "る" => "ろ",
            IchidanVerb "る" => "よ",
            KuruVerb "くる" => "こい",
            KuruVerb "来る" => "来い",
            SuruVerb "する" => "しろ",
            SuruVerb "為る" => "為ろ",
            SuruVerb "する" => "せよ",
            SuruVerb "為る" => "為よ",
        ],
        * => Volitional [
            GodanVerb "る" => "ろう",
            GodanVerb "う" => "おう",
            GodanVerb "つ" => "とう",
            GodanVerb "す" => "そう",
            GodanVerb "く" => "こう",
            GodanVerb "ぐ" => "ごう",
            GodanVerb "ぶ" => "ぼう",
            GodanVerb "む" => "もう",
            GodanVerb "ぬ" => "のう",
            IchidanVerb "る" => "よう",
            KuruVerb "くる" => "こよう",
            KuruVerb "来る" => "来よう",
            SuruVerb "する" => "しよう",
            SuruVerb "為る" => "為よう",
        ],
        * => Passive [
            GodanVerb "る" => "られる",
            GodanVerb "う" => "われる",
            GodanVerb "つ" => "たれる",
            GodanVerb "す" => "される",
            GodanVerb "く" => "かれる",
            GodanVerb "ぐ" => "がれる",
            GodanVerb "ぶ" => "ばれる",
            GodanVerb "む" => "まれる",
            GodanVerb "ぬ" => "なれる",
            IchidanVerb "る" => override PotentialPassive "られる",
            KuruVerb "くる" => override PotentialPassive "こられる",
            KuruVerb "来る" => override PotentialPassive "来られる",
            SuruVerb "する" => "される",
            SuruVerb "為る" => "為れる",
        ],
        * => Potential [
            GodanVerb "る" => "れる",
            GodanVerb "う" => "える",
            GodanVerb "つ" => "てる",
            GodanVerb "す" => "せる",
            GodanVerb "く" => "ける",
            GodanVerb "ぐ" => "げる",
            GodanVerb "ぶ" => "べる",
            GodanVerb "む" => "める",
            GodanVerb "ぬ" => "ねる",
            IchidanVerb "る" => "れる",
            KuruVerb "くる" => "これる",
            KuruVerb "来る" => "来れる",
            SuruVerb "する" => "できる",
        ],
        * => Causative [
            GodanVerb "る" => "らせる",
            GodanVerb "う" => "わせる",
            GodanVerb "つ" => "たせる",
            GodanVerb "す" => "させる",
            GodanVerb "く" => "かせる",
            GodanVerb "ぐ" => "がせる",
            GodanVerb "ぶ" => "ばせる",
            GodanVerb "む" => "ませる",
            GodanVerb "ぬ" => "なせる",
            IchidanVerb "る" => "させる",
            KuruVerb "くる" => "こさせる",
            KuruVerb "来る" => "来させる",
            SuruVerb "する" => "させる",
            SuruVerb "為る" => "為せる",
        ],
        * => Ba [
            GodanVerb "る" => "れば",
            GodanVerb "う" => "えば",
            GodanVerb "つ" => "てば",
            GodanVerb "す" => "せば",
            GodanVerb "く" => "けば",
            GodanVerb "ぐ" => "げば",
            GodanVerb "ぶ" => "べば",
            GodanVerb "む" => "めば",
            GodanVerb "ぬ" => "ねば",
            IchidanVerb "る" => "れば",
            KuruVerb "くる" => "くれば",
            KuruVerb "来る" => "来れば",
            SuruVerb "する" => "すれば",
            SuruVerb "為る" => "為れば",
        ],
        * => Zaru [
            GodanVerb "る" => "らざる",
            GodanVerb "う" => "わざる",
            GodanVerb "つ" => "たざる",
            GodanVerb "す" => "さざる",
            GodanVerb "く" => "かざる",
            GodanVerb "ぐ" => "がざる",
            GodanVerb "ぶ" => "ばざる",
            GodanVerb "む" => "まざる",
            GodanVerb "ぬ" => "なざる",
            IchidanVerb "る" => "ざる",
            KuruVerb "くる" => "こざる",
            KuruVerb "来る" => "来ざる",
            SuruVerb "する" => "せざる",
            SuruVerb "為る" => "為ざる",
        ],
        * => Neba [
            GodanVerb "る" => "らねば",
            GodanVerb "う" => "わねば",
            GodanVerb "つ" => "たねば",
            GodanVerb "す" => "さねば",
            GodanVerb "く" => "かねば",
            GodanVerb "ぐ" => "がねば",
            GodanVerb "ぶ" => "ばねば",
            GodanVerb "む" => "まねば",
            GodanVerb "ぬ" => "なねば",
            IchidanVerb "る" => "ねば",
            KuruVerb "くる" => "こねば",
            KuruVerb "来る" => "来ねば",
            SuruVerb "する" => "せねば",
            SuruVerb "為る" => "為ねば",
        ],
        * => Zu [
            GodanVerb "る" => "らず",
            GodanVerb "う" => "わず",
            GodanVerb "つ" => "たず",
            GodanVerb "す" => "さず",
            GodanVerb "く" => "かず",
            GodanVerb "ぐ" => "がず",
            GodanVerb "ぶ" => "ばず",
            GodanVerb "む" => "まず",
            GodanVerb "ぬ" => "なず",
            IchidanVerb "る" => "ず",
            KuruVerb "くる" => "こず",
            KuruVerb "来る" => "来ず",
            SuruVerb "する" => "せず",
            SuruVerb "為る" => "為ず",
        ],
        * => Nu [
            GodanVerb "る" => "らぬ",
            GodanVerb "う" => "わぬ",
            GodanVerb "つ" => "たぬ",
            GodanVerb "す" => "さぬ",
            GodanVerb "く" => "かぬ",
            GodanVerb "ぐ" => "がぬ",
            GodanVerb "ぶ" => "ばぬ",
            GodanVerb "む" => "まぬ",
            GodanVerb "ぬ" => "なぬ",
            IchidanVerb "る" => "ぬ",
            KuruVerb "くる" => "こぬ",
            KuruVerb "来る" => "来ぬ",
            SuruVerb "する" => "せぬ",
            SuruVerb "為る" => "為ぬ",
        ],
        * => ColloquialNegative [
            GodanVerb "る" => "らん",
            GodanVerb "う" => "わん",
            GodanVerb "つ" => "たん",
            GodanVerb "す" => "さん",
            GodanVerb "く" => "かん",
            GodanVerb "ぐ" => "がん",
            GodanVerb "ぶ" => "ばん",
            GodanVerb "む" => "まん",
            GodanVerb "ぬ" => "なん",
            IchidanVerb "る" => "ん",
            KuruVerb "くる" => "こん",
            KuruVerb "来る" => "来ん",
            SuruVerb "する" => "せん",
            SuruVerb "為る" => "為ん",
        ],
        * => ProvisionalColloquialNegative [
            GodanVerb "る" => "らなきゃ",
            GodanVerb "う" => "わなきゃ",
            GodanVerb "つ" => "たなきゃ",
            GodanVerb "す" => "さなきゃ",
            GodanVerb "く" => "かなきゃ",
            GodanVerb "ぐ" => "がなきゃ",
            GodanVerb "ぶ" => "ばなきゃ",
            GodanVerb "む" => "まなきゃ",
            GodanVerb "ぬ" => "ななきゃ",
            IchidanVerb "る" => "なきゃ",
            KuruVerb "くる" => "こなきゃ",
            KuruVerb "来る" => "来なきゃ",
            SuruVerb "する" => "しなきゃ",
            SuruVerb "為る" => "為なきゃ",
        ],
        * => ImperativeNegative [
            GodanVerb "る" => "るな",
            GodanVerb "う" => "うな",
            GodanVerb "つ" => "つな",
            GodanVerb "す" => "すな",
            GodanVerb "く" => "くな",
            GodanVerb "ぐ" => "ぐな",
            GodanVerb "ぶ" => "ぶな",
            GodanVerb "む" => "むな",
            GodanVerb "ぬ" => "ぬな",
            IchidanVerb "る" => "るな",
            KuruVerb "くる" => "くるな",
            KuruVerb "来る" => "来るな",
            SuruVerb "する" => "するな",
            SuruVerb "為る" => "為るな",
        ],
        * => Tari [
            GodanVerb "る" => "ったり",
            GodanVerb "う" => "ったり",
            GodanVerb "つ" => "ったり",
            GodanVerb "す" => "したり",
            GodanVerb "く" => "いたり",
            GodanVerb "ぐ" => "いだり",
            GodanVerb "ぶ" => "んだり",
            GodanVerb "む" => "んだり",
            GodanVerb "ぬ" => "んだり",
            IchidanVerb "る" => "たり",
            KuruVerb "くる" => "きたり",
            KuruVerb "来る" => "来たり",
            SuruVerb "する" => "したり",
            SuruVerb "為る" => "為たり",
            GodanVerb "行く" => "行ったり",
            GodanVerb "問う" => "問うたり",
            GodanVerb "請う" => "請うたり",
        ],
        * => Chau [
            GodanVerb "る" => "っちゃう",
            GodanVerb "う" => "っちゃう",
            GodanVerb "つ" => "っちゃう",
            GodanVerb "す" => "しちゃう",
            GodanVerb "く" => "いちゃう",
            GodanVerb "ぐ" => "いちゃう",
            GodanVerb "ぶ" => "んじゃう",
            GodanVerb "ぬ" => "んじゃう",
            GodanVerb "む" => "んじゃう",
            IchidanVerb "る" => "ちゃう",
            KuruVerb "くる" => "きちゃう",
            KuruVerb "来る" => "来ちゃう",
            SuruVerb "する" => "しちゃう",
            SuruVerb "為る" => "為ちゃう",
            GodanVerb "行く" => "行っちゃう",
            GodanVerb "問う" => "問うちゃう",
            GodanVerb "請う" => "請うちゃう",
        ],
        * => Chimau [
            GodanVerb "る" => "っちまう",
            GodanVerb "う" => "っちまう",
            GodanVerb "つ" => "っちまう",
            GodanVerb "す" => "しちまう",
            GodanVerb "く" => "いちまう",
            GodanVerb "ぐ" => "いちまう",
            GodanVerb "ぶ" => "んじまう",
            GodanVerb "ぬ" => "んじまう",
            GodanVerb "む" => "んじまう",
            IchidanVerb "る" => "ちまう",
            KuruVerb "くる" => "きちまう",
            KuruVerb "来る" => "来ちまう",
            SuruVerb "する" => "しちまう",
            SuruVerb "為る" => "為ちまう",
            GodanVerb "行く" => "行っちまう",
            GodanVerb "問う" => "問うちゃう",
            GodanVerb "請う" => "請うちゃう",
        ],
        * => Continuous [
            Te "で" => "でいる",
            Te "て" => "ている",
            Te "で" => "でおる",
            Te "て" => "ておる",
            Te "で" => "でる",
            Te "て" => "てる",
            Te "て" => "とる",
        ],
        * => Shimau [
            Te "で" => "でしまう",
            Te "て" => "てしまう",
        ],
        Adjective => * [
            "い" => Te "くて",
            "い" => Adverbial "く",
            "い" => Negative "くない",
            "い" => Past "かった",
            "い" => Ba "ければ",
            "い" => ProvisionalColloquialNegative "くなきゃ",
            "い" => Tara "かったら",
            "い" => Noun "さ",
            "い" => Sou "そう",
            "い" => Sugiru "すぎる",
            "い" => Ki "き",
            "い" => Volitional "かろう",
            "ない" => E "ねえ",
            "ない" => E "ねぇ",
            "ない" => E "ねー",
            "たい" => E "てえ",
            "たい" => E "てぇ",
            "たい" => E "てー",
        ],
        * => Conjunctive [
            GodanVerb "る" => "り",
            GodanVerb "う" => "い",
            GodanVerb "つ" => "ち",
            GodanVerb "す" => "し",
            GodanVerb "く" => "き",
            GodanVerb "ぐ" => "ぎ",
            GodanVerb "ぶ" => "び",
            GodanVerb "む" => "み",
            GodanVerb "ぬ" => "に",
            IchidanVerb "る" => "",
            KuruVerb "くる" => "き",
            KuruVerb "来る" => "来",
            SuruVerb "する" => "し",
            SuruVerb "為る" => "為",
        ],
        Conjunctive "" => Polite "ます",
        Polite "ます" => Negative "ません",
        Polite "ます" => Past "ました",
        Polite "ます" => Volitional "ましょう",
        Negative "せん" => Past "せんでした",
        Conjunctive "" => Tara "たら",
        Conjunctive "" => Tai "たい",
        Conjunctive "" => Nasai "なさい",
        Conjunctive "" => Sou "そう",
        Conjunctive "" => Sugiru "すぎる",
    ];
    vec
});

impl Deconjugator {
    pub fn deconjugate(&self, word: &str) -> impl Iterator<Item = ConjugationInfo> {
        RULES.iter().filter_map(|rule| {
            let current_form = WordForm::Any;
            if current_form != WordForm::Any && rule.conjugated_form != current_form {
                return None;
            }
            if !word.ends_with(rule.conjugated) {
                return None;
            }

            if rule.base_form.is_terminal() {
            } else {
            };
            todo!()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn foo() {
        println!(
            "{:?}",
            Deconjugator.deconjugate("します").collect::<Vec<_>>()
        );
    }
}
