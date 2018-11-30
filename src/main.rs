extern crate clap;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate fs_extra;
extern crate dirs;
extern crate hostname;

use clap::{Arg, App, SubCommand};

use std::error;

mod app;
mod templates;

fn main() -> Result<(), Box<error::Error>> {
    let matches = App::new("confine")
        .version("0.0.1")
        .author("Nikita Bilous <nikita@bilous.me>")
        .about("config file manager")
        .arg(Arg::with_name("quiet")
             .short("q")
             .help("be quiet")
        )
        .arg(Arg::with_name("trace")
             .long("trace")
             .hidden(true)
             .help("show trace info")
        )
        .arg(Arg::with_name("dry")
             .short("n")
             .help("dry run")
        )
        .arg(Arg::with_name("home")
             .long("home")
             .takes_value(true)
             .hidden(true)
             .help("override home dir")
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

    let mut app = app::Confine::new(&matches);
    app.run(&matches)?;
    Ok(())
}
