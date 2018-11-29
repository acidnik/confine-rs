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
use std::io::{self, BufRead, Write};
use std::fs;
use std::error;

type Result<T> = std::result::Result<T, Box<error::Error>>;

struct Meta<'a> {
    group: &'a Group,
}

impl<'a> Meta<'a> {
    fn add(&self, entry: &PathBuf) -> Result<()> {
        let meta_file = self.group.abs_path().join("meta.txt");
        trace!("{:?} add {:?}", meta_file, entry);
        let entry_str = entry.to_str().unwrap().to_string();
        if ! meta_file.exists() {
            let entry_str = entry.to_str().unwrap().to_string();
            fs::write(&meta_file, entry_str + "\n")?;
            return Ok(());
        }
        let mut lines = io::BufReader::new(fs::File::open(&meta_file)?).lines().map(|l| l.unwrap()).collect::<Vec<_>>();
        let len_before = lines.len();
        lines.push(entry_str);
        lines.sort();
        lines.dedup();
        if lines.len() == len_before {
            trace!("no new entries for meta");
            return Ok(());
        }
        trace!("new meta: {:?}", lines);
        fs::write(&meta_file, lines.join("\n") + "\n")?;

        Ok(())
    }
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
struct Group {
    root: PathBuf,
    dir: PathBuf,
}

impl Group {
    fn new(root: PathBuf, path: &str) -> Result<Self> {
        if let Some(idx) = path.find('/') {
            return Err("Invalid group name")?;
        }
        return Ok(Group { dir: PathBuf::from(path), root: root, });
    }
    fn add_meta(&self, entry: &PathBuf) -> Result<()> {
        let meta = Meta { group: self };
        meta.add(entry)
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
    fn run(&self, group: &Group, files: Vec<PathBuf>) -> Result<()>;
}

struct ActionMove {
    dry: bool,
    home: PathBuf,
}
struct ActionLink {
}

impl Action for ActionMove {
    fn run(&self, group: &Group, files: Vec<PathBuf>) -> Result<()> {
        for file in files {
            debug!("move [{}] {}", group, file.display());
            self.move_file(&group, &file)?;
        }
        Ok(())
    }
}

impl ActionMove {
    fn move_file(&self, group: &Group, file: &PathBuf) -> Result<()> {
        let file = if file.is_relative() {
            self.home.join(file)
        }
        else {
            file.clone()
        };
        trace!("canon {:?}", file);
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
        if ! self.dry {
            group.add_meta(&rel_path)?;
        }

        Ok(())
    }

    fn do_move_file(&self, from: &PathBuf, to: &PathBuf) -> Result<()> {
        let dest_dir = to.parent().unwrap();
        trace!("dest dir = {:?}", dest_dir);
        if ! self.dry {
            fs::create_dir_all(dest_dir)?;
        }
        else {
            warn!("dry: mkdir -p '{}'", dest_dir.display());
        }
        if ! self.dry {
            let from_vec = vec![from];
            fs_extra::copy_items(&from_vec, dest_dir, &fs_extra::dir::CopyOptions::new())?;
            fs_extra::remove_items(&from_vec)?;
            std::os::unix::fs::symlink(&to, &from)?
        }
        else {
            warn!("dry: cp -r '{}' '{}'", from.display(), dest_dir.display());
            warn!("dry: rm -rf '{}'", from.display());
            warn!("dry: ln -s '{}' '{}'", to.display(), from.display());
        }
        Ok(())
    }

    fn get_rel_path(&self, file: &PathBuf) -> Result<(String, PathBuf)> {
        // returns meta entry and relative path
        let home = &self.home;
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
    fn run(&self, group: &Group, files: Vec<PathBuf>) -> Result<()> {
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
    trace!("group_param = {}", group_param);
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

fn parse_args(args: ArgMatches, mut all_groups: &mut Groups) -> Result<(Box<Action>, HashSet<Group>, Vec<PathBuf>)> {
    let dry_run = args.is_present("dry");
    let home = args.value_of("home").map_or(dirs::home_dir().unwrap(), |p| PathBuf::from(p).canonicalize().unwrap());
    let (action, groups, files) : (Box<Action>, _, Vec<PathBuf>) =
    if let Some(matches) = args.subcommand_matches("link") {
        let (files, mut groups) = get_files_from_args(&matches, &mut all_groups);
        (Box::new(ActionLink { }), groups, files)
    }
    else if let Some(matches) = args.subcommand_matches("move") {
        let (files, mut groups) = get_files_from_args(&matches, &mut all_groups);
        (Box::new(ActionMove { dry: dry_run, home: home }), groups, files)
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

fn main() -> Result<()> {
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
        .arg(Arg::with_name("home")
             .long("home")
             .takes_value(true)
             // .hidden(true)
             .help("ovveride home dir")
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
