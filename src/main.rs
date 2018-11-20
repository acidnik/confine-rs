extern crate clap;

use clap::{Arg, ArgMatches, App, SubCommand};
use std::path::PathBuf;

struct Group {
    dir: PathBuf,
}

trait Action {
    fn run(&self, group: &Group, files: Vec<PathBuf>) -> Result<(), String>;
}

struct ActionMove {}
struct ActionLink {}

impl Action for ActionMove {
    fn run(&self, group: &Group, files: Vec<PathBuf>) -> Result<(), String> {
        for file in files {
            if let Err(err) = self.move_file(&group, &file) {
                return Err(err)
            }
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


fn main() {
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
    println!("{:?}", matches);
}
