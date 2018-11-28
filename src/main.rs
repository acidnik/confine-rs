extern crate clap;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate fs_extra;
extern crate dirs;

use clap::{Arg, ArgMatches, App, SubCommand};
use std::path::PathBuf;
use std::collections::{HashSet, HashMap};

use std::fmt;
use std::io;
use std::fs;

#[derive(Debug)]
enum ConfineIoError {
    IO(io::Error),
    IOExtra(fs_extra::error::Error),
}

#[derive(Debug)]
enum ConfineError {
    Generic(String),
    IO(ConfineIoError),
}

impl From<io::Error> for ConfineError {
    fn from(e: io::Error) -> ConfineError {
        ConfineError::IO(ConfineIoError::IO(e))
    }
}

impl From<fs_extra::error::Error> for ConfineError {
    fn from(e: fs_extra::error::Error) -> ConfineError {
        ConfineError::IO(ConfineIoError::IOExtra(e))
    }
}

impl From<String> for ConfineError {
    fn from(e: String) -> ConfineError {
        ConfineError::Generic(e)
    }
}

impl<'a> From<&'a str> for ConfineError {
    fn from(e: &str) -> ConfineError {
        ConfineError::Generic(e.to_string())
    }
}


#[derive(Debug, Hash, Eq, PartialEq, Clone)]
struct Group {
    root: PathBuf,
    dir: PathBuf,
}

impl Group {
    fn new(root: PathBuf, path: &str) -> Result<Self, ConfineError> {
        if let Some(idx) = path.find('/') {
            return Err("Invalid group name")?;
        }
        return Ok(Group { dir: PathBuf::from(path), root: root });
    }
    fn add_meta(&self, entry: &PathBuf) {

    }
    fn abs_path(&self) -> PathBuf {
        return self.root.join(self.dir.clone())
    }
}

#[derive(Debug)]
struct Groups {
    root: PathBuf,
    groups: HashMap<String, Group>,
}

impl fmt::Display for Group {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "{}", self.dir.display())
    }
    
}

impl Groups {
    fn new(root: PathBuf) -> Self {
        Groups {
            root: root,
            groups: HashMap::new(),
        }
    }
    fn is_group(&mut self, root: PathBuf, g: &str) -> Option<Group> {
        if g.len() == 0 {
            return None;
        }
        if let Some(group) = self.groups.get(g) {
            return Some(group.clone());
        }
        let p = self.root.join(g);
        if p.is_dir() {
            let group = Group::new(root, g).unwrap();
            self.groups.insert(g.to_string(), group.clone());
            return Some(group);
        }
        return None;
    }
}

trait Action {
    fn run(&self, group: &Group, files: Vec<PathBuf>) -> Result<(), ConfineError>;
}

struct ActionMove {
    dry: bool,
}
struct ActionLink {
}

impl Action for ActionMove {
    fn run(&self, group: &Group, files: Vec<PathBuf>) -> Result<(), ConfineError> {
        for file in files {
            debug!("move [{}] {}", group, file.display());
            self.move_file(&group, &file)?;
        }
        Ok(())
    }
}

impl ActionMove {
    fn move_file(&self, group: &Group, file: &PathBuf) -> Result<(), ConfineError> {
        let real_file = file.canonicalize()?;
        trace!("real file = {:?}", real_file);
        /*
            if file is inside a home dir - strip_prefix(~) and save relative path
            else - save absolute path, e.g. /etc/bash/bashrc
            in later case create full path under group:
            common/etc/bash/bashrc
            meta.txt:
            /etc/bash/bashrc
        */

        let (meta_entry, rel_path) = self.get_rel_path(&file)?; // relative to $HOME, or absolute, if not in home dir
        trace!("get_rel_path({:?}) => {}, {:?}", file, meta_entry, rel_path);

        let dest = group.abs_path().join(rel_path.clone());

        if dest == real_file {
            warn!("{} already moved, skip", file.display());
            return Ok(());
        }

        if dest.exists() {
            trace!("rel_path = {:?}", rel_path);
            return Err(format!("Can not move file {} to {} ({}): file exists", file.display(), group, dest.display()))?;
        }

        self.do_move_file(&file, &dest)?;
        group.add_meta(&rel_path);

        Ok(())
    }

    fn do_move_file(&self, from: &PathBuf, to: &PathBuf) -> Result<(), ConfineError> {
        let dest_dir = to.parent().unwrap();
        // let dest_dir = if from.is_dir() {
        //     to
        // }
        // else {
        //     to.parent().unwrap()
        // };
        trace!("dest dir = {:?}", dest_dir);
        if ! self.dry {
            fs::create_dir_all(dest_dir)?;
        }
        else {
            warn!("dry: mkdir -p '{}'", dest_dir.display());
        }
        if ! self.dry {
            fs_extra::copy_items(&vec![from], dest_dir, &fs_extra::dir::CopyOptions::new())?;
        }
        else {
            warn!("dry: cp -r '{}' '{}'", from.display(), dest_dir.display());
        }
        Ok(())
    }

    fn get_rel_path(&self, file: &PathBuf) -> Result<(String, PathBuf), ConfineError> {
        // returns meta entry and relative path
        let home = dirs::home_dir().unwrap();
        match file.strip_prefix(home) {
            Ok(rel) => return Ok((rel.display().to_string(), rel.to_path_buf())),
            Err(_) => {},
        }
        return match file.strip_prefix(PathBuf::from("/")) {
            Ok(rel) => Ok((file.display().to_string(), rel.to_path_buf())),
            Err(e) => Err(format!("{}", e))?,
        }
    }
}

impl Action for ActionLink {
    fn run(&self, group: &Group, files: Vec<PathBuf>) -> Result<(), ConfineError> {
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
    else {
        let root = all_groups.root.clone();
        groups.insert(Group::new(root, group_param).unwrap());
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

fn parse_args(args: ArgMatches, mut all_groups: &mut Groups) -> Result<(Box<Action>, HashSet<Group>, Vec<PathBuf>), ConfineError> {
    let dry_run = args.is_present("dry");
    let (action, groups, files) : (Box<Action>, _, Vec<PathBuf>) =
    if let Some(matches) = args.subcommand_matches("link") {
        let (files, mut groups) = get_files_from_args(&matches, &mut all_groups);
        (Box::new(ActionLink { }), groups, files)
    }
    else if let Some(matches) = args.subcommand_matches("move") {
        let (files, mut groups) = get_files_from_args(&matches, &mut all_groups);
        (Box::new(ActionMove { dry: dry_run }), groups, files)
    }
    else {
        return Err("Subcommand missing, see --help")?;
    };

    Ok((action, groups, files))
}

fn get_group_from_file(p: &str, mut all_groups: &mut Groups) -> (Option<Group>, PathBuf) {
    if let Some(idx) = p.find('/') {
        let dir = &p[0..idx];
        let root = all_groups.root.clone();
        if let Some(group) = all_groups.is_group(root, dir) {
            let pf = PathBuf::from(&p[(idx+1)..]);
            // let group = Group::new(root, dir).unwrap();
            return (Some(group), pf);
        }
    }
    (None, PathBuf::from(p))
}

fn init_logger(quiet: bool) {
    let mut builder = env_logger::Builder::from_default_env();
    let level = if quiet { log::LevelFilter::Error } else { log::LevelFilter::Debug };
    let level = log::LevelFilter::Trace; // XXX

    builder.filter_level(level).init();
}

fn main() -> Result<(), ConfineError> {
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
    

    let dry = matches.is_present("dry");
    let quiet = matches.is_present("quiet") && ! dry;
    init_logger(quiet);

    let root = PathBuf::from(matches.value_of("root").unwrap()).canonicalize().unwrap();
    let mut all_groups = Groups::new(root);
    let (action, groups, files) = parse_args(matches, &mut all_groups)?;
    trace!("group = {:?}, files = {:?}", groups, files);
    if groups.len() != 1 {
        return Err(format!("too many groups: {:?}", groups))?
    }
    let group = groups.iter().take(1).collect::<Vec<_>>()[0];
    action.run(group, files)?;
    Ok(())
}
