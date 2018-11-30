extern crate toml;

use std::path::PathBuf;
use std::collections::HashSet;

use app::Result;

pub struct Templates {
    root: PathBuf,
    inited: bool,
    templates: HashSet<PathBuf>,
    files: HashSet<PathBuf>,
}

impl Templates {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root: root,
            inited: false,
            templates: HashSet::new(),
            files: HashSet::new(),
        }
    }

    fn init(&mut self) {
        let tdir = self.root.join("tune/templates");
        self.templates = tdir.read_dir().unwrap().map(|f| PathBuf::from(f.unwrap().path())).collect();
        for template in &self.templates {

        }
    }

    pub fn check(&mut self, file: &PathBuf) -> bool {
        if ! self.inited {
            self.init()
        }
        true
    }
}
