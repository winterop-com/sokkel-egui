//! How `en-US` collates text, as `localeCompare` orders it, with `numeric` or without: ICU's root
//! order for the characters the examples' texts hold. The dataviz, dhis2, fhir and models examples
//! carried its letters, accents, expansions and `numeric`; the studio example (after the notes
//! example) carried the wider set of marks, the letters folded from `FOLDS` and combining marks.

use std::cmp::Ordering;

/// Spaces, punctuation and symbols in the order `en-US` collates them, before digits and letters.
const MARKS: &str = "\t\u{b}\u{c}\r\n\u{85} \u{a0}_-\u{2013}\u{2014},;:!\u{a1}?\u{bf}.\u{2026}\u{b7}'\u{2018}\u{2019}\"\u{201c}\u{201d}\u{ab}\u{bb}()[]{}\u{a7}\u{b6}@*/\\&#%\u{2022}`\u{b4}^\u{af}\u{a8}\u{b8}\u{b0}\u{a9}\u{ae}\u{2190}\u{2192}+\u{b1}\u{f7}\u{d7}<=>\u{ac}|\u{a6}~\u{2713}\u{a4}\u{a2}$\u{a3}\u{a5}\u{20ac}\u{2122}";

/// A letter's base letters, its accent's weight and whether it is an expansion, for the accented
/// Latin letters a text in English meets; none for a letter of its own.
fn latin(c: char) -> Option<(&'static str, u32, bool)> {
    let lower = c.to_lowercase().next().unwrap_or(c);
    let (base, accent) = match lower {
        'á' => ("a", 1),
        'à' => ("a", 2),
        'â' => ("a", 3),
        'ä' => ("a", 4),
        'ã' => ("a", 5),
        'å' => ("a", 6),
        'ç' => ("c", 1),
        'é' => ("e", 1),
        'è' => ("e", 2),
        'ê' => ("e", 3),
        'ë' => ("e", 4),
        'í' => ("i", 1),
        'ì' => ("i", 2),
        'î' => ("i", 3),
        'ï' => ("i", 4),
        'ñ' => ("n", 5),
        'ó' => ("o", 1),
        'ò' => ("o", 2),
        'ô' => ("o", 3),
        'ö' => ("o", 4),
        'õ' => ("o", 5),
        'ø' => ("o", 7),
        'ú' => ("u", 1),
        'ù' => ("u", 2),
        'û' => ("u", 3),
        'ü' => ("u", 4),
        'ý' => ("y", 1),
        'ÿ' => ("y", 4),
        'æ' => return Some(("ae", 0, true)),
        'œ' => return Some(("oe", 0, true)),
        'ß' => return Some(("ss", 0, true)),
        _ => return folded(c).map(|plain| (plain, OTHER_ACCENT, false)),
    };
    Some((base, accent, false))
}

/// The accent weight of a mark `latin` does not name.
const OTHER_ACCENT: u32 = 8;

/// The Latin letters with a mark that `latin` does not name, each followed by the plain letter the
/// collator reads it as first.
const FOLDS: &str = "ÐdðdĀaāaĂaăaĄaąaĆcćcĈcĉcĊcċcČcčcĎdďdĐdđdĒeēeĔeĕeĖeėeĘeęeĚeěeĜgĝgĞgğgĠgġgĢgģgĤhĥhĦhħhĨiĩiĪiīiĬiĭiĮiįiİiĴjĵjĶkķkĹlĺlĻlļlĽlľlĿlŀlŁlłlŃnńnŅnņnŇnňnŌoōoŎoŏoŐoőoŔrŕrŖrŗrŘrřrŚsśsŜsŝsŞsşsŠsšsŢtţtŤtťtŨuũuŪuūuŬuŭuŮuůuŰuűuŲuųuŴwŵwŶyŷyŸyŹzźzŻzżzŽzžzſsƠoơoƯuưuǍaǎaǏiǐiǑoǒoǓuǔuǕuǖuǗuǘuǙuǚuǛuǜuǞaǟaǠaǡaǦgǧgǨkǩkǪoǫoǬoǭoǰjǴgǵgǸnǹnǺaǻaǾoǿoȀaȁaȂaȃaȄeȅeȆeȇeȈiȉiȊiȋiȌoȍoȎoȏoȐrȑrȒrȓrȔuȕuȖuȗuȘsșsȚtțtȞhȟhȦaȧaȨeȩeȪoȫoȬoȭoȮoȯoȰoȱoȲyȳy";

/// The plain letter a Latin letter with a mark is read as first, as a text of one letter, if
/// `FOLDS` holds it.
fn folded(c: char) -> Option<&'static str> {
    let mut pairs = FOLDS.char_indices();
    while let (Some((_, marked)), Some((plain, letter))) = (pairs.next(), pairs.next()) {
        if marked == c {
            return Some(&FOLDS[plain..plain + letter.len_utf8()]);
        }
    }
    None
}

/// The accent weight a combining mark gives the letter before it: the weight `latin` gives the
/// letter written with that mark at once, so the two forms of `é` weigh the same.
fn combining(c: char) -> u32 {
    match c {
        '\u{301}' | '\u{327}' => 1,
        '\u{300}' => 2,
        '\u{302}' => 3,
        '\u{308}' => 4,
        '\u{303}' => 5,
        '\u{30a}' => 6,
        _ => OTHER_ACCENT,
    }
}

/// One collation element: its primary weights, its accent and its case.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Element {
    primary: Vec<u32>,
    secondary: u32,
    tertiary: u32,
}

/// The groups the collator orders before one another: spaces and punctuation, other symbols,
/// digits, Latin letters, then every other letter.
const CLASS_MARK: u32 = 1;
const CLASS_SYMBOL: u32 = 2;
const CLASS_DIGIT: u32 = 3;
const CLASS_LETTER: u32 = 4;
const CLASS_OTHER: u32 = 5;

/// A text's collation elements: marks by their place in `MARKS`, digits by their value or, with
/// `numeric`, each run of them by the number it writes, letters by their base letter with the
/// accent and the case after, a combining mark as the accent of the letter before it; control
/// characters and soft hyphens are left out.
fn elements(text: &str, numeric: bool) -> Vec<Element> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut at = 0;
    while at < chars.len() {
        let c = chars[at];
        if ('\u{300}'..='\u{36f}').contains(&c) {
            if let Some(last) = out.last_mut().filter(|last: &&mut Element| last.secondary == 0) {
                last.secondary = combining(c);
            }
        } else if c == '\u{ad}' {
        } else if c.is_ascii_digit() {
            if numeric {
                let start = at;
                while at < chars.len() && chars[at].is_ascii_digit() {
                    at += 1;
                }
                let run: String = chars[start..at].iter().collect();
                let run = run.trim_start_matches('0');
                let mut primary = vec![CLASS_DIGIT, run.len() as u32];
                primary.extend(run.bytes().map(u32::from));
                out.push(Element {
                    primary,
                    secondary: 0,
                    tertiary: 0,
                });
                continue;
            }
            out.push(Element {
                primary: vec![CLASS_DIGIT, 1, u32::from(c)],
                secondary: 0,
                tertiary: 0,
            });
        } else if let Some(place) = MARKS.chars().position(|mark| mark == c) {
            out.push(Element {
                primary: vec![CLASS_MARK, place as u32],
                secondary: 0,
                tertiary: 0,
            });
        } else if c.is_control() {
        } else if let Some((base, accent, expansion)) = latin(c) {
            let upper = u32::from(c.is_uppercase());
            for letter in base.chars() {
                out.push(Element {
                    primary: vec![CLASS_LETTER, u32::from(letter)],
                    secondary: accent,
                    tertiary: upper + if expansion { 2 } else { 0 },
                });
            }
        } else if c.is_ascii_alphabetic() {
            out.push(Element {
                primary: vec![CLASS_LETTER, u32::from(c.to_ascii_lowercase())],
                secondary: 0,
                tertiary: u32::from(c.is_ascii_uppercase()),
            });
        } else if !c.is_alphabetic() {
            out.push(Element {
                primary: vec![CLASS_SYMBOL, u32::from(c)],
                secondary: 0,
                tertiary: 0,
            });
        } else {
            let lower = c.to_lowercase().next().unwrap_or(c);
            out.push(Element {
                primary: vec![CLASS_OTHER, u32::from(lower)],
                secondary: 0,
                tertiary: u32::from(lower != c),
            });
        }
        at += 1;
    }
    out
}

/// Two texts compared as `a.localeCompare(b, 'en-US', { numeric })` compares them: by their
/// letters, digits and marks first, then their accents, then their case, lowercase first; texts
/// alike at every level are equal.
pub fn locale_compare(a: &str, b: &str, numeric: bool) -> Ordering {
    let (a, b) = (elements(a, numeric), elements(b, numeric));
    let primary = |of: &[Element]| of.iter().map(|one| one.primary.clone()).collect::<Vec<_>>();
    primary(&a)
        .cmp(&primary(&b))
        .then_with(|| {
            let secondary = |of: &[Element]| of.iter().map(|one| one.secondary).collect::<Vec<_>>();
            secondary(&a).cmp(&secondary(&b))
        })
        .then_with(|| {
            let tertiary = |of: &[Element]| of.iter().map(|one| one.tertiary).collect::<Vec<_>>();
            tertiary(&a).cmp(&tertiary(&b))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn texts_are_ordered_as_locale_compare_orders_them() {
        let mut names = ["b", "B", "a10", "a2", "á", "à", "a", "-x", "ł", "z", "æ", "e\u{301}"];
        names.sort_by(|a, b| locale_compare(a, b, true));
        assert_eq!(
            names,
            ["-x", "a", "á", "à", "a2", "a10", "æ", "b", "B", "e\u{301}", "ł", "z"]
        );
        assert_eq!(locale_compare("e\u{301}", "é", false), Ordering::Equal);
        assert_eq!(locale_compare("a10", "a2", false), Ordering::Less);
        assert_eq!(locale_compare("\u{a7}1", "1", false), Ordering::Less);
    }
}
