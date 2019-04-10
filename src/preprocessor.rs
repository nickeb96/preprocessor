
use std::path::Path;
use std::io;
use std::fs::File;
use std::io::prelude::*;
use std::collections::HashMap;
use std::str::pattern::{Pattern, Searcher};

use regex::Regex;
use tokenizer;

use macrotable::{MacroTable, MacroValue};
use config::Config;
use lineiter;

fn expand_function_macro(text: &str, params: &HashMap<String, usize>, args: Vec<&str>) -> String {
    let mut ret = String::new();
    let mut iter = tokenizer::iter_tokens(text.to_string());
    let mut cursor = 0;

    assert_eq!(params.len(), args.len(), "Error: mismatch argument and paramater length for macro");

    eprintln!("expand_function_macro called with -> params: {:?} | args: {:?}", params, args);

    while let Some((begin, end)) = iter.next() {
        let token = &text[begin..end];

        if let Some(&index) = params.get(token) {
            ret.push_str(&text[cursor..begin]);
            ret.push_str(args[index]);
        }
        else {
            ret.push_str(&text[cursor..end]);
        }

        cursor = end;
    }
    ret
}

fn expand_line_wraps(source: &str) -> String {
    let re = Regex::new(r"(?m)\\\n").unwrap();

    let source = re.replace_all(&source, "");

    source.into_owned()
}

fn strip_comments(source: &str) -> String {
    let multi_line_comments = Regex::new(r"(?msU)/\*.*\*/").unwrap();
    let single_line_comments = Regex::new(r"//.*").unwrap();

    let source = multi_line_comments.replace_all(&source, "");
    let source = single_line_comments.replace_all(&source, "");

    source.into_owned()
}



lazy_static! {
    static ref DOUBLE_QUOTE_RE: Regex = Regex::new(r#""\s*$"#).unwrap();
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum State {
    NotYetFound,
    WithinTrueBlock,
    AlreadyFound,
}


pub struct PreProcessor<'b> {
    input: String,
    output: String,
    pub macros: MacroTable,
    config: &'b Config,
    state_stack: Vec<State>,
}

impl<'b> PreProcessor<'b> {
    pub fn new(conf: &Config) -> PreProcessor {
        PreProcessor {
            input: String::new(),
            output: String::new(),
            macros: MacroTable::new(),
            config: conf,
            state_stack: Vec::new(),
        }
    }

    fn get_header_contents(&self, file_name: &Path) -> io::Result<String> {
        for include_dir in self.config.include_dirs.iter() {
            let full_path = include_dir.join(file_name);
            if full_path.is_file() {
                let mut file = File::open(full_path)?;
                let mut contents = String::new();
                file.read_to_string(&mut contents);
                return Ok(contents);
            }
        }

        Err(io::Error::new(io::ErrorKind::NotFound, "Header file not found"))
    }

    fn include_source(&mut self, s: &str) {
        let re = Regex::new(r###"\s*("([^"]+)"|<([^<>]+)>)\s*"###).unwrap();
        let path: &Path;

        if let Some(caps) = re.captures(s) {
            if let Some(header_name) = caps.get(2) {
                path = Path::new(header_name.as_str());
            }
            else if let Some(header_name) = caps.get(3) {
                path = Path::new(header_name.as_str());
            }
            else {
                panic!("Something bad happened, idk what");
            }
        }
        else {
            panic!("Ill formatted include directive");
        }

        let source = self.get_header_contents(path).unwrap();

        self.preprocess_source(&source);
    }

    fn run_directive(&mut self, line: &str) {
        let re = Regex::new(r"^\s*#\s*([a-z]+)\s*(.*)\s*$").unwrap();

        let caps = re.captures(line).expect("Ill formatted preprocessor directive");

        match caps.get(1).unwrap().as_str() {
            "include" => {
                if self.state_stack.last().cloned().unwrap_or(State::WithinTrueBlock) == State::WithinTrueBlock {
                    self.include_source(caps.get(2).unwrap().as_str());
                }
            }
            "define" => {
                if self.state_stack.last().cloned().unwrap_or(State::WithinTrueBlock) == State::WithinTrueBlock {
                    self.macros.define(caps.get(2).unwrap().as_str());
                }
            }
            "undef" => {
                if self.state_stack.last().cloned().unwrap_or(State::WithinTrueBlock) == State::WithinTrueBlock {
                    self.macros.undef(caps.get(2).unwrap().as_str());
                }
            }
            "error" => {
                if self.state_stack.last().cloned().unwrap_or(State::WithinTrueBlock) == State::WithinTrueBlock {
                    panic!("Error: {}", caps.get(2).map(|arg| arg.as_str()).unwrap_or("**No Error Message**"));
                }
            }
            "warning" => {
                eprintln!("Warning: {}", caps.get(2).map(|arg| arg.as_str()).unwrap_or("**No Warning Message**"));
            }
            "ifdef" => {
                let parent_state = self.state_stack.last().cloned().unwrap_or(State::WithinTrueBlock);

                if parent_state == State::WithinTrueBlock {
                    if self.macros.is_defined(caps.get(2).unwrap().as_str()) {
                        self.state_stack.push(State::WithinTrueBlock);
                    }
                    else {
                        self.state_stack.push(State::NotYetFound);
                    }
                }
                else {
                    self.state_stack.push(State::AlreadyFound);
                }
            }
            "ifndef" => {
                let parent_state = self.state_stack.last().cloned().unwrap_or(State::WithinTrueBlock);

                if parent_state == State::WithinTrueBlock {
                    if !self.macros.is_defined(caps.get(2).unwrap().as_str()) {
                        self.state_stack.push(State::WithinTrueBlock);
                    }
                    else {
                        self.state_stack.push(State::NotYetFound);
                    }
                }
                else {
                    self.state_stack.push(State::AlreadyFound);
                }
            }
            "if" => {
                let parent_state = self.state_stack.last().cloned().unwrap_or(State::WithinTrueBlock);

                if parent_state == State::WithinTrueBlock {
                    if self.macros.expand_condition(caps.get(2).unwrap().as_str()) {
                        self.state_stack.push(State::WithinTrueBlock);
                    }
                    else {
                        self.state_stack.push(State::NotYetFound);
                    }
                }
                else {
                    self.state_stack.push(State::AlreadyFound);
                }
            }
            "elif" => {
                let current_state = self.state_stack.last().cloned().expect("Ill formatted conditional directive");

                if current_state == State::NotYetFound && self.macros.expand_condition(caps.get(2).unwrap().as_str()) {
                    *self.state_stack.last_mut().unwrap() = State::WithinTrueBlock;
                }
                else if current_state == State::WithinTrueBlock {
                    *self.state_stack.last_mut().unwrap() = State::AlreadyFound;
                }
            }
            "else" => {
                let current_state = self.state_stack.last().cloned().expect("Ill formatted conditional directive");

                if current_state == State::WithinTrueBlock {
                    *self.state_stack.last_mut().unwrap() = State::AlreadyFound;
                }
                else if current_state == State::NotYetFound {
                    *self.state_stack.last_mut().unwrap() = State::WithinTrueBlock;
                }
            }
            "endif" => {
                self.state_stack.pop().expect("Ill formatted conditional directives");
            }
            other => {
                panic!("Unrecognized preprocessor directive {:?}", other);
            }
        }
    }


    pub fn feed_line(&mut self, s: &str, line_number: usize) {
        self.input.push_str(s);
        self.process_input(line_number);
    }

    pub fn get_output(&self) -> String {
        self.output.clone()
    }

    pub fn gather_macro_args<'a>(&self, s: &'a str) -> Option<(usize, Vec<&'a str>)> {
        let mut args = Vec::new();
        let mut pdepth = 0; // parenthesis depth
        let mut arg_begin = 0usize;
        let mut arg_end = arg_begin;
        let mut end_of_args = s.len();

        eprintln!("gather_macro_args recieved: {:?}", s);

        for (begin, end) in tokenizer::iter_tokens(String::from(s)) {
            let token = &s[begin..end];

            if token == "(" {
                if pdepth == 0 {
                    arg_begin = end; // make sure the first arg does not include the leading '('
                }
                pdepth += 1;
            }
            else if token == ")" {
                pdepth -= 1;
            }

            if pdepth < 1 {
                if arg_end > arg_begin {
                    args.push(&s[arg_begin..arg_end]);
                }
                end_of_args = end;
                break;
            }
            else if token == "," && pdepth == 1 {
                args.push(&s[arg_begin..arg_end]);
                arg_begin = end;
                arg_end = end;
            }
            else {
                arg_end = end;
            }
        }

        eprintln!("gather_macro_args returning: {:?}", args);

        if pdepth > 0 {
            None
        }
        else {
            Some((end_of_args, args))
        }
    }

    pub fn process_input(&mut self, line_number: usize) {
        let mut buf = String::new();
        let mut cursor = 0usize;
        let mut iter = tokenizer::iter_tokens(self.input.clone());

        while let Some((begin, end)) = iter.next() {
            let token = String::from(&self.input[begin..end]);
            if let Some(macro_val) = self.macros.get(&token) {
                let next_c = self.input.get(end..).and_then(|s| s.chars().next());
                match macro_val {
                    MacroValue::Constant(ref text) => {
                        self.input.replace_range(begin..end, text);
                        iter = tokenizer::iter_tokens(self.input.clone());
                        iter.set_cursor(cursor);
                    }
                    MacroValue::Function(ref text, ref params) => {
                        if next_c == Some('(') {
                            if let Some((offset, args)) = self.gather_macro_args(&self.input[end..].to_string()) {
                                let expanded = expand_function_macro(text, params, args);
                                eprintln!("expanded text is: {:?}", expanded);
                                self.input.replace_range(begin..(end+offset), &expanded);
                                iter = tokenizer::iter_tokens(self.input.clone());
                                iter.set_cursor(cursor);
                            }
                            else {
                                self.input.drain(..cursor);
                                self.output.push_str(&buf);
                                return;
                            }
                        }
                        else {
                            buf.push_str(&self.input[cursor..end]);
                            cursor = end;
                        }
                    }
                }
            }
            else if token == "#" { // stringify macro operator
                if let Some((next_begin, next_end)) = iter.next() {
                    let mut s = String::new();
                    s.push_str(&self.input[cursor..begin]);
                    s.push('"');
                    if let Some(MacroValue::Constant(ref text)) = self.macros.get(&self.input[next_begin..next_end]) {
                        s.push_str(text);
                    }
                    else {
                        s.push_str(&self.input[next_begin..next_end]);
                    }
                    s.push('"');
                    self.input.replace_range(begin..next_end, &s);
                    iter = tokenizer::iter_tokens(self.input.clone());
                    iter.set_cursor(cursor);
                }
            }
            else if token == "##" { // concatenate macro operator
                if let Some((next_begin, next_end)) = iter.next() {
                    let next_token = &self.input[next_begin..next_end];
                    while let Some(c) = buf.pop() {
                        if c != ' ' {
                            buf.push(c);
                            break;
                        }
                    }
                    if let Some(MacroValue::Constant(ref text)) = self.macros.get(&self.input[next_begin..next_end]) {
                        buf.push_str(text);
                    }
                    else {
                        buf.push_str(&self.input[next_begin..next_end]);
                    }
                    cursor = next_end;
                }
            }
            else if token == "__LINE__" {
                eprintln!("__LINE__ token encountered");
                buf.push_str(&self.input[cursor..begin]);
                buf.push_str(&line_number.to_string());
                cursor = end;
            }
            else if token.starts_with("\"") && DOUBLE_QUOTE_RE.is_match(&buf) {
                eprintln!("Encountered two string literals in a row, the second one is: {:?}", token);
                while let Some(c) = buf.pop() {
                    if c == '"' {
                        break;
                    }
                }
                let mut temp = token.to_string();
                temp.remove(0);
                buf.push_str(&temp);
                cursor = end;
            }
            else {
                buf.push_str(&self.input[cursor..end]);
                cursor = end;
            }
        }

        self.output.push_str(&buf);
        self.output.push('\n');
        self.input.clear();
    }

    pub fn preprocess_source(&mut self, source: &str) {
        let directive = Regex::new(r"^\s*#").unwrap();
        //let source = expand_line_wraps(&source);
        //let source = strip_comments(&source);

        for (line_number, ref line) in lineiter::iter_lines(source) {
            if directive.is_match(&line) {
                self.run_directive(&line);
            }
            else if self.state_stack.last().cloned().unwrap_or(State::WithinTrueBlock) == State::WithinTrueBlock {
                self.feed_line(&line, line_number);
            }
        }
    }
}
