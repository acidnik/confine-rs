use snafu::*;
use std::path::PathBuf;

#[derive(Debug, Snafu)]
#[snafu(visibility = "pub")]
pub enum Error {
    // #[snafu(display("Invalid group name {}", group_name))]
    // InvalidGroupName {group_name: String},
    //
    // #[snafu(display("Can not open file {}: {}", path.display(), source))]
    // OpenFile { path: PathBuf, source: std::io::Error },
    //
    // #[snafu(display("Can not write to file {}: {}", path.display(), source))]
    // WriteError { path: PathBuf, source: std::io::Error },
    //
    // #[snafu(display("Subcommand missing, see --help"))]
    // SubcommandMissing { },
    //
    // #[snafu(display("File {} not in meta.txt", file.display()))]
    // NotInMeta { file: PathBuf },
    //
    // #[snafu(display("File {}: template required", file.display()))]
    // TemplateRequred { file: PathBuf },

    #[snafu(display("Template processing error: {}", template_name.display()))] // TODO add tera error as source
    TemplateError { template_name: PathBuf, source: tera::Error },

    #[snafu(display("{}", message))]
    MiscError { message: String },
    
    #[snafu(display("{}: {}", file.display(), message))]
    MiscErrorFile { message: String, file: PathBuf },

    #[snafu(display("IO error {}: {}", path.display(), source))]
    IoError {path: PathBuf, source: std::io::Error},

    #[snafu(display("strip prefix error: {} -- {}:  {}", path.display(), prefix.display(), source))]
    StripPrefixError {path: PathBuf, prefix: PathBuf, source: std::path::StripPrefixError },

    #[snafu(display("{}: {}", path.display(), source))]
    FsError { path: PathBuf, source: fs_extra::error::Error },
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[macro_export]
macro_rules! misc_error {
    ( $x:expr ) => {
        Err(Error::MiscError {message: $x.to_string()})?
    };
}

#[macro_export]
macro_rules! misc_error_file {
    ( $x:expr, $file:expr ) => {
        Err(Error::MiscErrorFile {message: $x.to_string(), file: $file})?
    };
}

