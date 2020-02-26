use crate::{
    source::{
        Source
    },
    lexable::{
        Lexable
    }
};

use std::{
    collections::{
        HashMap,
        HashSet
    },
    ops::{
        Range
    }
};

#[derive(Clone)]
pub struct Lexer<T, S> {
    source: S,
    pub token: T,
    pub source_begin: usize,
    pub source_end: usize,
    pub token_begin: usize,
    pub token_end: usize,
    current_pos: usize
}

impl<'source, T, S> Lexer<T, S> 
    where T: Lexable, S: Source<'source> {

    pub fn new(source: S) -> Self {
        let len = source.len();
        Self {
            source: source,
            token: T::get_error_variant(),
            source_begin: 0,
            source_end: len,
            token_begin: 0,
            token_end: 0,
            current_pos: 0
        }
    }

    fn get_slice(&self) -> &'source str {
        self.source.get_at(self.current_pos)
    }

    fn is_whitespace(&self, slice: &str) -> bool {
        match slice {
            " " => true,
            "\t" => true,
            "\n" => true,
            "\r" => true,
            _ => false
        }
    }

    pub fn advance(&mut self) {
        let mut begin_pos = self.current_pos;
        let mut matched_in_past = false;

        if self.current_pos >= self.source_end {
            self.token_begin = begin_pos;
            self.token_end = begin_pos;
            self.token = T::get_end_variant();
            return;
        }
        
        //println!("Lexer advance source len: {}", self.source_end);
        //println!("Lexer advance remaining input: {}", self.source.get_slice(self.current_pos, self.source_end));

        let mut current_slice = String::new();
        let mut last_slice;

        let mut last_matches: Vec<T> = Vec::new();

        let mut token_match_map: HashMap<T, Range<usize>> = HashMap::new();

        while self.current_pos < self.source_end {
            last_slice = self.get_slice();
            current_slice += last_slice;

            let token_matches = T::match_token(&current_slice);

            //println!("Token matches: {:?}", token_matches);

            if token_matches.is_empty() && self.is_whitespace(last_slice) {
                if matched_in_past {
                    //println!("Breaking out of lexer loop.");
                    break;
                } else if current_slice.trim().is_empty() {
                    begin_pos += 1;
                    current_slice = String::from(current_slice.trim_start());
                }
            }

            if token_matches == last_matches {
                for token in token_matches.iter() {
                    if let Some(range) = token_match_map.get_mut(token) {
                        *range = range.start..self.current_pos + 1;
                    }
                }
            }

            if token_matches != last_matches {
                matched_in_past = true;

                for token in last_matches.iter() {
                    if !token_matches.contains(token) {
                        if let Some(range) = token_match_map.get_mut(token) {
                            if token.is_inclusive() {
                                //println!("It is inclusive.");
                                *range = range.start..self.current_pos - 1;
                                //println!("Slice: {}", self.source.get_slice(range.start, range.end));
                            } else {
                                *range = range.start..self.current_pos;
                            }
                        }
                    }
                }

                for token in token_matches.iter() {
                    if !last_matches.contains(token) {
                        let range = begin_pos..self.current_pos + 1;
                        token_match_map.insert(token.clone(), range);
                    }
                }

                last_matches = token_matches;
            }

            self.current_pos += 1;
        }

        if self.current_pos == self.source_end {
            if !matched_in_past {
                self.token = T::get_end_variant();
            }
        }

        let mut match_results: Vec<(T, Range<usize>)> = token_match_map.into_iter().collect();

        match_results.sort_by(|(t1, range1), (t2, range2)| {
            let len1 = range1.len();
            let len2 = range2.len();
            let prio1 = t1.prio();
            let prio2 = t2.prio();
            if len1 == len2 {
                return prio2.cmp(&prio1);
            } else {
                return len2.cmp(&len1);
            }
        });

        if match_results.is_empty() {
            self.token = T::get_error_variant();
            self.token_begin = begin_pos;
            self.token_end = self.current_pos;
            return;
        }

        let (token, token_range) = match_results.get(0).unwrap();

        //println!("Best match: {:?}, {:?}", token, token_range);
        //println!("Last token of this match: {}", self.source.get_at(token_range.end - 1));

        self.token_begin = token_range.start;
        self.token_end = token_range.end;
        self.current_pos = token_range.end;
        self.token = token.clone();

        if self.token.should_skip() {
            //println!("Skipping this token.");
            self.advance();
        }
    }

    pub fn slice(&self) -> &'source str {
        self.source.get_slice(self.token_begin, self.token_end)
    }

    pub fn range(&self) -> Range<usize> {
        self.token_begin..self.token_end
    }
}