extern crate regex;

use std::fs::File;
use std::io::Read;
use std::process::Command;
use regex::Regex;
use std::cell::{RefCell, Ref, RefMut};

#[derive(Debug)]
struct Node {
    name: String,
    sym: bool,
    is_git: bool,
    sub: Vec<RefCell<Node>>,
}

impl Node {
    fn new(name: &str) -> RefCell<Node> {
        RefCell::new(Node {
            name: name.to_string(),
            sym: false,
            is_git: false,
            sub: vec![],
        })
    }
    fn add_son(&mut self, son: RefCell<Node>) {
        self.sub.push(son);
    }
    fn get_son(&self, name: &str) -> Option<Ref<Node>> {
        self.sub.iter().find(|x| x.borrow().name == name).map(|x| x.borrow())
    }
    fn get_mut_son(&self, name: &str) -> Option<RefMut<Node>> {
        self.sub.iter().find(|x| x.borrow().name == name).map(|x| x.borrow_mut())
    }
}

fn split (s: &str, pattern: &str) -> Vec<String> {
    Regex::new(pattern).unwrap().split(&s).map(|x| x.to_string()).collect()
}

fn readlink(s: &str) -> String {
    let home = format!("{}", std::env::home_dir().unwrap().display());
    s.replace("~", &home).replace("+ ", "").replace("- ", "")
}

fn read(path: &str) -> Vec<String> {
    let mut file = File::open(path).unwrap();
    let mut string = String::new();
    file.read_to_string(&mut string).unwrap();
    split(&string, r"\n")
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

fn extend<'a, T>(mut node: RefMut<Node>, mut iter: T)
where T: Iterator<Item = &'a String> {
    let name = iter.next();
    if let None = name {
        return ();
    }
    let name = name.unwrap();
    if node.get_son(&name).is_none() {
        node.add_son(Node::new(&name));
    }
    extend(node.get_mut_son(&name).expect("get mut error"), iter);
}

fn make_tree(root: RefCell<Node>, list: &Vec<String>) -> RefCell<Node> {
    for x in list {
        let list = split(&x, "/");
        let iter = list.iter().skip(1);
        extend(root.borrow_mut(), iter);
    }
    root
}

fn chdir(s: &str) -> Result<(), std::io::Error> {
    std::env::set_current_dir(std::path::Path::new(s))
}
fn pwd() -> String {
    format!("{}", std::env::current_dir().unwrap().display())
}

fn travel(node: Ref<Node>, root: &str, path: &str) {
    let path = &format!("{}/{}", path, node.name);
    println!("{}", path);
    if node.sub.is_empty() {
        let real_path = path.to_string().replace(root, "");
        Command::new("ln").args(&["-s", &real_path, path]).status().expect("ln error");
    } else {
        Command::new("mkdir").arg(path).status().expect("mkdir error");
        for x in &node.sub {
            travel(x.borrow(), root, path);
        }
    }
}

fn output(node: Ref<Node>) {
    let outdir = format!("{}/output", pwd());
    println!("{}", outdir);
    Command::new("rm").args(&["-R", &outdir]).status();
    Command::new("mkdir").arg(&outdir).status();
    travel(node, &outdir, &outdir);
}

fn main() {
    let data = read("./data/file");
    let add: Vec<String> = data.clone().into_iter()
        .filter(|s| Regex::new(r"^\+").unwrap().is_match(s))
        .map(|s| readlink(&s)).collect();
    let ignore: Vec<String> = data.clone().into_iter()
        .filter(|s| Regex::new(r"^-").unwrap().is_match(s))
        .map(|s| readlink(&s)).collect();

    let git_repo = {
        let ignore = &find_ignore_args(&ignore);
        let mut ret: Vec<String> = vec![];
        for x in &add {
            let s = Command::new("find")
                .arg(x).args(ignore)
                .args(&["-name", ".git", "-print"])
                .output().expect("failed").stdout;
            let s = String::from_utf8(s).expect("failed");
            ret.append(&mut split(&s, r"\n"));
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
        files.append(&mut split(&s, r"\n"));
    }
    let root = make_tree(Node::new(""), &files);

    let (git, submodule): (Vec<_>, Vec<_>) = git_repo.clone().into_iter().partition(|x| {
        git_repo.iter().fold(true, |boo, y| boo & (!x.starts_with(y) | (&x == &y)))
    });
    let get_git_files = |path: &str| -> Vec<String> {
        let pwd = pwd();
        chdir(path).expect("change dir failed");
        let stdout = Command::new("git").arg("ls-files").output().expect("git ls-files").stdout;
        chdir(&pwd);
        String::from_utf8(stdout).unwrap()
            .lines().map(|x| format!("{}/{}", path, x)).collect()
    };
    let mut git_files = git.iter().fold(vec![], |mut vec, x| {
        vec.append(&mut get_git_files(x));
        vec
    });
    for x in &submodule {
        if git_files.iter().find(|y| y == &x).is_some() {
            git_files.append(&mut get_git_files(x));
        }
    }
    let root = make_tree(root, &git_files);
    output(root.borrow());
}
