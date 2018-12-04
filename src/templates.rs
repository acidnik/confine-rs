extern crate toml;

use std::fs;

use std::path::PathBuf;
use std::collections::{HashMap};

use app::Result;

/*
files in tune/templates - "control files"
files in groups - "templates"
*/

pub struct Templates {
    root: PathBuf,
    inited: bool,
    control_files: HashMap<PathBuf, Vec<PathBuf>>, // control file => template files
    templates: HashMap<PathBuf, Vec<PathBuf>>, // template => control files
    vars: HashMap<PathBuf, HashMap<PathBuf, toml::value::Table>>, // control file => { template_file => variables }
    home: PathBuf,
}

impl Templates {
    pub fn new(root: PathBuf, home: PathBuf) -> Self {
        Self {
            root: root,
            inited: false,
            control_files: HashMap::new(),
            templates: HashMap::new(),
            vars: HashMap::new(),
            home: home,
        }
    }

    fn init(&mut self) {

        if self.inited {
            return
        }

        self.inited = true;
        
        let tdir = self.root.join("tune/templates");
        trace!("templates init in {:?}", tdir);
        if ! tdir.is_dir() {
            trace!("no template dir {:?}", tdir);
            return;
        }
        let control_files = tdir.read_dir().unwrap().map(|f| PathBuf::from(f.unwrap().path())).collect::<Vec<_>>();
        for cfile in control_files {
            if cfile.extension().map_or(true, |e| e != "toml") {
                continue
            }
            trace!("load template control file {:?}", cfile);
            let t = fs::read_to_string(&cfile).unwrap().parse::<toml::Value>().unwrap();
            let table = t.as_table().unwrap();
            trace!("{:?}", table);
            trace!("{:?} -- {:?}", cfile, table.keys().map(|f| PathBuf::from(f)).collect::<Vec<_>>());
            self.control_files.insert(cfile.clone(), table.keys().map(|f| PathBuf::from(f)).collect());
            for f in table.keys() {
                let p = PathBuf::from(f);
                self.templates.entry(p.clone()).or_insert(Vec::new()).push(cfile.clone());
                // self.vars.insert(p.clone(), table.get(f).unwrap().as_table().unwrap().clone());
                self.vars.entry(cfile.clone()).or_insert(HashMap::new()).insert(p.clone(), table.get(f).unwrap().as_table().unwrap().clone());
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
        let control = if control.ends_with(".toml") {
            control.to_string()
        }
        else {
            control.to_string() + ".toml"
        };
        let control = if let Some(idx) = control.rfind('/') {
            control[(idx+1)..].to_string()
        }
        else {
            control
        };
        let control_file = self.control_files.keys().find(|k| k.file_name().unwrap() == &control[..]);
        if control_file.is_none() {
            return Err(format!("template description not found: {}", control))?
        }
        let control_file = control_file.unwrap().canonicalize()?;
        
        debug!("process template config {} with variables from {}", file.display(), control_file.display());

        trace!("read {:?}", file);
        let file_str = fs::read_to_string(&file)?;
        let mut context = tera::Context::new();
        let vars = self.vars.get(&control_file);
        if vars.is_none() {
            return Err(format!("no variables found for file {} in template descripiton {}", file.display(), control_file.display()))?;
        }
        let vars = vars.unwrap().get(template_name);
        if vars.is_none() {
            return Err(format!("variables for file {} missing in {}", file.display(), control_file.display()))?;
        }
        let mut vars = vars.unwrap().clone();
        if vars.get("HOME").is_none() {
            vars.insert("HOME".to_string(), toml::Value::String(self.home.to_str().unwrap().to_string()));
        }
        for (key, val) in vars.iter() {
            trace!("{} - {:?}", key, val);
            context.insert(key, val.as_str().unwrap());
        }
        let processed = tera::Tera::one_off(&file_str, &context, false)?;
        trace!("{}", processed);

        let tdir = self.root.join("tune/templates/processed/");
        let tdir = tdir.join(template_name.parent().unwrap());
        // TODO dry?
        fs::create_dir_all(&tdir)?;
        let processed_file = tdir.join(template_name.file_name().unwrap());

        trace!("write to {:?}", processed_file);
        fs::write(&processed_file, &processed)?;

        Ok(processed_file)
    }
}
