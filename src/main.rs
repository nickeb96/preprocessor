
#![allow(unused)]
#![feature(box_syntax, pattern, generators, generator_trait)]

extern crate regex;
extern crate getopts;
extern crate tokenizer;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate indoc;

use std::fs::File;
use std::path::Path;
use std::io::prelude::*;

mod preprocessor;
mod config;
mod macrotable;
mod lineiter;

use preprocessor::PreProcessor;
use config::Config;


fn preprocess_file(file_name: &str, config: &Config) {
    let source = {
        let mut file = File::open(file_name).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).expect(&format!("Unable to read {}", file_name));
        contents
    };

    // you will need to pass this to the preprocessor later when you make it so
    // that headers are included relative to where they are included from
    let _path = Path::new(file_name);

    let mut cpp = PreProcessor::new(config);

    for mac in config.macro_defs.iter() {
        cpp.macros.define_from_arg(mac);
    }

    cpp.preprocess_source(&source);

    println!("{}", cpp.get_output());
}


fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut config = match config::make_config(args) {
        Ok(config) => config,
        Err(string) => {
            eprintln!("{}", string);
            std::process::exit(1);
        }
    };

    if config.help_flag {
        println!("{}", config.opts.usage(&config.opts.short_usage(&config.program_name)));
        return;
    }

    config.add_default_include_dirs();

    for file_name in config.input_files.iter() {
        preprocess_file(file_name, &config);
    }
}

