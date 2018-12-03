extern crate toml;

use std::fs;

use std::path::PathBuf;
use std::collections::{HashSet, HashMap};

use app::Result;

/*

   for file in link_files {
        1. check that control required
        2. check that control has entry for file
        // templates.process(file, template)?
        if templates.is_template_file(file) {
            if ! template {
            ...
            }
            file = templates.process(template, file)
        }
   }
   

*/

/*
files in tune/templates - "control files"
files in groups - "templates"
*/

pub struct Templates {
    root: PathBuf,
    inited: bool,
    control_files: HashMap<PathBuf, Vec<PathBuf>>, // control file => template files
    templates: HashMap<PathBuf, Vec<PathBuf>>, // template => control files
}

impl Templates {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root: root,
            inited: false,
            control_files: HashMap::new(),
            templates: HashMap::new(),
        }
    }

    fn init(&mut self) {

        if self.inited {
            return
        }

        self.inited = true;
        
        let tdir = self.root.join("tune/templates");
        if ! tdir.is_dir() {
            return;
        }
        let control_files = tdir.read_dir().unwrap().map(|f| PathBuf::from(f.unwrap().path())).collect::<Vec<_>>();
        for cfile in control_files {
            trace!("load template control file {:?}", cfile);
            let t = fs::read_to_string(&cfile).unwrap().parse::<toml::Value>().unwrap();
            let table = t.as_table().unwrap();
            trace!("{:?}", table);
            trace!("{:?} -- {:?}", cfile, table.keys().map(|f| PathBuf::from(f)).collect::<Vec<_>>());
            self.control_files.insert(cfile.clone(), table.keys().map(|f| PathBuf::from(f)).collect());
            // trace!("control {:?} -- templates {:?}", template, table.keys());
            for f in table.keys() {
                // TODO validation: f must contain '/' (and before / must be a group name) or f = 'include'
                self.templates.entry(PathBuf::from(f)).or_insert(Vec::new()).push(cfile.clone());
                // self.templates.insert(PathBuf::from(f));
            }
        }
    }

    pub fn needs_template(&mut self, file: &PathBuf) -> bool {
        self.init();
        if let Some(cfiles) = self.templates.get(file) {
            debug!("{} is a template file: required by {:?}", file.display(), cfiles);
            return true;
        }
        return false;
    }

    pub fn process(&mut self, template_name: &PathBuf, file: &PathBuf, control: &str) -> Result<PathBuf> {
        let contorl = if control.ends_with(".toml") {
            // control[..-5]
            ()
        }
        else {
            // control[..]
            ()
        };
        // let control_file = self.control_files.keys().find(|k| );

        Ok(PathBuf::new())
    }
}
