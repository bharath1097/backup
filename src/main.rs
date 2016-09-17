extern crate regex;

use std::fs::File;
use std::io::Read;
use std::process::Command;
use regex::Regex;

struct Node {
    name: String,
    sub: Vec<Node>,
}

fn split (s: &str) -> Vec<String> {
    Regex::new(r"\n").unwrap().split(&s).map(|x| x.to_string()).collect()
}

fn read(path: &str) -> Vec<String> {
    let mut file = File::open(path).expect("");
    let mut string = String::new();
    file.read_to_string(&mut string).expect("");
    split(&string)
}

fn find_ignore_args (list: &Vec<String>) -> Vec<String> {
    list.iter().fold(vec![], |mut ret, s| {
        ret.append(&mut vec!["-path".to_string(),
                            s.clone(), 
                            "-prune".to_string(),
                            "-o".to_string()]);
        ret
    })
}

fn main() {
    let data = read("./data/file");
    let add: Vec<String> = data.clone().into_iter()
        .filter(|s| Regex::new(r"^\+").unwrap().is_match(s)).collect();
    let ignore: Vec<String> = data.clone().into_iter()
        .filter(|s| Regex::new(r"^-").unwrap().is_match(s)).collect();
    let readlink = |s: &str| -> String {
        let home = format!("{}", std::env::home_dir().unwrap().display());
        s.replace("~", &home).replace("+ ", "").replace("- ", "")
    };
    let ignore: Vec<String> = ignore.iter().map(|s| readlink(s)).collect();
    let add: Vec<String> = add.iter().map(|s| readlink(s)).collect();
    let git_repo = {
        let ignore = &find_ignore_args(&ignore);
        let mut ret: Vec<String> = vec![];
        for x in &add {
            let s = Command::new("find")
                .arg(x).args(ignore)
                .args(&["-name", ".git", "-print"])
                .output().expect("failed").stdout;
            let s = String::from_utf8(s).expect("failed");
            ret.append(&mut split(&s));
        }
        ret.iter().map(|x| x.replace("/.git", "")).collect()
    };
    let mut files: Vec<String> = vec!{};
    for x in &add {
        let s = Command::new("find")
            .arg(x).args(&find_ignore_args(&ignore)).args(&find_ignore_args(&git_repo))
            .arg("-print")
            .output().expect("").stdout;
        let s = String::from_utf8(s).expect("");
        files.append(&mut split(&s));
    }
}
