use clap::{ArgMatches};

use std::path::PathBuf;
use std::collections::{HashSet, HashMap};

use std::fmt;
use std::io::{self, BufRead};
use std::fs;

use templates::Templates;
use file_utils::FileUtils;

use snafu::*;
use errors::*;

struct Meta {
    dry: bool,
    meta_file: PathBuf,
    entries: Vec<String>,
}

impl Meta {
    fn new(group: &Group) -> Result<Self> {
        let meta_file = group.abs_path().join("meta.txt");
        let entries = if meta_file.exists() {
            io::BufReader::new(fs::File::open(PathBuf::from(&meta_file)).context(IoError {path: meta_file.clone()})?)
                .lines()
                .map(|l| l.unwrap())
                .collect::<Vec<_>>()
        }
        else {
            Vec::new()
        };
        let dry = group.dry;
        Ok(Self {dry, meta_file, entries})
    }
    fn add(&mut self, entry: &PathBuf) -> Result<()> {
        let meta_file = &self.meta_file;
        trace!("{:?} add {:?}", meta_file, entry);
        let entry_str = entry.to_str().unwrap().to_string();
        if ! meta_file.exists() {
            let entry_str = entry.to_str().unwrap().to_string();
            fs::write(&meta_file, entry_str + "\n").context(IoError { path: meta_file })?;
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
        self.entries = lines;
        self.save()?;

        Ok(())
    }
    fn delete(&mut self, entry: &PathBuf) -> Result<()> {
        let entries = self.entries.clone().into_iter().filter(|s| PathBuf::from(s) != *entry).collect::<Vec<_>>();
        self.entries = entries;
        self.save()?;

        Ok(())
    }
    fn save(&self) -> Result<()> {
        trace!("new meta: {:?}", self.entries);
        if self.dry {
            return Ok(())
        }
        fs::write(&self.meta_file, self.entries.join("\n") + "\n").context(IoError {path: &self.meta_file})?;

        Ok(())
    }
    fn list(&self) -> Result<Vec<String>> {
        Ok(self.entries.clone())
    }
    fn check(&self, entry: &PathBuf) -> bool {
        // is entry in meta.txt?
        self.entries.iter().any(|ref e| &PathBuf::from(e) == entry)
    }
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
struct Group {
    dry: bool,
    root: PathBuf,
    dir: PathBuf,
}

impl Group {
    fn new(dry: bool, root: PathBuf, path: &str) -> Result<Self> {
        if path.find('/').is_some() || path == "backup" {
            misc_error_file!("Invalid group name", PathBuf::from(path))
        }
        Ok(Group { dry, dir: PathBuf::from(path), root, })
    }
    fn add_meta(&self, entry: &PathBuf) -> Result<()> {
        let mut meta = Meta::new(&self)?;
        meta.add(entry)
    }
    fn abs_path(&self) -> PathBuf {
        self.root.join(self.dir.clone())
    }
}

impl fmt::Display for Group {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "{}", self.dir.display())
    }
}


pub struct Confine {
    dry: bool,
    home: PathBuf,
    root: PathBuf,
    templates: Templates,
    groups: HashMap<String, Group>,
    template: Option<String>,
    del_link_only: bool,
    fs: FileUtils,
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
            templates: Templates::new(root.clone(), home.clone()),
            home: home,
            root: root,
            groups: HashMap::new(),
            template: None,
            del_link_only: false,
            fs: FileUtils::new(dry),
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
           self.template = matches.value_of("template").map(|s| s.to_string());
           self.link_files(group, files)
        }
        else if let Some(matches) = matches.subcommand_matches("move") {
           let (files, group) = self.get_files_from_args(&matches)?;
           self.move_files(group, files)
        }
        else if let Some(matches) = matches.subcommand_matches("undo") {
            let (files, group) = self.get_files_from_args(&matches)?;
            self.undo_files(group, files)
        }
        else if let Some(matches) = matches.subcommand_matches("delete") {
            let (files, group) = self.get_files_from_args(&matches)?;
            self.del_link_only = matches.is_present("link");
            self.delete_files(group, files)
        }
        else {
            return misc_error!("Subcommand missing")
        }
    }

    fn link_files(&mut self, group: Group, files: Vec<PathBuf>) -> Result<()> {
        let meta = Meta::new(&group)?;
        let files = if ! files.is_empty() {
            files
        }
        else {
            meta.list()?.into_iter().map(PathBuf::from).collect()
        };
        for file in files {
            debug!("link [{}] {}", group, file.display());
            if ! meta.check(&file) {
                misc_error_file!("File not in meta.txt", file.clone())
            }
            self.link_file(&group, &file)?;
        }
        Ok(())
    }
    fn link_file(&mut self, group: &Group, file: &PathBuf) -> Result<()> {
        let template_name = group.dir.join(&file);
        
        let src = if self.templates.needs_template(&template_name) {
            if self.template.is_none() {
                // self.template is arg to -t <template>
                misc_error_file!("Template required for file", file.to_path_buf())
            }
            else {
                self.templates.process(&template_name, &group.abs_path().join(file), &self.template.clone().unwrap())?
            }
        }
        else {
            group.abs_path().join(file)
        };
        
        let dest = if file.is_relative() {
            self.home.join(file)
        }
        else {
            return misc_error!("absolute paths are not supported yet")
        };
        if dest.exists() {
            let destd = dest.display();
            warn!("link: destination file {} exists", destd);
            if self.fs.is_symlink(&dest)? {
                let dest_canon = dest.canonicalize().context(IoError { path: dest.clone() })?;
                if dest_canon == src {
                    warn!("{} is already a link to {}", destd, src.display());
                    return Ok(());
                }
                warn!("link: destination file {} is symlink, removing", destd);
                self.fs.unlink(&dest)?;
            }
            else {
                warn!("creating backup for {} before overwriting", destd);
                self.backup_file(&group, &dest)?;
                self.fs.unlink(&dest)?;
            }
        }
        let link_dir = dest.parent().unwrap();
        if ! link_dir.exists() {
            self.fs.mkpath(&link_dir.to_owned())?
        }

        if ! src.exists() {
            error!("file {} not found! Please remove it with confine delete", src.display());
            misc_error_file!("Source file not found", src.clone())
        }

        self.fs.symlink(&src, &dest)?;

        Ok(())
    }

    fn backup_file(&self, group: &Group, path: &PathBuf) -> Result<()> {
        let path = path.canonicalize().context(IoError {path: path})?;
        let rel_path = path.strip_prefix(self.home.clone()).context(StripPrefixError {path: path.clone(), prefix: self.home.clone() })?.to_owned();
        let hostname = hostname::get_hostname().unwrap();
        let backup_dest = group.root.join("backup").join(&hostname).join(&rel_path).parent().unwrap().to_owned();
        
        self.fs.mkpath(&backup_dest)?;
        
        trace!("backup {:?} to {:?}", path, backup_dest);

        self.fs.copy(&path, &backup_dest)?;

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
        let real_file = file.canonicalize().context(IoError {path: file.clone()})?;
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
            misc_error!(format!("Can not move file {} to {} ({}): file exists", file.display(), group, dest.display()))
        }

        self.do_move_file(&file, &dest)?;
        if ! self.dry {
            group.add_meta(&rel_path)?;
        }

        Ok(())
    }

    fn do_move_file(&self, from: &PathBuf, to: &PathBuf) -> Result<()> {
        let dest_dir = to.parent().unwrap().to_owned();
        trace!("dest dir = {:?}", dest_dir);
        self.fs.mkpath(&dest_dir)?;

        self.fs.copy(&from, &dest_dir)?;
        self.fs.unlink(&from)?;
        self.fs.symlink(&to, &from)?;
        
        Ok(())
    }

    fn undo_files(&self, group: Group, files: Vec<PathBuf>) -> Result<()> {
        let meta = Meta::new(&group)?;
        let files = if ! files.is_empty() {
            files
        }
        else {
            meta.list()?.into_iter().map(PathBuf::from).collect()
        };
        for file in files {
            debug!("undo link [{}] {}", group, file.display());
            if ! meta.check(&file) {
                misc_error_file!("file not in meta.txt", file.clone())
            }
            self.undo_link_file(&group, &file)?;
        }
        Ok(())
    }
    
    fn undo_link_file(&self, _group: &Group, file: &PathBuf) -> Result<()> {
        // before: ~/.foo.rc -> ~/config/grp/.foo.rc
        // after: ~/.foo.rc (copied from ~/config/grp/.foo.rc)

        let link_file = if file.is_relative() {
            self.home.join(file)
        }
        else {
            return misc_error!("absolute paths are not supported yet")
        };
        if ! link_file.exists() {
            // TODO just warning?
            return misc_error_file!("file does not exists, can not undo", link_file)
        }
        if ! self.fs.is_symlink(&link_file)? {
            warn!("{} is not a symlink, nothing to undo", link_file.display());
            return Ok(());
        }

        let real_file = link_file.canonicalize().context(IoError {path: link_file.clone()})?;

        self.fs.unlink(&link_file)?;
        self.fs.copy(&real_file, &link_file)?;
        
        Ok(())
    }
    
    fn delete_files(&self, group: Group, files: Vec<PathBuf>) -> Result<()> {
        if files.is_empty() {
            warn!("No files specified. Not deleting whole group. Please do `confine undo` and remove whole group directory by hand if you don't need it anymore");
            return Ok(());
        }
        for file in files {
            debug!("delete {}", file.display());
            self.delete_file(&group, &file)?;
        }
        Ok(())
    }
    
    fn delete_file(&self, group: &Group, file: &PathBuf) -> Result<()> {
        // 1. delete link
        // 2. delete file
        //   2.1 delete processed template (TODO)
        // 3. delete from meta
        let mut meta = Meta::new(&group)?;
        if ! meta.check(&file) {
            return misc_error_file!("file is not in meta.txt", file.to_path_buf())
        }
        let link_file = if file.is_relative() {
            self.home.join(file)
        }
        else {
            return misc_error!("absolute paths are not supported yet")
        };

        if link_file.exists() {
            if ! self.fs.is_symlink(&link_file)? {
                warn!("file {} is not a symlink, not deleting", link_file.display());
            }
            else {
                self.fs.unlink(&link_file)?
            }
        }
        
        if self.del_link_only {
            return Ok(())
        }
        
        let src = group.abs_path().join(file);
        if ! src.exists() {
            debug!("{} already deleted, ok", src.display());
        }
        else {
            self.fs.unlink(&src)?
        }

        if meta.check(&file) {
            meta.delete(&file)?
        }

        Ok(())
    }

    fn get_rel_path(&self, file: &PathBuf) -> Result<(String, PathBuf)> {
        // returns meta entry and relative path (relative to home or root dir)
        let home = &self.home;
        if let Ok(rel) = file.strip_prefix(home) {
            return Ok((rel.display().to_string(), rel.to_path_buf()))
        }
        match file.strip_prefix(PathBuf::from("/")) {
            Ok(rel) => Ok((file.display().to_string(), rel.to_path_buf())),
            Err(e) => misc_error!(format!("{}", e)),
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
            if let Some(group_file) = group_file {
                new_files.push(group_file);
            }
        }
        else {
            let root = self.root.clone();
            groups.insert(Group::new(self.dry, root, group_param).unwrap());
        }

        // check if any file is actually a group/file
        for file in files {
            let (group, new_path) = self.get_group_from_file(&file);
            if let Some(new_path) = new_path {
                new_files.push(new_path); // new_path == path if group is none
            }
            if let Some(group) = group {
                groups.insert(group);
            }
        }

        if groups.len() == 0 {
            return misc_error!("Group missing")
        }
        else if groups.len() > 1 {
            return misc_error!("Too many groups")
        }

        let group = groups.iter().take(1).collect::<Vec<_>>()[0].clone();
        
        Ok((new_files, group))
    }

    fn get_group_from_file(&mut self, p: &str) -> (Option<Group>, Option<PathBuf>) {
        if let Some(idx) = p.find('/') {
            let dir = &p[0..idx];
            if let Some(group) = self.find_group(dir) {
                let pf = if idx < p.len()-1 {
                    Some(PathBuf::from(&p[(idx+1)..]))
                }
                else {
                    None
                };
                return (Some(group), pf);
            }
        }
        (None, Some(PathBuf::from(p)))
    }

    fn find_group(&mut self, g: &str) -> Option<Group> {
        if g.is_empty() {
            return None;
        }
        if let Some(group) = self.groups.get(g) {
            return Some(group.clone());
        }
        let p = self.root.join(g);
        if p.is_dir() {
            // not checking if meta.txt presents in dir, thou it seems like a good idea, because on
            // first mv there'll be no such file
            let group = Group::new(self.dry, self.root.clone(), g).unwrap();
            self.groups.insert(g.to_string(), group.clone());
            return Some(group);
        }
        None
    }

}
