extern crate clap;

use clap::{Arg, ArgMatches, App, SubCommand};
use std::path::PathBuf;
use std::collections::{HashSet, HashMap};



#[derive(Debug, Hash, Eq, PartialEq)]
struct Group {
    dir: PathBuf,
}

#[derive(Debug)]
struct Groups {
    root: PathBuf,
    groups: HashMap<String, Group>,
}

impl Groups {
    fn new(root: PathBuf) -> Self {
        Groups {
            root: root,
            groups: HashMap::new(),
        }
    }
    fn is_group(&mut self, g: &str) -> bool {
        if self.groups.contains_key(g) {
            return true;
        }
        let p = self.root.join(g);
        if p.is_dir() {
            self.groups.insert(g.to_string(), Group { dir: PathBuf::from(p) });
            return true;
        }
        return false;
    }
}

trait Action {
    fn run(&self, group: &Group, files: Vec<PathBuf>) -> Result<(), String>;
}

struct ActionMove {
    quiet: bool,
}
struct ActionLink {
    // because who needs inheritance, right, rust?
    quiet: bool,
}

impl Action for ActionMove {
    fn run(&self, group: &Group, files: Vec<PathBuf>) -> Result<(), String> {
        for file in files {
            self.move_file(&group, &file)?;
        }
        Ok(())
    }
}

impl ActionMove {
    fn move_file(&self, group: &Group, file: &PathBuf) -> Result<(), String> {

        // let file = file.absolute();
        // let real_file = get_real_file(file);
        // if file != real_file {
        //     // file is a symlink
        //     eprintln!("file {} was a symlink to {}", file.display(), real_file.display());
        // }
        // let rel_path = file->relative(self.home());
        // group.meta_add(rel_path);

        Ok(())
    }
}

impl Action for ActionLink {
    fn run(&self, group: &Group, files: Vec<PathBuf>) -> Result<(), String> {
        Ok(())
    }
}

fn get_files_from_args(matches: &ArgMatches, mut all_groups: &mut Groups) -> (Vec<PathBuf>, HashSet<Group>) {
    // TODO allow full path in group or file, get absolute path and remove root

    let arg_files = matches.values_of("files");
    let files = match matches.values_of("files") {
        Some(files) => files.map(|f| f.to_string()).collect(),
        None => Vec::new(),
    };
    
    let mut groups = HashSet::new();
    let mut new_files = Vec::new();
    
    // check if group is actually a group/file
    let group_param = matches.value_of("group").unwrap();
    let (group, group_file) = get_group_from_file(&group_param, &mut all_groups);
    if let Some(group) = group {
        groups.insert(group);
        new_files.push(group_file);
    }

    // check if any file is actually a group/file
    for file in files {
        let (group, new_path) = get_group_from_file(&file, &mut all_groups);
        new_files.push(new_path); // new_path == path if group is none
        if let Some(group) = group {
            groups.insert(group);
        }
    }
    
    (new_files, groups)
}

fn parse_args(args: ArgMatches, mut all_groups: &mut Groups) -> Result<(Box<Action>, HashSet<Group>, Vec<PathBuf>), String> {
    let quiet = args.is_present("quiet");
    let (action, groups, files) : (Box<Action>, _, Vec<PathBuf>) =
    if let Some(matches) = args.subcommand_matches("link") {
        let (files, mut groups) = get_files_from_args(&matches, &mut all_groups);
        (Box::new(ActionLink { quiet: quiet }), groups, files)
    }
    else if let Some(matches) = args.subcommand_matches("move") {
        let (files, mut groups) = get_files_from_args(&matches, &mut all_groups);
        (Box::new(ActionMove { quiet: quiet }), groups, files)
    }
    else {
        return Err("Subcommand missing, see --help".to_string());
    };

    Ok((action, groups, files))
}

fn get_group_from_file(p: &str, mut all_groups: &mut Groups) -> (Option<Group>, PathBuf) {
    if let Some(idx) = p.find('/') {
        let dir = &p[0..idx];
        if all_groups.is_group(dir) {
            let p = PathBuf::from(&p[(idx+1)..]);
            let dir = PathBuf::from(dir);
            return (Some(Group{dir}), p);
        }
    }
    (None, PathBuf::from(p))
}

fn main() -> Result<(), String> {
    let matches = App::new("confine")
        .version("0.0.1")
        .author("Nikita Bilous <nikita@bilous.me>")
        .about("config file manager")
        .arg(Arg::with_name("quiet")
             .short("q")
             .help("be quiet")
        )
        .arg(Arg::with_name("dry")
             .short("n")
             .help("dry run")
        )
        .arg(Arg::with_name("root")
             .short("r")
             .default_value(".")
             .help("config storage root")
        )
        .subcommand(SubCommand::with_name("move")
            .aliases(&["mv"])
            .about("move file under config control")
            .arg(Arg::with_name("group")
                 .index(1)
                 .required(true)
                 .help("group")
            )
            .arg(Arg::with_name("files")
                 .multiple(true)
            )
        )
        .subcommand(SubCommand::with_name("link")
            .aliases(&["ln"])
            .about("create symlink")
            .arg(Arg::with_name("group")
                 .index(1)
                 .required(true)
                 .help("group")
            )
            .arg(Arg::with_name("files")
                 .multiple(true)
            )
        )
        .get_matches();
    // let confine = Confine::new(matches);
    let root = PathBuf::from(matches.value_of("root").unwrap()).canonicalize().unwrap();
    let mut all_groups = Groups::new(root);
    let (action, groups, files) = parse_args(matches, &mut all_groups)?;
    println!("{:?}   {:?}", groups, files);
    // let (groups, files) = parse_files(args);
    if groups.len() != 1 {
        return Err(format!("too many groups: {:?}", groups))
    }
    Ok(())
}
