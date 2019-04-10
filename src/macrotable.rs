
use std::collections::HashMap;
use regex::{Regex, Captures};
//use tokenizer::iter_tokens;

#[derive(Debug)]
pub enum MacroValue {
    Constant(String),
    //Function(String, Vec<String>),
    Function(String, HashMap<String, usize>),
}

/*impl MacroValue {
    pub fn expand_function(&self, arguments: &Vec<&str>) -> Option<String> {
        match self {
            &MacroValue::Function(ref contents, ref parameters) => {
                let mut ret = String::new();

                for token in iter_tokens(contents) {
                    if let Some(&index) = parameters.get(token) {
                        ret.push_str(arguments[index]);
                        ret.push(' ');
                    }
                    else {
                        ret.push_str(token);
                        ret.push(' ');
                    }
                }

                Some(ret)
            }
            _ => None,
        }
    }
}*/

#[derive(Debug)]
pub struct MacroTable {
    pub map: HashMap<String, MacroValue>,
}

impl MacroTable {
    pub fn new() -> MacroTable {
        MacroTable { map: HashMap::new() }
    }

    pub fn define_from_arg(&mut self, arg: &str) {
        let re = Regex::new(r"=").unwrap();
        let line = re.replace(arg, " ");
        self.define(&line);
    }

    pub fn define_constant(&mut self, line: &str) {
        let re = Regex::new(r"^\s*([a-zA-Z_]+)\s*(.*)?$").unwrap();
        let caps = re.captures(line).unwrap();
        let name = caps.get(1).unwrap().as_str();
        let contents = caps.get(2).map_or("", |mat| mat.as_str());

        self.map.insert(name.to_string(),
                        MacroValue::Constant(contents.to_string()));
    }

    pub fn define_function(&mut self, line: &str) {
        let re = Regex::new(r"^\s*([a-zA-Z_]+)\(([^)]*)\)\s*(.*)?$").unwrap();
        let arg_splitter = Regex::new(r",").unwrap();
        let caps = re.captures(line).unwrap();
        let name = caps.get(1).unwrap().as_str();
        let args = caps.get(2).unwrap().as_str();
        let contents = caps.get(3).map_or("", |mat| mat.as_str());

        let mut arg_map = HashMap::new();

        for (index, arg) in arg_splitter.split(args).map(|s| s.to_string()).enumerate() {
            arg_map.insert(arg.trim().to_string(), index);
        }

        self.map.insert(name.to_string(),
                        MacroValue::Function(contents.to_string(), arg_map));
    }

    pub fn define(&mut self, line: &str) {
        let re = Regex::new(r"^\s*([a-zA-Z_]+)\(").unwrap();
        if re.is_match(line) {
            self.define_function(line);
        }
        else {
            self.define_constant(line);
        }
    }

    pub fn undef(&mut self, macro_name: &str) {
        self.map.remove(macro_name);
    }

    pub fn is_defined(&self, macro_name: &str) -> bool {
        self.map.contains_key(macro_name)
    }

    pub fn get(&self, macro_name: &str) -> Option<&MacroValue> {
        self.map.get(macro_name)
    }

    pub fn expand_constant(&self, macro_name: &str) -> Option<String> {
        self.map.get(macro_name).and_then(|value|
            match value {
                &MacroValue::Constant(ref s) => Some(s.clone()),
                _ => None,
        })
    }

    /*pub fn expand_function(&self, macro_name: &str, args: Vec<&str>) -> Option<String> {
        match self.map.get(macro_name) {
            Some(&MacroValue::Function(ref s, ref arg_names)) => {
                let mut ret = String::new();
                for token in iter_tokens(s) {
                    if let Some(&index) = arg_names.get(token) {
                        ret.push_str(args[index]);
                        ret.push(' ');
                    }
                    else {
                        ret.push_str(token);
                        ret.push(' ');
                    }
                }
                Some(ret)
            }
            Some(&MacroValue::Constant(ref s)) => {
                Some(s.clone())
            }
            None => None,
        }
    }*/

    pub fn expand_line2(&self, line: &str) -> String {
        let re = Regex::new(r"([a-zA-Z_][a-zA-Z_0-9]*)").unwrap();

        re.replace_all(line, |caps: &Captures| {
            match self.map.get(caps.get(1).unwrap().as_str()) {
                Some(&MacroValue::Constant(ref s)) => s.clone(),
                _ => String::from(caps.get(1).unwrap().as_str()),
            }
        }).into_owned()
    }

    #[cfg(no)]
    fn expand_str(&self, line: &str) -> String {
        let mut ret = String::new();
        let mut iter = line.char_indices().peekable();
        let mut prev_c = ' ';
        let mut in_quotes = false;
        while let Some((i, c)) = iter.next() {
            if c == '"' && prev_c != '\\' {
                in_quotes = !in_quotes;
                continue;
            }
            if in_quotes {
                continue;
            }
            let mut found_macro = false;
            for macro_name in self.map.keys() {
                if line[i..].starts_with(macro_name) {
                    found_macro = true;
                    // skip the macro name from being added to the return
                    for j in 0..macro_name.chars().count() {
                        iter.next();
                    }
                    // check if macro is a function or constant
                    match iter.peek() {
                        Some(&(_, '(')) => {
                            match self.map[macro_name] {
                                MacroValue::Constant(val) => {
                                    ret.push_str(val);
                                }
                                MacroValue::Function(ref val, ref args) => {
                                    let mut args = Vec::<&str>::new();

                                    ret.push_str(self.map[macro_name].expand_function(&args).unwrap());
                                }
                            }
                        }
                        _ => {
                            match self.map[macro_name] {
                                MacroValue::Constant(val) => {
                                    ret.push_str(val);
                                }
                                MacroValue::Function(ref _val, ref _args) => {
                                    ret.push_str(macro_name);
                                }
                            }
                        }
                    }
                    break;
                }
            }
            if !found_macro {
                ret.push(c);
            }
            prev_c = c;
        }
        ret
    }

    #[cfg(no)]
    pub fn expand_line(&self, line: &str) -> String {
        let mut ret = String::new();
        let mut iter = iter_tokens(line);

        while let Some(token) = iter.next() {
            if let Some(macro_value) = self.map.get(token) {
                match macro_value {
                    &MacroValue::Constant(ref s) => {
                        ret.push_str(s);
                        ret.push(' ');
                    }
                    &MacroValue::Function(ref s, ref args) => {
                        ret.push_str(s); // this is not right, its just temporary
                        ret.push(' ');
                    }
                }
            }
            else {
                ret.push_str(token);
                ret.push(' ');
            }
        }

        ret.push('\n');
        ret
    }

    pub fn expand_line(&self, line: &str) -> String {
        String::from(line)
    }

    pub fn expand_condition(&self, condition: &str) -> bool {
        if let Ok(num) = condition.parse::<i64>() {
            return num != 0;
        }

        true
    }
}
