//! A small backtracking regular expression, as JavaScript reads one, for a schema's `pattern` and
//! the find bar's regexp: literals, `.`, classes, `\d \w \s \b` and their negations, anchors,
//! groups, alternation and the greedy and lazy quantifiers. Each test is given a step budget, as
//! sokkel-ui's guarded patterns are, so a pattern that would take long answers "unknown".

use std::cell::Cell;

#[derive(Debug, Clone)]
enum Node {
    Char(char),
    Any,
    Class(Class),
    Start,
    End,
    WordBoundary(bool),
    Group(Vec<Vec<Node>>),
    Repeat(Box<Node>, u32, Option<u32>, bool),
}

#[derive(Debug, Clone)]
struct Class {
    negated: bool,
    items: Vec<ClassItem>,
}

#[derive(Debug, Clone)]
enum ClassItem {
    Range(char, char),
    Digit(bool),
    Word(bool),
    Space(bool),
}

/// A compiled pattern.
#[derive(Debug, Clone)]
pub(crate) struct Regex {
    alternatives: Vec<Vec<Node>>,
    ignore_case: bool,
}

/// How many steps one test may take before it gives up.
const BUDGET: u32 = 200_000;

impl Regex {
    /// The pattern compiled, or `None` for one this engine does not read.
    pub(crate) fn new(pattern: &str, ignore_case: bool) -> Option<Self> {
        let chars: Vec<char> = pattern.chars().collect();
        let mut at = 0;
        let alternatives = parse_alternatives(&chars, &mut at)?;
        if at != chars.len() {
            return None;
        }
        Some(Self {
            alternatives,
            ignore_case,
        })
    }

    /// Whether the pattern matches anywhere in `text`; `None` when the test ran out of steps.
    pub(crate) fn is_match(&self, text: &str) -> Option<bool> {
        let chars: Vec<char> = text.chars().collect();
        let matcher = Matcher {
            text: &chars,
            ignore_case: self.ignore_case,
            steps: Cell::new(0),
        };
        for start in 0..=chars.len() {
            if matcher.alternatives(&self.alternatives, start, &mut |_| true) {
                return Some(true);
            }
            if matcher.steps.get() > BUDGET {
                return None;
            }
        }
        Some(false)
    }

    /// Every match in `text` that takes at least one character, as byte ranges, leftmost first.
    pub(crate) fn find_all(&self, text: &str) -> Vec<(usize, usize)> {
        let chars: Vec<char> = text.chars().collect();
        let mut bytes = Vec::with_capacity(chars.len() + 1);
        let mut offset = 0;
        for c in &chars {
            bytes.push(offset);
            offset += c.len_utf8();
        }
        bytes.push(offset);
        let matcher = Matcher {
            text: &chars,
            ignore_case: self.ignore_case,
            steps: Cell::new(0),
        };
        let mut found = Vec::new();
        let mut start = 0;
        while start <= chars.len() {
            matcher.steps.set(0);
            let mut end = None;
            matcher.alternatives(&self.alternatives, start, &mut |p| {
                end = Some(p);
                true
            });
            match end {
                Some(end) if end > start => {
                    found.push((bytes[start], bytes[end]));
                    start = end;
                }
                _ => start += 1,
            }
        }
        found
    }
}

fn parse_alternatives(chars: &[char], at: &mut usize) -> Option<Vec<Vec<Node>>> {
    let mut alternatives = vec![Vec::new()];
    while *at < chars.len() {
        let c = chars[*at];
        match c {
            ')' => break,
            '|' => {
                *at += 1;
                alternatives.push(Vec::new());
            }
            _ => {
                let atom = parse_atom(chars, at)?;
                let atom = parse_quantifier(chars, at, atom)?;
                alternatives.last_mut()?.push(atom);
            }
        }
    }
    Some(alternatives)
}

fn parse_atom(chars: &[char], at: &mut usize) -> Option<Node> {
    let c = chars[*at];
    *at += 1;
    Some(match c {
        '.' => Node::Any,
        '^' => Node::Start,
        '$' => Node::End,
        '(' => {
            if chars.get(*at) == Some(&'?') {
                if chars.get(*at + 1) == Some(&':') {
                    *at += 2;
                } else {
                    return None;
                }
            }
            let inner = parse_alternatives(chars, at)?;
            if chars.get(*at) != Some(&')') {
                return None;
            }
            *at += 1;
            Node::Group(inner)
        }
        '[' => Node::Class(parse_class(chars, at)?),
        '\\' => parse_escape(chars, at)?,
        '*' | '+' | '?' => return None,
        c => Node::Char(c),
    })
}

fn parse_escape(chars: &[char], at: &mut usize) -> Option<Node> {
    let c = *chars.get(*at)?;
    *at += 1;
    let class = |item| {
        Node::Class(Class {
            negated: false,
            items: vec![item],
        })
    };
    Some(match c {
        'd' => class(ClassItem::Digit(true)),
        'D' => class(ClassItem::Digit(false)),
        'w' => class(ClassItem::Word(true)),
        'W' => class(ClassItem::Word(false)),
        's' => class(ClassItem::Space(true)),
        'S' => class(ClassItem::Space(false)),
        'b' => Node::WordBoundary(true),
        'B' => Node::WordBoundary(false),
        'p' | 'P' | 'k' => return None,
        other => Node::Char(escaped_char(other, chars, at)?),
    })
}

fn escaped_char(c: char, chars: &[char], at: &mut usize) -> Option<char> {
    Some(match c {
        'n' => '\n',
        't' => '\t',
        'r' => '\r',
        'f' => '\u{c}',
        'v' => '\u{b}',
        '0' => '\0',
        'u' => {
            let hex: String = chars.get(*at..*at + 4)?.iter().collect();
            *at += 4;
            char::from_u32(u32::from_str_radix(&hex, 16).ok()?)?
        }
        'x' => {
            let hex: String = chars.get(*at..*at + 2)?.iter().collect();
            *at += 2;
            char::from_u32(u32::from_str_radix(&hex, 16).ok()?)?
        }
        c if c.is_ascii_digit() => return None,
        c => c,
    })
}

fn parse_class(chars: &[char], at: &mut usize) -> Option<Class> {
    let mut class = Class {
        negated: false,
        items: Vec::new(),
    };
    if chars.get(*at) == Some(&'^') {
        class.negated = true;
        *at += 1;
    }
    let mut first = true;
    loop {
        let c = *chars.get(*at)?;
        *at += 1;
        if c == ']' && !first {
            break;
        }
        first = false;
        let low = if c == '\\' {
            let e = *chars.get(*at)?;
            *at += 1;
            match e {
                'd' => {
                    class.items.push(ClassItem::Digit(true));
                    continue;
                }
                'D' => {
                    class.items.push(ClassItem::Digit(false));
                    continue;
                }
                'w' => {
                    class.items.push(ClassItem::Word(true));
                    continue;
                }
                'W' => {
                    class.items.push(ClassItem::Word(false));
                    continue;
                }
                's' => {
                    class.items.push(ClassItem::Space(true));
                    continue;
                }
                'S' => {
                    class.items.push(ClassItem::Space(false));
                    continue;
                }
                'b' => '\u{8}',
                e => escaped_char(e, chars, at)?,
            }
        } else {
            c
        };
        if chars.get(*at) == Some(&'-') && chars.get(*at + 1).is_some_and(|c| *c != ']') {
            *at += 1;
            let mut high = *chars.get(*at)?;
            *at += 1;
            if high == '\\' {
                let e = *chars.get(*at)?;
                *at += 1;
                high = escaped_char(e, chars, at)?;
            }
            class.items.push(ClassItem::Range(low, high));
        } else {
            class.items.push(ClassItem::Range(low, low));
        }
    }
    Some(class)
}

fn parse_quantifier(chars: &[char], at: &mut usize, atom: Node) -> Option<Node> {
    let (min, max) = match chars.get(*at) {
        Some('*') => {
            *at += 1;
            (0, None)
        }
        Some('+') => {
            *at += 1;
            (1, None)
        }
        Some('?') => {
            *at += 1;
            (0, Some(1))
        }
        Some('{') => {
            let close = chars[*at..].iter().position(|c| *c == '}').map(|p| *at + p);
            let Some(close) = close else {
                return Some(atom);
            };
            let inside: String = chars[*at + 1..close].iter().collect();
            let (min, max) = match inside.split_once(',') {
                None => {
                    let n = inside.trim().parse().ok()?;
                    (n, Some(n))
                }
                Some((a, b)) => {
                    let a = a.trim().parse().ok()?;
                    let b = if b.trim().is_empty() {
                        None
                    } else {
                        Some(b.trim().parse().ok()?)
                    };
                    (a, b)
                }
            };
            *at = close + 1;
            (min, max)
        }
        _ => return Some(atom),
    };
    if matches!(atom, Node::Start | Node::End | Node::WordBoundary(_)) {
        return None;
    }
    let greedy = if chars.get(*at) == Some(&'?') {
        *at += 1;
        false
    } else {
        true
    };
    Some(Node::Repeat(Box::new(atom), min, max, greedy))
}

struct Matcher<'t> {
    text: &'t [char],
    ignore_case: bool,
    steps: Cell<u32>,
}

fn is_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn is_space(c: char) -> bool {
    c.is_whitespace() || c == '\u{feff}'
}

impl Matcher<'_> {
    fn spent(&self) -> bool {
        let steps = self.steps.get() + 1;
        self.steps.set(steps);
        steps > BUDGET
    }

    fn same(&self, a: char, b: char) -> bool {
        a == b || (self.ignore_case && a.to_lowercase().eq(b.to_lowercase()))
    }

    fn in_class(&self, class: &Class, c: char) -> bool {
        let lower = c.to_lowercase().next().unwrap_or(c);
        let upper = c.to_uppercase().next().unwrap_or(c);
        let hit = class.items.iter().any(|item| match *item {
            ClassItem::Range(low, high) => {
                (low..=high).contains(&c)
                    || (self.ignore_case && ((low..=high).contains(&lower) || (low..=high).contains(&upper)))
            }
            ClassItem::Digit(yes) => c.is_ascii_digit() == yes,
            ClassItem::Word(yes) => is_word(c) == yes,
            ClassItem::Space(yes) => is_space(c) == yes,
        });
        hit != class.negated
    }

    fn alternatives(&self, alternatives: &[Vec<Node>], pos: usize, k: &mut dyn FnMut(usize) -> bool) -> bool {
        for branch in alternatives {
            if self.sequence(branch, pos, k) {
                return true;
            }
            if self.steps.get() > BUDGET {
                return false;
            }
        }
        false
    }

    fn sequence(&self, seq: &[Node], pos: usize, k: &mut dyn FnMut(usize) -> bool) -> bool {
        if self.spent() {
            return false;
        }
        match seq.split_first() {
            None => k(pos),
            Some((node, rest)) => self.node(node, pos, &mut |p| self.sequence(rest, p, k)),
        }
    }

    fn node(&self, node: &Node, pos: usize, k: &mut dyn FnMut(usize) -> bool) -> bool {
        let here = self.text.get(pos).copied();
        match node {
            Node::Char(c) => here.is_some_and(|h| self.same(h, *c)) && k(pos + 1),
            Node::Any => here.is_some_and(|h| h != '\n' && h != '\r') && k(pos + 1),
            Node::Class(class) => here.is_some_and(|h| self.in_class(class, h)) && k(pos + 1),
            Node::Start => pos == 0 && k(pos),
            Node::End => pos == self.text.len() && k(pos),
            Node::WordBoundary(yes) => {
                let before = pos > 0 && is_word(self.text[pos - 1]);
                let after = here.is_some_and(is_word);
                ((before != after) == *yes) && k(pos)
            }
            Node::Group(alternatives) => self.alternatives(alternatives, pos, k),
            Node::Repeat(inner, min, max, greedy) => self.repeat(inner, *min, *max, *greedy, 0, pos, k),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn repeat(
        &self,
        inner: &Node,
        min: u32,
        max: Option<u32>,
        greedy: bool,
        count: u32,
        pos: usize,
        k: &mut dyn FnMut(usize) -> bool,
    ) -> bool {
        if self.spent() {
            return false;
        }
        let can_more = max.is_none_or(|m| count < m);
        if count < min {
            return can_more
                && self.node(inner, pos, &mut |p| {
                    self.repeat(inner, min, max, greedy, count + 1, p, k)
                });
        }
        if greedy {
            let more = can_more
                && self.node(inner, pos, &mut |p| {
                    p != pos && self.repeat(inner, min, max, greedy, count + 1, p, k)
                });
            more || k(pos)
        } else {
            if k(pos) {
                return true;
            }
            can_more
                && self.node(inner, pos, &mut |p| {
                    p != pos && self.repeat(inner, min, max, greedy, count + 1, p, k)
                })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Regex;

    fn test(pattern: &str, text: &str) -> bool {
        Regex::new(pattern, false).and_then(|r| r.is_match(text)).unwrap()
    }

    #[test]
    fn the_studio_s_patterns_read_as_javascript_reads_them() {
        assert!(test("^[a-z][a-z0-9-]*$", "nightly-export"));
        assert!(!test("^[a-z][a-z0-9-]*$", "Nightly"));
        assert!(test("^\\S+( \\S+){4}$", "0 2 * * *"));
        assert!(!test("^\\S+( \\S+){4}$", "0 2 * *"));
        assert!(test("^[0-9]+(s|m|h)$", "10m"));
        assert!(!test("^[0-9]+(s|m|h)$", "10d"));
        assert!(test("^https://", "https://example.org"));
        assert!(test("a.*?b", "xxaxxbyy"));
    }

    #[test]
    fn find_all_gives_byte_ranges() {
        let r = Regex::new("o+", true).unwrap();
        assert_eq!(r.find_all("fOo bar oo"), vec![(1, 3), (8, 10)]);
    }
}
