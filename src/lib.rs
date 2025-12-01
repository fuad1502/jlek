//! Generate lexer from token specifications.
//!
//! JLEK is the lexer used by [JJIK]() parser generator. Currently, JLEK cannot be used standalone
//! without [JJIK](), since the generated lexer module (`lexer.rs`) depends on a module generated
//! by [JJIK]() (`symbol.rs`).
//!
//! # Usage
//!
//! ```toml
//! [dependencies]
//! jlek = "0.1.0"
//! ```
//! A token specification consists of an identifier and a regular expression (see
//! [TokenSpec](struct.TokenSpec.html)). Accepted regular expression syntax is given in [Regular
//! Expression Syntax](crate#regular-expression-syntax).
//!
//! ```rust
//! use std::path::PathBuf;
//!
//! // create a token specification for decimal numbers
//! let number = jlek::TokenSpec::new("Number".to_string(), "\\d\\d*".to_string());
//! let token_specs = vec![number];
//!
//! // generate `lexer.rs` at `output_directory`
//! let output_directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
//! jlek::generate(&token_specs, &output_directory).unwrap();
//! ```
//! You can then use the generated lexer as follows:
//!
//! ```ignore
//! mod lexer;
//!
//! let lexer = lexer::Lexer::from_source_str("123");
//! lexer.next_token().unwrap();
//! ```
//! # Regular Expression Syntax
//!
//! ## Character classes
//!
//! ```text
//! \d      decimal digit (0-9).
//! \w      lowercase character (a-z).
//! \       escape character for matching with special characters (`\`, `*`, `|`, `(`, `)`), e.g.
//!         `\*` matches with "*".
//! ```
//! ## Supported operators
//!
//! ```text
//! xy      concatenation; match with x followed by y.
//! x|y     disjunction; match with either x or y.
//! x*      kleene; match with one or more occurance x.
//! (x)     parenthesis; groups an expression for overriding precedence.
//! ```

mod code_gen;
mod lexer_spec;
mod regex_parser;

pub use code_gen::generate;

/// A token specification.
pub struct TokenSpec {
    name: String,
    pattern: String,
}

impl TokenSpec {
    /// Creates a new token specification.
    ///
    /// `name` and `pattern` corresponds to the identifier and regular expression of a terminal
    /// listed in a [GG file](), respectively. See [Regular Expression
    /// Syntax](crate#regular-expression-syntax) for valid regular expression syntax.
    pub fn new(name: String, pattern: String) -> Self {
        Self { name, pattern }
    }

    /// Obtains the token specification regular expression.
    pub fn pattern(&self) -> &str {
        &self.pattern
    }

    /// Obtains the token specification identifier.
    pub fn name(&self) -> &str {
        &self.name
    }
}
