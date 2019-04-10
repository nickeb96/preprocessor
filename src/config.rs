
use std;
use getopts;

pub fn make_options() -> getopts::Options {
    let mut opts = getopts::Options::new();

    opts.optflag(
        "h",
        "help",
        "display help and exit",
    );

    opts.optflag(
        "V",
        "version",
        "display version and exit",
    );

    opts.optflag(
        "v",
        "verbose",
        "run in verbose mode",
    );

    opts.optflag(
        "q",
        "quiet",
        "run in quiet mode",
    );

    opts.opt(
        "",
        "multi-threaded",
        "preprocess each source file simultaneously in a different thread",
        "",
        getopts::HasArg::No,
        getopts::Occur::Optional,
    );

    opts.opt(
        "",
        "single-threaded",
        "preprocess each source file one after another",
        "",
        getopts::HasArg::No,
        getopts::Occur::Optional,
    );

    opts.opt(
        "I",
        "include-path",
        "specify include directories",
        "INCLUDE_DIR",
        getopts::HasArg::Yes,
        getopts::Occur::Multi,
    );

    opts.opt(
        "D",
        "define",
        "define macro",
        "MACRO_NAME[=value]",
        getopts::HasArg::Yes,
        getopts::Occur::Multi,
    );

    opts
}


pub fn make_config(args: Vec<String>) -> Result<Config, String> {
    //let args: Vec<String> = std::env::args().collect();
    let mut opts = make_options();
    let mut config = Config::new();

    config.program_name = args[0].clone();

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(e) => {
            return Err(e.to_string());
        }
    };

    config.opts = opts;

    if matches.opt_present("help") {
        config.help_flag = true;
        return Ok(config);
    }

    config.include_dirs = matches.opt_strs("include-path").into_iter().map(|s| std::path::PathBuf::from(s)).collect();
    config.macro_defs = matches.opt_strs("define");
    config.input_files = matches.free;

    Ok(config)
}


pub struct Config {
    pub opts: getopts::Options,
    pub program_name: String,
    pub help_flag: bool,
    pub include_dirs: Vec<std::path::PathBuf>,
    pub macro_defs: Vec<String>,
    pub input_files: Vec<String>,
}


impl Config {
    pub fn new() -> Config {
        Config {
            opts: getopts::Options::new(),
            program_name: String::new(),
            help_flag: false,
            include_dirs: Vec::new(),
            macro_defs: Vec::new(),
            input_files: Vec::new(),
        }
    }

    pub fn add_default_include_dirs(&mut self) {
        let default_include_dirs = ["/usr/include", "/usr/local/include"];

        for dir in default_include_dirs.iter() {
            self.include_dirs.push(std::path::PathBuf::from(dir));
        }
    }
}

impl Default for Config {
    fn default() -> Config {
        Config::new()
    }
}

