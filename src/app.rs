use clap::{ArgMatches};

use std::path::PathBuf;
use std::collections::{HashSet, HashMap};

use std::fmt;
use std::io::{self, BufRead};
use std::fs;
use std::error;

struct Meta {
    meta_file: PathBuf,
    entries: Vec<String>,
}

impl Meta {
    fn new(group: &Group) -> Result<Self> {
        let meta_file = group.abs_path().join("meta.txt");
        let entries = if meta_file.exists() {
            io::BufReader::new(fs::File::open(PathBuf::from(&meta_file))?)
                .lines()
                .map(|l| l.unwrap())
                .collect::<Vec<_>>()
        }
        else {
            Vec::new()
        };
        Ok(Self {meta_file, entries})
    }
    fn add(&self, entry: &PathBuf) -> Result<()> {
        let meta_file = &self.meta_file;
        trace!("{:?} add {:?}", meta_file, entry);
        let entry_str = entry.to_str().unwrap().to_string();
        if ! meta_file.exists() {
            let entry_str = entry.to_str().unwrap().to_string();
            fs::write(&meta_file, entry_str + "\n")?;
            return Ok(());
        }
        let mut lines = self.list()?;
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
    fn list(&self) -> Result<Vec<String>> {
        Ok(self.entries.clone())
    }
    fn check(&self, entry: &PathBuf) -> bool {
        // is entry in meta.txt?
        self.entries.iter().find(|&e| &PathBuf::from(e) == entry).is_some()
    }
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
struct Group {
    root: PathBuf,
    dir: PathBuf,
}

impl Group {
    fn new(root: PathBuf, path: &str) -> Result<Self> {
        if let Some(_) = path.find('/') {
            return Err("Invalid group name")?;
        }
        if path == "backup" {
            return Err("Forbidden group name")?;
        }
        return Ok(Group { dir: PathBuf::from(path), root: root, });
    }
    fn add_meta(&self, entry: &PathBuf) -> Result<()> {
        let meta = Meta::new(&self)?;
        meta.add(entry)
    }
    fn abs_path(&self) -> PathBuf {
        return self.root.join(self.dir.clone())
    }
}

impl fmt::Display for Group {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "{}", self.dir.display())
    }
}


type Result<T> = std::result::Result<T, Box<error::Error>>;

pub struct Confine {
    dry: bool,
    home: PathBuf,
    root: PathBuf,
    groups: HashMap<String, Group>,
}

impl Confine {
    pub fn new(matches: &ArgMatches) -> Self {
        let dry = matches.is_present("dry");
        let quiet = matches.is_present("quiet") && ! dry;
        let trace = matches.is_present("trace");
        Self::init_logger(quiet, trace);
    
        let root = PathBuf::from(matches.value_of("root").unwrap()).canonicalize().unwrap();
        let home = matches.value_of("home").map_or(dirs::home_dir().unwrap(), |p| PathBuf::from(p).canonicalize().unwrap());

        Self {
            dry: dry,
            home: home,
            root: root,
            groups: HashMap::new(),
        }
    }

    fn init_logger(quiet: bool, trace: bool) {
        let mut builder = env_logger::Builder::from_default_env();
        let level = match (quiet, trace) {
            (_, true) => log::LevelFilter::Trace,
            (true, _) => log::LevelFilter::Error,
            (false, _) => log::LevelFilter::Debug
        };

        builder.filter_level(level).init();
    }

    pub fn run(&mut self, matches: &ArgMatches) -> Result<()> {
        if let Some(matches) = matches.subcommand_matches("link") {
           let (files, group) = self.get_files_from_args(&matches)?;
           self.link_files(group, files)
        }
        else if let Some(matches) = matches.subcommand_matches("move") {
           let (files, group) = self.get_files_from_args(&matches)?;
           self.move_files(group, files)
        }
        else {
           return Err("Subcommand missing, see --help")?
        }
    }

    fn link_files(&self, group: Group, files: Vec<PathBuf>) -> Result<()> {
        let meta = Meta::new(&group)?;
        let files = if files.len() > 0 {
            files
        }
        else {
            meta.list()?.into_iter().map(|f| PathBuf::from(f)).collect()
        };
        for file in files {
            debug!("link [{}] {}", group, file.display());
            if ! meta.check(&file) {
                Err(format!("file {} not in meta.txt", file.display()))?
            }
            self.link_file(&group, &file)?;
        }
        Ok(())
    }
    fn link_file(&self, group: &Group, file: &PathBuf) -> Result<()> {
        let src = group.abs_path().join(file);
        let dest = if file.is_relative() {
            self.home.join(file)
        }
        else {
            file.clone()
        };
        if dest.exists() {
            let destd = dest.display();
            warn!("link: destination file {} exists", destd);
            if fs::symlink_metadata(&dest)?.file_type().is_symlink() {
                let dest_canon = dest.canonicalize()?;
                if dest_canon == src {
                    warn!("{} is already a link to {}", src.display(), destd);
                    return Ok(());
                }
                warn!("link: destination file {} is symlink, removing", destd);
                if ! self.dry {
                    fs::remove_file(&dest)?
                }
                else {
                    warn!("dry: rm '{}'", destd);
                }
            }
            else {
                warn!("creating backup for {} before erasing", destd);
                self.backup_file(&group, &dest)?;
                if ! self.dry {
                    warn!("rm -rf '{}'", destd);
                    if dest.is_dir() {
                        fs::remove_dir_all(&dest)?
                    }
                    else {
                        fs::remove_file(&dest)?
                    }
                }
                else {
                    warn!("dry: rm -rf '{}'", destd);
                }
            }
        }
        trace!("link {:?} -> {:?}", src, dest);
        if ! self.dry {
            std::os::unix::fs::symlink(&src, &dest)?;
        }
        else {
            warn!("dry: ln -s '{}' '{}'", src.display(), dest.display());
        }

        Ok(())
    }

    fn backup_file(&self, group: &Group, path: &PathBuf) -> Result<()> {
        let path = path.canonicalize()?;
        let rel_path = path.strip_prefix(self.home.clone())?.to_owned();
        let hostname = hostname::get_hostname().unwrap();
        let backup_dest = group.root.join("backup").join(&hostname).join(&rel_path).parent().unwrap().to_owned();
        if ! self.dry {
            fs::create_dir_all(&backup_dest)?
        }
        else {
            warn!("dry: mkdir -p '{}'", backup_dest.display());
        }
        trace!("backup {:?} to {:?}", path, backup_dest);
        fs_extra::copy_items(&vec![path], backup_dest, &fs_extra::dir::CopyOptions::new())?;

        Ok(())
    }
    fn move_files(&self, group: Group, files: Vec<PathBuf>) -> Result<()> {
        for file in files {
            debug!("move [{}] {}", group, file.display());
            self.move_file(&group, &file)?;
        }
        Ok(())
    }
    
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
            std::os::unix::fs::symlink(&to, &from)?;
        }
        else {
            warn!("dry: cp -r '{}' '{}'", from.display(), dest_dir.display());
            warn!("dry: rm -rf '{}'", from.display());
            warn!("dry: ln -s '{}' '{}'", to.display(), from.display());
        }
        Ok(())
    }

    fn get_rel_path(&self, file: &PathBuf) -> Result<(String, PathBuf)> {
        // returns meta entry and relative path (relative to home or root dir)
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

    fn get_files_from_args(&mut self, matches: &ArgMatches) -> Result<(Vec<PathBuf>, Group)> {
        let files = match matches.values_of("files") {
            Some(files) => files.map(|f| f.to_string()).collect(),
            None => Vec::new(),
        };
        
        let mut groups = HashSet::new();
        let mut new_files = Vec::new();
        
        // check if group is actually a group/file
        let group_param = matches.value_of("group").unwrap();
        let (group, group_file) = self.get_group_from_file(&group_param);
        if let Some(group) = group {
            groups.insert(group);
            new_files.push(group_file);
        }
        else {
            let root = self.root.clone();
            groups.insert(Group::new(root, group_param).unwrap());
        }

        // check if any file is actually a group/file
        for file in files {
            let (group, new_path) = self.get_group_from_file(&file);
            new_files.push(new_path); // new_path == path if group is none
            if let Some(group) = group {
                groups.insert(group);
            }
        }

        if groups.len() == 0 {
            return Err("Group missing")?
        }
        else if groups.len() > 1 {
            return Err("Too many groups")?
        }

        let group = groups.iter().take(1).collect::<Vec<_>>()[0].clone();
        
        Ok((new_files, group))
    }

    fn get_group_from_file(&mut self, p: &str) -> (Option<Group>, PathBuf) {
        if let Some(idx) = p.find('/') {
            let dir = &p[0..idx];
            if let Some(group) = self.is_group(dir) {
                let pf = PathBuf::from(&p[(idx+1)..]);
                return (Some(group), pf);
            }
        }
        (None, PathBuf::from(p))
    }

    fn is_group(&mut self, g: &str) -> Option<Group> {
        if g.len() == 0 {
            return None;
        }
        if let Some(group) = self.groups.get(g) {
            return Some(group.clone());
        }
        let p = self.root.join(g);
        if p.is_dir() {
            // not checking if meta.txt presents in dir, thou it seems like a good idea, because on
            // first mv there'll be no such file
            let group = Group::new(self.root.clone(), g).unwrap();
            self.groups.insert(g.to_string(), group.clone());
            return Some(group);
        }
        return None;
    }

}
