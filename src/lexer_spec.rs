use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
};

use crate::{
    TokenSpec,
    regex_parser::{self, RegexNode, RegexTerminal},
};

pub struct LexerSpec<'a> {
    pub token_specs: &'a Vec<TokenSpec>,
    pub states: Vec<State>,
    pub initial_states: Vec<usize>,
}

#[derive(Debug)]
pub struct State {
    pub accepts: Option<String>,
    pub next: HashMap<char, usize>,
}

#[derive(Debug)]
struct DfaState {
    terminals: HashSet<RegexTerminal>,
    next: HashMap<char, usize>,
}

#[derive(Default)]
struct Cache {
    first_pos_table: HashMap<Rc<RegexNode>, HashSet<RegexTerminal>>,
    last_pos_table: HashMap<Rc<RegexNode>, HashSet<RegexTerminal>>,
    follow_pos_table: HashMap<RegexTerminal, HashSet<RegexTerminal>>,
    nullable_table: HashMap<Rc<RegexNode>, bool>,
}

impl<'a> LexerSpec<'a> {
    pub fn new(token_specs: &'a Vec<TokenSpec>) -> Self {
        Self {
            token_specs,
            states: vec![],
            initial_states: vec![],
        }
        .fill_states()
    }

    fn fill_states(mut self) -> Self {
        for token_spec in self.token_specs {
            let dfa = Self::create_dfa(&token_spec.pattern);

            let dfa_root_idx = self.states.len();
            self.initial_states.push(dfa_root_idx);

            for dfa_state in dfa {
                let accepts = dfa_state.is_accepting().then_some(token_spec.name.clone());
                let next = dfa_state
                    .next
                    .iter()
                    .map(|(&ch, next)| (ch, next + dfa_root_idx))
                    .collect();
                let state = State { accepts, next };
                self.states.push(state);
            }
        }
        self
    }

    fn create_dfa(pattern: &str) -> Vec<DfaState> {
        let (regex_root, alphabet) = regex_parser::parse_regex(pattern).unwrap();
        let cache = Cache::new(&regex_root);

        let first_state = DfaState::new(cache.first_pos(&regex_root).clone());
        let mut states = vec![first_state];
        let mut visited_states = 0;

        while visited_states < states.len() {
            for ch in &alphabet {
                let mut follow_pos_union = HashSet::new();
                for terminal in states[visited_states]
                    .terminals
                    .iter()
                    .filter(|&t| t.ch == *ch)
                {
                    if let Some(follow_pos) = cache.follow_pos(terminal) {
                        follow_pos_union = &follow_pos_union | follow_pos;
                    }
                }
                if let Some(idx) = states.iter().position(|s| follow_pos_union == s.terminals) {
                    states[visited_states].next.insert(*ch, idx);
                } else if !follow_pos_union.is_empty() {
                    let new_state = DfaState::new(follow_pos_union);
                    let idx = states.len();
                    states[visited_states].next.insert(*ch, idx);
                    states.push(new_state);
                }
            }
            visited_states += 1;
        }
        states
    }
}

impl DfaState {
    fn new(terminals: HashSet<RegexTerminal>) -> Self {
        Self {
            terminals,
            next: HashMap::new(),
        }
    }

    fn is_accepting(&self) -> bool {
        self.terminals.iter().any(|t| t.ch == '\0')
    }
}

impl Cache {
    fn new(root_node: &Rc<RegexNode>) -> Self {
        let mut cache = Self::default();
        cache.calculate_nullable(root_node);
        cache.calculate_first_pos(root_node);
        cache.calculate_last_pos(root_node);
        cache.calculate_follow_pos(root_node);
        cache
    }

    fn calculate_nullable(&mut self, node: &Rc<RegexNode>) {
        match &**node {
            RegexNode::Cat(left, right) => {
                self.calculate_nullable(left);
                self.calculate_nullable(right);
                let left_nullable = self.nullable(left);
                let right_nullable = self.nullable(right);
                let nullable = left_nullable && right_nullable;
                _ = self.nullable_table.insert(node.clone(), nullable);
            }
            RegexNode::Or(left, right) => {
                self.calculate_nullable(left);
                self.calculate_nullable(right);
                let left_nullable = self.nullable(left);
                let right_nullable = self.nullable(right);
                let nullable = left_nullable || right_nullable;
                _ = self.nullable_table.insert(node.clone(), nullable);
            }
            RegexNode::Parenthesized(child) => {
                self.calculate_nullable(child);
                let nullable = self.nullable(child);
                _ = self.nullable_table.insert(node.clone(), nullable);
            }
            RegexNode::Kleene(child) => {
                self.calculate_nullable(child);
                _ = self.nullable_table.insert(node.clone(), true);
            }
            RegexNode::Terminal(_) => _ = self.nullable_table.insert(node.clone(), false),
        };
    }

    fn calculate_first_pos(&mut self, node: &Rc<RegexNode>) {
        match &**node {
            RegexNode::Cat(left, right) => {
                self.calculate_first_pos(left);
                self.calculate_first_pos(right);
                let left_first_pos = self.first_pos(left);
                let right_first_pos = self.first_pos(right);
                let left_nullable = self.nullable(left);
                let first_pos = if left_nullable {
                    left_first_pos.union(right_first_pos).cloned().collect()
                } else {
                    left_first_pos.clone()
                };
                _ = self.first_pos_table.insert(node.clone(), first_pos);
            }
            RegexNode::Or(left, right) => {
                self.calculate_first_pos(left);
                self.calculate_first_pos(right);
                let left_first_pos = self.first_pos(left);
                let right_first_pos = self.first_pos(right);
                let first_pos = left_first_pos.union(right_first_pos).cloned().collect();
                _ = self.first_pos_table.insert(node.clone(), first_pos);
            }
            RegexNode::Parenthesized(child) | RegexNode::Kleene(child) => {
                self.calculate_first_pos(child);
                let first_pos = self.first_pos(child).clone();
                _ = self.first_pos_table.insert(node.clone(), first_pos);
            }
            RegexNode::Terminal(t) => {
                let first_pos = HashSet::from([t.clone()]);
                _ = self.first_pos_table.insert(node.clone(), first_pos);
            }
        }
    }

    fn calculate_last_pos(&mut self, node: &Rc<RegexNode>) {
        match &**node {
            RegexNode::Cat(left, right) => {
                self.calculate_last_pos(left);
                self.calculate_last_pos(right);
                let left_last_pos = self.last_pos(left);
                let right_last_pos = self.last_pos(right);
                let right_nullable = self.nullable(right);
                let last_pos = if right_nullable {
                    right_last_pos.union(left_last_pos).cloned().collect()
                } else {
                    right_last_pos.clone()
                };
                _ = self.last_pos_table.insert(node.clone(), last_pos);
            }
            RegexNode::Or(left, right) => {
                self.calculate_last_pos(left);
                self.calculate_last_pos(right);
                let left_last_pos = self.last_pos(left);
                let right_last_pos = self.last_pos(right);
                let last_pos = left_last_pos.union(right_last_pos).cloned().collect();
                _ = self.last_pos_table.insert(node.clone(), last_pos);
            }
            RegexNode::Parenthesized(child) | RegexNode::Kleene(child) => {
                self.calculate_last_pos(child);
                let last_pos = self.last_pos(child).clone();
                _ = self.last_pos_table.insert(node.clone(), last_pos);
            }
            RegexNode::Terminal(t) => {
                let last_pos = HashSet::from([t.clone()]);
                _ = self.last_pos_table.insert(node.clone(), last_pos);
            }
        }
    }

    fn calculate_follow_pos(&mut self, node: &Rc<RegexNode>) {
        match &**node {
            RegexNode::Cat(left, right) => {
                let left_last_pos = self.last_pos_table.get(left).unwrap();
                let right_first_pos = self.first_pos_table.get(right).unwrap();
                for terminal in left_last_pos {
                    if !self.follow_pos_table.contains_key(terminal) {
                        self.follow_pos_table
                            .insert(terminal.clone(), HashSet::new());
                    }
                    for follow in right_first_pos {
                        self.follow_pos_table
                            .get_mut(terminal)
                            .unwrap()
                            .insert(follow.clone());
                    }
                }
                self.calculate_follow_pos(left);
                self.calculate_follow_pos(right);
            }
            RegexNode::Kleene(node) => {
                let last_pos = self.last_pos_table.get(node).unwrap();
                let first_pos = self.first_pos_table.get(node).unwrap();
                for terminal in last_pos {
                    if !self.follow_pos_table.contains_key(terminal) {
                        self.follow_pos_table
                            .insert(terminal.clone(), HashSet::new());
                    }
                    for follow in first_pos {
                        self.follow_pos_table
                            .get_mut(terminal)
                            .unwrap()
                            .insert(follow.clone());
                    }
                }
                self.calculate_follow_pos(node);
            }
            RegexNode::Parenthesized(node) => self.calculate_follow_pos(node),
            RegexNode::Or(left, right) => {
                self.calculate_follow_pos(left);
                self.calculate_nullable(right);
            }
            RegexNode::Terminal(_) => (),
        }
    }

    fn nullable(&self, node: &Rc<RegexNode>) -> bool {
        *self.nullable_table.get(node).unwrap()
    }

    fn first_pos(&self, node: &Rc<RegexNode>) -> &HashSet<RegexTerminal> {
        self.first_pos_table.get(node).unwrap()
    }

    fn last_pos(&self, node: &Rc<RegexNode>) -> &HashSet<RegexTerminal> {
        self.last_pos_table.get(node).unwrap()
    }

    fn follow_pos(&self, node: &RegexTerminal) -> Option<&HashSet<RegexTerminal>> {
        self.follow_pos_table.get(node)
    }
}

impl RegexNode {
    pub fn terminal(ch: char, pos: usize) -> Self {
        let terminal = RegexTerminal { ch, pos };
        Self::Terminal(terminal)
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use crate::{TokenSpec, lexer_spec::LexerSpec};

    #[test]
    fn number() {
        let number = TokenSpec {
            name: "Number".to_string(),
            pattern: "\\d\\d*".to_string(),
        };
        let token_specs = vec![number];
        let lexer_spec = LexerSpec::new(&token_specs);
        assert_eq!(&lexer_spec.initial_states, &vec![0]);
        assert_eq!(&lexer_spec.states[0].accepts, &None);
        assert_eq!(
            &lexer_spec.states[0].next,
            &HashMap::from([
                ('0', 1),
                ('1', 1),
                ('2', 1),
                ('3', 1),
                ('4', 1),
                ('5', 1),
                ('6', 1),
                ('7', 1),
                ('8', 1),
                ('9', 1),
            ])
        );
        assert_eq!(&lexer_spec.states[1].accepts, &Some("Number".to_string()));
        assert_eq!(
            &lexer_spec.states[1].next,
            &HashMap::from([
                ('0', 1),
                ('1', 1),
                ('2', 1),
                ('3', 1),
                ('4', 1),
                ('5', 1),
                ('6', 1),
                ('7', 1),
                ('8', 1),
                ('9', 1),
            ])
        );
    }
}
