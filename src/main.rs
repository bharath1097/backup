extern crate regex;

use std::env;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::process::Command;
use std::collections::BTreeMap;
use std::time::SystemTime;
use regex::Regex;
use std::cell::{RefCell, Ref, RefMut};

#[derive(Debug)]
struct Node {
    name: String,
    sym: bool,
    sub: Vec<RefCell<Node>>,
}
type Database = BTreeMap<String, SystemTime>;

impl Node {
    fn new(name: &str, sym: bool) -> RefCell<Node> {
        RefCell::new(Node {
            name: name.to_string(),
            sym: sym,
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
    let home = format!("{}", env::home_dir().unwrap().display());
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

fn extend<'a, T>(mut node: RefMut<Node>, mut iter: T, deep: usize, base_deep: usize)
where T: Iterator<Item = &'a String> {
    let name = iter.next();
    if let None = name {
        return ();
    }
    let name = name.unwrap();
    if node.get_son(&name).is_none() {
        node.add_son(Node::new(&name, deep>=base_deep));
    }
    extend(node.get_mut_son(&name).expect("get mut error"), iter, deep+1, base_deep);
}

fn make_tree(root: RefCell<Node>, list: &Vec<String>, deep: &Vec<usize>) -> RefCell<Node> {
    let mut deep_iter = deep.iter();
    for x in list {
        let list = split(&x, "/");
        let iter = list.iter().skip(1);
        extend(root.borrow_mut(), iter, 0, *deep_iter.next().unwrap_or(&usize::max_value()));
    }
    root
}

fn chdir(s: &str) -> Result<(), std::io::Error> {
    env::set_current_dir(std::path::Path::new(s))
}
fn pwd() -> String {
    format!("{}", env::current_dir().unwrap().display())
}

fn travel(mut node: RefMut<Node>, root: &str, path: &str) {
    let path = &format!("{}/{}", path, node.name);
    if node.sym {
        node.sub.clear();
    }
    if node.sub.is_empty() {
        let real_path = path.to_string().replace(root, "");
        Command::new("ln").args(&["-s", &real_path, path]).status().expect("ln error");
    } else {
        Command::new("mkdir").arg(path).status().expect("mkdir error");
        for x in &node.sub {
            travel(x.borrow_mut(), root, path);
        }
    }
}

fn output(node: RefMut<Node>) {
    let outdir = format!("{}/output", pwd());
    Command::new("rm").args(&["-R", &outdir]).status().unwrap();
    travel(node, &outdir, &outdir);
}

fn flag<'a, T>(mut node: RefMut<Node>, mut iter: T)
where T: Iterator<Item = &'a String> {
    node.sym = false;
    let name = iter.next();
    match node.get_mut_son(name.unwrap()) {
        None => (),
        Some(x) => flag(x, iter),
    }
}

trait DatabaseTrait {
    fn from_list(files: &Vec<String>) -> Self;
    fn read(path: &str) -> Self;
    fn write(&self, path: &str) -> Result<(), ()>;
}

impl DatabaseTrait for Database {
    fn from_list(files: &Vec<String>) -> BTreeMap<String, SystemTime> {
        let mut ret = BTreeMap::new();
        for x in files {
            let time = File::open(x).unwrap().metadata().unwrap().modified().unwrap();
            ret.insert(x.to_string(), time);
        }
        ret
    }
    fn read(path: &str) -> BTreeMap<String, SystemTime> {
        let mut ret = BTreeMap::new();
        let mut file = File::open(path).unwrap();
        let mut s = String::new();
        file.read_to_string(&mut s).unwrap();
        let regex = Regex::new(r"^(\d+) (\d+) (.+)$").unwrap();
        for s in s.lines() {
            let cap = regex.captures_iter(s).next().unwrap();
            let (a, b, s) = (cap.at(1).unwrap(), cap.at(2).unwrap(), cap.at(3).unwrap());
            let a = a.parse::<u64>().unwrap();
            let b = b.parse::<u64>().unwrap();
            let time: SystemTime = unsafe{ std::mem::transmute((a, b)) };
            ret.insert(s.to_string(), time);
        }
        ret
    }
    fn write(&self, path: &str) -> Result<(), ()> {
        let mut file = File::create(path).unwrap();
        for (key, value) in self {
            let (a, b): (u64, u64) = unsafe { std::mem::transmute_copy(value) };
            writeln!(file, "{} {} {}", a, b, key).unwrap();
        }
        Ok(())
    }
}

fn main() {
    let file = env::args().nth(1).unwrap_or("file".to_string());
    let data = read(&format!("./data/{}", file));
    let add: Vec<String> = data.clone().into_iter()
        .filter(|s| Regex::new(r"^\+").unwrap().is_match(s))
        .map(|s| readlink(&s)).collect();
    let ignore: Vec<String> = data.clone().into_iter()
        .filter(|s| Regex::new(r"^-").unwrap().is_match(s))
        .map(|s| readlink(&s)).collect();

    let git_repo: Vec<String> = {
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

    // remove git repo from add
    let add: Vec<String> = add.into_iter()
        .filter(|x| git_repo.iter().find(|y| y == &x).is_none())
        .collect();

    let mut files: Vec<String> = vec![];
    let mut deep: Vec<usize> = vec![];
    for x in &add {
        let s = Command::new("find")
            .arg(x).args(&find_ignore_args(&ignore)).args(&find_ignore_args(&git_repo))
            .arg("-print")
            .output().expect("").stdout;
        let s = String::from_utf8(s).expect("");
        let mut new = split(&s, r"\n");
        let tmp = x.matches("/").count() - 1;
        let mut newdeep: Vec<_> = new.iter().map(|_| tmp).collect();
        files.append(&mut new);
        deep.append(&mut newdeep)
    }
    let root = make_tree(Node::new("", false), &files, &deep);

    for x in &ignore {
        let list = split(x, "/");
        let iter = list.iter().skip(1);
        flag(root.borrow_mut(), iter);
    }

//{{{ add git files
    let (git, submodule): (Vec<_>, Vec<_>) = git_repo.clone().into_iter().partition(|x| {
        git_repo.iter().fold(true, |boo, y| boo & (!x.starts_with(y) | (&x == &y)))
    });
    let get_git_files = |path: &str| -> Vec<String> {
        let pwd = pwd();
        chdir(path).expect("change dir failed");
        let stdout = Command::new("git").arg("ls-files").output().expect("git ls-files").stdout;
        chdir(&pwd).expect("chdir error");
        let mut ret: Vec<_> = String::from_utf8(stdout).unwrap()
            .lines().map(|x| format!("{}/{}", path, x)).collect();
        let find = Command::new("find").arg(format!("{}/.git", path))
            .output().unwrap().stdout;
        ret.append(&mut split(&String::from_utf8(find).unwrap(), r"\n"));
        ret
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
    let root = make_tree(root, &git_files, &vec![]);
//}}}

    output(root.borrow_mut());
}
