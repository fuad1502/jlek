use std::{collections::HashMap, fs::File, io::Read, path::Path};

use crate::symbol::{Span, Terminal, TerminalClass};

static NUM_OF_STATES: usize = 2;

#[derive(Copy, Clone)]
struct State {
    class: Option<TerminalClass>,
}

pub struct Lexer {
    chars: Vec<u8>,
    line_start_indices: Vec<usize>,
    start_pos: usize,
    current_pos: usize,
    current_token: Option<Terminal>,
    states: [State; NUM_OF_STATES],
    transition_table: Vec<HashMap<char, usize>>,
    states_stack: Vec<Vec<usize>>,
}

impl Lexer {
    pub fn from_source_str(source: &str) -> Self {
        let chars = source.chars().map(|c| c as u8).collect::<Vec<u8>>();
        let mut line_start_indices = chars
            .iter()
            .enumerate()
            .filter_map(|(i, c)| if *c == b'\n' { Some(i + 1) } else { None })
            .collect::<Vec<usize>>();
        line_start_indices.insert(0, 0);
        let states = [
            State { class: None },
            State { class: Some(TerminalClass::Number) },
        ];
        let initial_states = vec![0];
        let mut state_0_transitions = HashMap::new();
        state_0_transitions.insert('4', 1);
        state_0_transitions.insert('0', 1);
        state_0_transitions.insert('7', 1);
        state_0_transitions.insert('3', 1);
        state_0_transitions.insert('2', 1);
        state_0_transitions.insert('9', 1);
        state_0_transitions.insert('5', 1);
        state_0_transitions.insert('8', 1);
        state_0_transitions.insert('1', 1);
        state_0_transitions.insert('6', 1);
        let mut state_1_transitions = HashMap::new();
        state_1_transitions.insert('8', 1);
        state_1_transitions.insert('5', 1);
        state_1_transitions.insert('2', 1);
        state_1_transitions.insert('4', 1);
        state_1_transitions.insert('0', 1);
        state_1_transitions.insert('7', 1);
        state_1_transitions.insert('1', 1);
        state_1_transitions.insert('6', 1);
        state_1_transitions.insert('9', 1);
        state_1_transitions.insert('3', 1);
        let transition_table = vec![
            state_0_transitions,
            state_1_transitions,
        ];

        Self {
            chars,
            line_start_indices,
            start_pos: 0,
            current_pos: 0,
            current_token: None,
            states,
            transition_table,
            states_stack: vec![initial_states],
        }
    }
            
    pub fn new(source_file: &Path) -> Result<Self, std::io::Error> {
        let mut source_file = File::open(source_file)?;
        let mut source = String::new();
        let _ = source_file.read_to_string(&mut source)?;
        Ok(Self::from_source_str(&source))
    }

    pub fn next_token(&mut self) -> Result<Terminal, String> {
        let token = self.peek_token()?.clone();
        self.move_start_pos();
        self.current_token = None;
        Ok(token)
    }

    pub fn peek_token(&mut self) -> Result<&Terminal, String> {
        if self.current_token.is_none() {
            self.skip_whitespaces();
            if self.peek_char().is_none() {
                let end_token = Terminal::new(TerminalClass::End, self.current_span());
                self.current_token = Some(end_token);
            } else {
                self.current_token = Some(self.get()?);
                _ = self.states_stack.split_off(1);
            }
        }
        Ok(self.current_token.as_ref().unwrap())
    }

    pub fn get_lexeme(&self, token: &Terminal) -> &str {
        str::from_utf8(&self.chars[token.span().start_pos()..token.span().end_pos()]).unwrap()
    }

    pub fn show_span(&self, span: &Span) -> String {
        let line_number = self
            .line_start_indices
            .partition_point(|&i| i <= span.start_pos());
        let line_start_idx = self.line_start_indices[line_number - 1];
        let line_end_idx = match self.line_start_indices.get(line_number) {
            Some(idx) => idx - 1,
            None => self.chars.len(),
        };
        let line = &self.chars[line_start_idx..line_end_idx];
        let line = str::from_utf8(line).unwrap();
        let span_offset = span.start_pos() - line_start_idx;
        let span_length = span.end_pos() - span.start_pos();
        let span_marker = format!(
            "{}{}{}",
            " ".repeat(span_offset),
            "^",
            "-".repeat(span_length.saturating_sub(1))
        );
        format!("Line {line_number:3}|{line}\n         {span_marker}")
    }

    fn move_start_pos(&mut self) {
        self.start_pos = self.current_pos;
    }

    fn get(&mut self) -> Result<Terminal, String> {
        loop {
            match self.peek_char() {
                Some(c) => {
                    if self.move_states_on_stack(c) {
                        self.read_char();
                    } else {
                        return self.evaluate_stack();
                    }
                }
                None => return self.evaluate_stack(),
            }
        }
    }

    fn move_states_on_stack(&mut self, input: char) -> bool {
        let mut new_states = vec![];
        for state in self.states_stack.last().unwrap() {
            if let Some(new_state) = self.transition_table[*state].get(&input) {
                new_states.push(*new_state);
            }
        }
        if !new_states.is_empty() {
            self.states_stack.push(new_states);
            return true;
        }
        false
    }

    fn evaluate_stack(&mut self) -> Result<Terminal, String> {
        loop {
            let mut accepting_classes = vec![];
            for state in self.states_stack.last().unwrap() {
                if let Some(class) = self.states[*state].class {
                    accepting_classes.push(class);
                }
            }
            if let Some(prioritized_class) = accepting_classes.iter().copied().min() {
                let span = self.current_span();
                let class = prioritized_class;
                return Ok(Terminal::new(class, span));
            } else if self.states_stack.len() == 1 {
                return Err(self.report_error());
            } else {
                self.states_stack.pop();
                self.revert_char();
            }
        }
    }

    fn report_error(&self) -> String {
        let span_str = self.show_span(&self.current_span());
        format!(
            "{span_str}\nerror: unexpected character found: {}",
            self.peek_char()
                .map(|c| c.to_string())
                .unwrap_or(String::from("EOF"))
        )
    }

    fn peek_char(&self) -> Option<char> {
        self.chars.get(self.current_pos).copied().map(|c| c as char)
    }

    fn read_char(&mut self) -> Option<char> {
        let ch = self.peek_char();
        if ch.is_some() {
            self.current_pos += 1;
        }
        ch
    }

    fn revert_char(&mut self) {
        self.current_pos -= 1;
    }

    fn skip_whitespaces(&mut self) {
        while let Some(c) = self.peek_char() {
            if c.is_whitespace() {
                self.read_char();
            } else {
                break;
            }
        }
        self.move_start_pos();
    }

    fn current_span(&self) -> Span {
        Span::new(self.start_pos, self.current_pos)
    }
}
