
use std::borrow::Cow;
use std::cell::Cell;

use regex::Regex;


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

pub fn iter_lines(source: &str) -> impl Iterator<Item=(usize, Cow<str>)> {
    let mut iter = source.lines().enumerate();

    // find a way to collapse two consecutive items in an iterator if a
    // condition is met

    let mut new_iter = iter.scan((Cell::new(String::new()), 0usize), |state, (line_number, line)| {
        let (ref mut acc, ref mut ln) = state;
        if line.ends_with("\\") {
            if acc.get_mut().is_empty() {
                *ln = line_number + 1;
            }
            acc.get_mut().push_str(line.trim_end_matches("\\"));
            //Some((0, Cow::from("")))
            Some(None)
        }
        else if !acc.get_mut().is_empty() {
            acc.get_mut().push_str(line);
            Some(Some((*ln, Cow::from(acc.replace(String::new())))))
        }
        else {
            Some(Some((line_number + 1, Cow::from(line))))
        }
    }).filter_map(|option| option);

    //new_iter.map(|(line, line_number)| {
    //
    //})
    new_iter
}

