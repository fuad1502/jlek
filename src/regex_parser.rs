mod lexer;

use std::{collections::HashSet, rc::Rc};

use crate::regex_parser::lexer::{Lexer, SpecialToken, Token};

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct RegexTerminal {
    pub pos: usize,
    pub ch: char,
}

#[derive(PartialEq, Eq, Hash, Debug)]
pub enum RegexNode {
    Cat(Rc<RegexNode>, Rc<RegexNode>),
    Or(Rc<RegexNode>, Rc<RegexNode>),
    Parenthesized(Rc<RegexNode>),
    Kleene(Rc<RegexNode>),
    Terminal(RegexTerminal),
}

pub fn parse_regex(pattern: &str) -> Result<(Rc<RegexNode>, HashSet<char>), String> {
    let parser = RegexParser::new(pattern);
    parser.parse()
}

struct RegexParser {
    lexer: Lexer,
    current_pos: usize,
    alphabet: HashSet<char>,
}

// P -> P1
// P1 -> P2 '|' P1 | P2   % Or expression
// P2 -> P3 P2 | P3       % Concatenated expression
// P3 -> P4* | P4         % Kleene expression
// P4 -> '(' P1 ')' | P5  % Parenthesized expression
// P5 -> Char | Special   % Basic expression

impl RegexParser {
    fn new(pattern: &str) -> Self {
        let lexer = Lexer::new(pattern);
        Self {
            lexer,
            current_pos: 0,
            alphabet: HashSet::new(),
        }
    }

    fn parse(mut self) -> Result<(Rc<RegexNode>, HashSet<char>), String> {
        let p1 = self.p1()?;
        if *self.lexer.peek()? != Token::End {
            return Err("Expected EOF".to_string());
        }
        Ok((self.augment(p1), self.alphabet))
    }

    fn p1(&mut self) -> Result<Rc<RegexNode>, String> {
        let mut p1 = self.p2()?;
        while *self.lexer.peek()? == Token::Or {
            _ = self.lexer.next()?;
            let p2 = self.p2()?;
            p1 = Self::or(p1, p2);
        }
        Ok(p1)
    }

    fn p2(&mut self) -> Result<Rc<RegexNode>, String> {
        let mut p2 = self.p3()?;
        while !Self::is_in_follow_p2(self.lexer.peek()?) {
            let p3 = self.p3()?;
            p2 = Self::cat(p2, p3);
        }
        Ok(p2)
    }

    fn p3(&mut self) -> Result<Rc<RegexNode>, String> {
        let mut p3 = self.p4()?;
        if *self.lexer.peek()? == Token::Star {
            _ = self.lexer.next();
            p3 = Self::kleene(p3);
        }
        Ok(p3)
    }

    fn p4(&mut self) -> Result<Rc<RegexNode>, String> {
        if *self.lexer.peek()? == Token::LeftParen {
            _ = self.lexer.next()?;
            let p1 = self.p1()?;
            if *self.lexer.peek()? != Token::RightParen {
                return Err("Expected closing right parenthesis".to_string());
            }
            _ = self.lexer.next()?;
            Ok(Self::parenthesized(p1))
        } else {
            self.p5()
        }
    }

    fn p5(&mut self) -> Result<Rc<RegexNode>, String> {
        match self.lexer.next()? {
            Token::Char(ch) => Ok(self.single_char(ch)),
            Token::Special(SpecialToken::Number) => Ok(self.number()),
            Token::Special(SpecialToken::Lowercase) => Ok(self.lowercase()),
            _ => Err("Expected (special) character".to_string()),
        }
    }

    fn is_in_follow_p2(token: &Token) -> bool {
        *token == Token::Or || *token == Token::End || *token == Token::RightParen
    }

    fn augment(&mut self, node: Rc<RegexNode>) -> Rc<RegexNode> {
        let sentinel = Rc::new(RegexNode::terminal('\0', self.current_pos));
        self.current_pos += 1;
        Self::cat(node, sentinel)
    }

    fn number(&mut self) -> Rc<RegexNode> {
        let mut node = None;
        for i in 0..10 {
            let ch = char::from_digit(i, 10).unwrap();
            match node {
                None => node = Some(self.single_char(ch)),
                Some(n) => node = Some(Self::or(n, self.single_char(ch))),
            }
            self.alphabet.insert(ch);
        }
        node.unwrap()
    }

    fn lowercase(&mut self) -> Rc<RegexNode> {
        let mut node = None;
        for ch in 'a'..='z' {
            match node {
                None => node = Some(self.single_char(ch)),
                Some(n) => node = Some(Self::or(n, self.single_char(ch))),
            }
            self.alphabet.insert(ch);
        }
        node.unwrap()
    }

    fn single_char(&mut self, ch: char) -> Rc<RegexNode> {
        let char = Rc::new(RegexNode::terminal(ch, self.current_pos));
        self.current_pos += 1;
        self.alphabet.insert(ch);
        char
    }

    fn cat(left: Rc<RegexNode>, right: Rc<RegexNode>) -> Rc<RegexNode> {
        Rc::new(RegexNode::Cat(left, right))
    }

    fn or(left: Rc<RegexNode>, right: Rc<RegexNode>) -> Rc<RegexNode> {
        Rc::new(RegexNode::Or(left, right))
    }

    fn parenthesized(node: Rc<RegexNode>) -> Rc<RegexNode> {
        Rc::new(RegexNode::Parenthesized(node))
    }

    fn kleene(node: Rc<RegexNode>) -> Rc<RegexNode> {
        Rc::new(RegexNode::Kleene(node))
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashSet;

    use crate::regex_parser::parse_regex;

    #[test]
    fn main() {
        let (_, alphabet) = parse_regex("a(bc)*|\\d").unwrap();
        assert_eq!(
            alphabet,
            HashSet::from([
                'a', 'b', 'c', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9'
            ])
        );
    }
}
