pub struct Lexer {
    pattern: Vec<char>,
    current_pos: usize,
    current_token: Option<Token>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Token {
    Star,
    Or,
    LeftParen,
    RightParen,
    Char(char),
    Special(SpecialToken),
    End,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum SpecialToken {
    Lowercase,
    Number,
}

impl Lexer {
    pub fn new(pattern: &str) -> Self {
        Self {
            pattern: pattern.chars().collect(),
            current_pos: 0,
            current_token: None,
        }
    }

    pub fn next(&mut self) -> Result<Token, String> {
        let token = self.peek()?.clone();
        self.current_token = None;
        Ok(token)
    }

    pub fn peek(&mut self) -> Result<&Token, String> {
        if self.current_token.is_none() {
            self.current_token = Some(self.get()?);
        }
        Ok(self.current_token.as_ref().unwrap())
    }

    fn get(&mut self) -> Result<Token, String> {
        match self.char() {
            Some('*') => Ok(Token::Star),
            Some('|') => Ok(Token::Or),
            Some('(') => Ok(Token::LeftParen),
            Some(')') => Ok(Token::RightParen),
            Some('\\') => self.special_character(),
            Some(c) => Ok(Token::Char(c)),
            None => Ok(Token::End),
        }
    }

    fn special_character(&mut self) -> Result<Token, String> {
        match self.char() {
            Some(ch) if ch == '*' || ch == '|' || ch == '(' || ch == ')' || ch == '\\' => {
                Ok(Token::Char(ch))
            }
            Some('d') => Ok(Token::Special(SpecialToken::Number)),
            Some('w') => Ok(Token::Special(SpecialToken::Lowercase)),
            _ => Err("Error while parsing special character".to_string()),
        }
    }

    fn char(&mut self) -> Option<char> {
        let ch = self.pattern.get(self.current_pos).copied();
        if ch.is_some() {
            self.current_pos += 1;
        }
        ch
    }
}

#[cfg(test)]
mod test {
    use crate::regex_parser::lexer::{Lexer, Token};

    #[test]
    fn main() {
        let mut lexer = Lexer::new("a(bc)");
        assert_eq!(lexer.next().unwrap(), Token::Char('a'));
        assert_eq!(lexer.next().unwrap(), Token::LeftParen);
        assert_eq!(lexer.next().unwrap(), Token::Char('b'));
        assert_eq!(lexer.next().unwrap(), Token::Char('c'));
        assert_eq!(lexer.next().unwrap(), Token::RightParen);
        assert_eq!(lexer.next().unwrap(), Token::End);
    }
}
