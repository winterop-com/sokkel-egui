//! What `d2:validatePattern` leans on the browser for: `new RegExp(`^(?:${pattern})$`).test(text)`.
//! A backtracking reading of JavaScript's regular expressions without flags, as far as a pattern a
//! program rule checks a field with goes: literals and escapes, classes with ranges, `.`, groups
//! that capture and those that do not, back references, alternation, lookaheads, the anchors and
//! word boundaries, and greedy and lazy quantifiers. A pattern the browser refuses (an unclosed
//! group or class, a quantifier with nothing to repeat, `{2,1}`, a range out of order, a `\` at
//! the end) is refused here too.

use std::cell::RefCell;

#[derive(Clone, Debug)]
enum Class {
    Digit,
    Word,
    Space,
}

#[derive(Clone, Debug)]
enum Item {
    Char(char),
    Range(char, char),
    Class(Class, bool),
}

#[derive(Clone, Debug)]
enum Node {
    Char(char),
    Any,
    Set(Vec<Item>, bool),
    Class(Class, bool),
    Start,
    End,
    Boundary(bool),
    Group(Box<Node>, Option<usize>),
    Look(Box<Node>, bool),
    Back(usize),
    Repeat(Box<Node>, usize, Option<usize>, bool),
    Concat(Vec<Node>),
    Alt(Vec<Node>),
}

struct Parser {
    chars: Vec<char>,
    at: usize,
    groups: usize,
}

fn is_space(c: char) -> bool {
    crate::js::is_space(c)
}

fn is_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn in_class(class: &Class, c: char) -> bool {
    match class {
        Class::Digit => c.is_ascii_digit(),
        Class::Word => is_word(c),
        Class::Space => is_space(c),
    }
}

impl Parser {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.at).copied()
    }

    fn disjunction(&mut self) -> Result<Node, ()> {
        let mut options = vec![self.alternative()?];
        while self.peek() == Some('|') {
            self.at += 1;
            options.push(self.alternative()?);
        }
        Ok(if options.len() == 1 {
            options.remove(0)
        } else {
            Node::Alt(options)
        })
    }

    fn alternative(&mut self) -> Result<Node, ()> {
        let mut terms = Vec::new();
        while let Some(c) = self.peek() {
            if c == '|' || c == ')' {
                break;
            }
            terms.push(self.term()?);
        }
        Ok(Node::Concat(terms))
    }

    /// A `{n}`, `{n,}` or `{n,m}` at the parser's place, without moving it; none when the brace
    /// starts no quantifier, which leaves it a literal.
    fn braces(&self) -> Option<(usize, Option<usize>, usize)> {
        let mut at = self.at + 1;
        let digits = |at: &mut usize| {
            let start = *at;
            while self.chars.get(*at).is_some_and(char::is_ascii_digit) {
                *at += 1;
            }
            (start != *at).then(|| {
                self.chars[start..*at]
                    .iter()
                    .collect::<String>()
                    .parse::<usize>()
                    .unwrap_or(usize::MAX)
            })
        };
        let min = digits(&mut at)?;
        let max = if self.chars.get(at) == Some(&',') {
            at += 1;
            digits(&mut at)
        } else {
            Some(min)
        };
        (self.chars.get(at) == Some(&'}')).then_some((min, max, at + 1))
    }

    fn quantifier(&mut self) -> Result<Option<(usize, Option<usize>)>, ()> {
        let found = match self.peek() {
            Some('*') => {
                self.at += 1;
                Some((0, None))
            }
            Some('+') => {
                self.at += 1;
                Some((1, None))
            }
            Some('?') => {
                self.at += 1;
                Some((0, Some(1)))
            }
            Some('{') => match self.braces() {
                Some((min, max, next)) => {
                    if max.is_some_and(|max| max < min) {
                        return Err(());
                    }
                    self.at = next;
                    Some((min, max))
                }
                None => None,
            },
            _ => None,
        };
        Ok(found)
    }

    fn term(&mut self) -> Result<Node, ()> {
        let c = self.peek().ok_or(())?;
        let (atom, quantifiable) = match c {
            '^' => {
                self.at += 1;
                (Node::Start, false)
            }
            '$' => {
                self.at += 1;
                (Node::End, false)
            }
            '*' | '+' | '?' => return Err(()),
            '{' if self.braces().is_some() => return Err(()),
            '(' => {
                self.at += 1;
                let rest: String = self.chars[self.at..].iter().take(2).collect();
                if rest == "?:" {
                    self.at += 2;
                    let inner = self.disjunction()?;
                    self.close()?;
                    (Node::Group(Box::new(inner), None), true)
                } else if rest == "?=" || rest == "?!" {
                    self.at += 2;
                    let inner = self.disjunction()?;
                    self.close()?;
                    (Node::Look(Box::new(inner), rest == "?="), true)
                } else {
                    self.groups += 1;
                    let index = self.groups;
                    let inner = self.disjunction()?;
                    self.close()?;
                    (Node::Group(Box::new(inner), Some(index)), true)
                }
            }
            '.' => {
                self.at += 1;
                (Node::Any, true)
            }
            '[' => {
                self.at += 1;
                (self.set()?, true)
            }
            '\\' => {
                self.at += 1;
                match self.peek() {
                    Some('b') => {
                        self.at += 1;
                        (Node::Boundary(true), false)
                    }
                    Some('B') => {
                        self.at += 1;
                        (Node::Boundary(false), false)
                    }
                    Some(d) if d.is_ascii_digit() && d != '0' => {
                        let start = self.at;
                        while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                            self.at += 1;
                        }
                        let number: usize = self.chars[start..self.at]
                            .iter()
                            .collect::<String>()
                            .parse()
                            .unwrap_or(usize::MAX);
                        (Node::Back(number), true)
                    }
                    _ => (self.escape(false)?, true),
                }
            }
            other => {
                self.at += 1;
                (Node::Char(other), true)
            }
        };
        if !quantifiable {
            if matches!(self.peek(), Some('*' | '+' | '?')) || (self.peek() == Some('{') && self.braces().is_some()) {
                return Err(());
            }
            return Ok(atom);
        }
        match self.quantifier()? {
            Some((min, max)) => {
                let lazy = self.peek() == Some('?');
                if lazy {
                    self.at += 1;
                }
                Ok(Node::Repeat(Box::new(atom), min, max, !lazy))
            }
            None => Ok(atom),
        }
    }

    fn close(&mut self) -> Result<(), ()> {
        if self.peek() == Some(')') {
            self.at += 1;
            Ok(())
        } else {
            Err(())
        }
    }

    fn hex(&mut self, count: usize) -> Option<char> {
        let text: String = self.chars.get(self.at..self.at + count)?.iter().collect();
        let value = u32::from_str_radix(&text, 16).ok()?;
        if text.len() != count || !text.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        self.at += count;
        char::from_u32(value)
    }

    /// The escape after a `\`, the backslash read: a class or one character.
    fn escape(&mut self, in_set: bool) -> Result<Node, ()> {
        let c = self.peek().ok_or(())?;
        self.at += 1;
        Ok(match c {
            'd' => Node::Class(Class::Digit, false),
            'D' => Node::Class(Class::Digit, true),
            'w' => Node::Class(Class::Word, false),
            'W' => Node::Class(Class::Word, true),
            's' => Node::Class(Class::Space, false),
            'S' => Node::Class(Class::Space, true),
            't' => Node::Char('\t'),
            'n' => Node::Char('\n'),
            'r' => Node::Char('\r'),
            'v' => Node::Char('\u{b}'),
            'f' => Node::Char('\u{c}'),
            'b' if in_set => Node::Char('\u{8}'),
            '0' if !self.peek().is_some_and(|c| c.is_ascii_digit()) => Node::Char('\0'),
            'x' => match self.hex(2) {
                Some(one) => Node::Char(one),
                None => Node::Char('x'),
            },
            'u' => match self.hex(4) {
                Some(one) => Node::Char(one),
                None => Node::Char('u'),
            },
            'c' => match self.peek() {
                Some(letter) if letter.is_ascii_alphabetic() => {
                    self.at += 1;
                    Node::Char(char::from((letter as u8) % 32))
                }
                _ => {
                    self.at -= 1;
                    Node::Char('\\')
                }
            },
            other => Node::Char(other),
        })
    }

    fn set(&mut self) -> Result<Node, ()> {
        let negated = self.peek() == Some('^');
        if negated {
            self.at += 1;
        }
        let mut items = Vec::new();
        loop {
            let c = self.peek().ok_or(())?;
            if c == ']' {
                self.at += 1;
                break;
            }
            let first = self.set_atom()?;
            if self.peek() == Some('-') && self.chars.get(self.at + 1).is_some_and(|c| *c != ']') {
                self.at += 1;
                let second = self.set_atom()?;
                match (first, second) {
                    (Item::Char(a), Item::Char(b)) => {
                        if b < a {
                            return Err(());
                        }
                        items.push(Item::Range(a, b));
                    }
                    (first, second) => {
                        items.push(first);
                        items.push(Item::Char('-'));
                        items.push(second);
                    }
                }
            } else {
                items.push(first);
            }
        }
        Ok(Node::Set(items, negated))
    }

    fn set_atom(&mut self) -> Result<Item, ()> {
        let c = self.peek().ok_or(())?;
        self.at += 1;
        if c != '\\' {
            return Ok(Item::Char(c));
        }
        Ok(match self.escape(true)? {
            Node::Char(one) => Item::Char(one),
            Node::Class(class, negated) => Item::Class(class, negated),
            _ => return Err(()),
        })
    }
}

struct Matcher<'a> {
    text: &'a [char],
    caps: RefCell<Vec<Option<(usize, usize)>>>,
}

impl Matcher<'_> {
    fn at(&self, node: &Node, pos: usize, k: &dyn Fn(usize) -> bool) -> bool {
        let text = self.text;
        match node {
            Node::Char(c) => text.get(pos) == Some(c) && k(pos + 1),
            Node::Any => {
                text.get(pos)
                    .is_some_and(|c| !matches!(c, '\n' | '\r' | '\u{2028}' | '\u{2029}'))
                    && k(pos + 1)
            }
            Node::Class(class, negated) => text.get(pos).is_some_and(|c| in_class(class, *c) != *negated) && k(pos + 1),
            Node::Set(items, negated) => {
                let Some(c) = text.get(pos).copied() else {
                    return false;
                };
                let inside = items.iter().any(|item| match item {
                    Item::Char(one) => *one == c,
                    Item::Range(a, b) => *a <= c && c <= *b,
                    Item::Class(class, negated) => in_class(class, c) != *negated,
                });
                inside != *negated && k(pos + 1)
            }
            Node::Start => pos == 0 && k(pos),
            Node::End => pos == text.len() && k(pos),
            Node::Boundary(wanted) => {
                let before = pos > 0 && is_word(text[pos - 1]);
                let after = pos < text.len() && is_word(text[pos]);
                ((before != after) == *wanted) && k(pos)
            }
            Node::Group(inner, index) => match index {
                None => self.at(inner, pos, k),
                Some(index) => {
                    let index = *index;
                    self.at(inner, pos, &|end| {
                        let was = self.caps.borrow()[index];
                        self.caps.borrow_mut()[index] = Some((pos, end));
                        if k(end) {
                            return true;
                        }
                        self.caps.borrow_mut()[index] = was;
                        false
                    })
                }
            },
            Node::Look(inner, positive) => {
                let saved = self.caps.borrow().clone();
                let found = self.at(inner, pos, &|_| true);
                if *positive {
                    if found && k(pos) {
                        return true;
                    }
                    *self.caps.borrow_mut() = saved;
                    false
                } else {
                    *self.caps.borrow_mut() = saved;
                    !found && k(pos)
                }
            }
            Node::Back(index) => {
                let found = self.caps.borrow().get(*index).copied().flatten();
                match found {
                    None => k(pos),
                    Some((from, to)) => {
                        let len = to - from;
                        pos + len <= text.len() && text[from..to] == text[pos..pos + len] && k(pos + len)
                    }
                }
            }
            Node::Repeat(inner, min, max, greedy) => self.repeat(inner, *min, *max, *greedy, 0, pos, k),
            Node::Concat(nodes) => self.sequence(nodes, pos, k),
            Node::Alt(options) => options.iter().any(|one| self.at(one, pos, k)),
        }
    }

    fn sequence(&self, nodes: &[Node], pos: usize, k: &dyn Fn(usize) -> bool) -> bool {
        match nodes.split_first() {
            None => k(pos),
            Some((first, rest)) => self.at(first, pos, &|next| self.sequence(rest, next, k)),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn repeat(
        &self,
        inner: &Node,
        min: usize,
        max: Option<usize>,
        greedy: bool,
        count: usize,
        pos: usize,
        k: &dyn Fn(usize) -> bool,
    ) -> bool {
        if count < min {
            return self.at(inner, pos, &|next| {
                self.repeat(inner, min, max, greedy, count + 1, next, k)
            });
        }
        let more = max.is_none_or(|max| count < max);
        let again = || {
            more && self.at(inner, pos, &|next| {
                next != pos && self.repeat(inner, min, max, greedy, count + 1, next, k)
            })
        };
        // A greedy one tries one more first, a lazy one tries what follows first.
        let (first, second): (&dyn Fn() -> bool, &dyn Fn() -> bool) = if greedy {
            (&again, &|| k(pos))
        } else {
            (&|| k(pos), &again)
        };
        first() || second()
    }
}

/// Whether the whole of `text` matches `pattern`, as `new RegExp(`^(?:${pattern})$`).test(text)`
/// answers; none when the browser would refuse the pattern.
pub fn full_match(pattern: &str, text: &str) -> Option<bool> {
    let mut parser = Parser {
        chars: pattern.chars().collect(),
        at: 0,
        groups: 0,
    };
    let node = parser.disjunction().ok()?;
    if parser.at != parser.chars.len() {
        return None;
    }
    let text: Vec<char> = text.chars().collect();
    let matcher = Matcher {
        text: &text,
        caps: RefCell::new(vec![None; parser.groups + 1]),
    };
    let len = text.len();
    let whole = Node::Concat(vec![Node::Start, Node::Group(Box::new(node), None), Node::End]);
    Some(matcher.at(&whole, 0, &|end| end == len))
}
