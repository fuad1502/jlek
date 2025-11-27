mod code_gen;
mod lexer_spec;
mod regex_parser;

pub use code_gen::generate;

pub struct TokenSpec {
    name: String,
    pattern: String,
}

impl TokenSpec {
    pub fn new(name: String, pattern: String) -> Self {
        Self { name, pattern }
    }

    pub fn pattern(&self) -> &str {
        &self.pattern
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}
