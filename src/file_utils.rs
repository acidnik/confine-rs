use std::path::PathBuf;
use std::fs;

use app::Result;

pub struct FileUtils {
    dry: bool,
}

impl FileUtils {
    pub fn new(dry: bool) -> Self {
        FileUtils {dry}
    }

    pub fn log<S: Into<String>>(&self, msg: S) {
        let prefix = if self.dry {
            "dry: "
        }
        else {
            ""
        };
        warn!("{}{}", prefix, msg.into());
    }

    pub fn is_symlink(&self, p: &PathBuf) -> Result<bool> {
        Ok(fs::symlink_metadata(p)?.file_type().is_symlink())
    }

    pub fn unlink(&self, p: &PathBuf) -> Result<()> {
        self.log(format!("rm {}", p.display()));
        if self.dry {
            return Ok(());
        }
        fs_extra::remove_items(&vec![p])?;

        Ok(())
    }

    pub fn mkpath(&self, p: &PathBuf) -> Result<()> {
        self.log(format!("mkdir {}", p.display()));
        if self.dry {
            return Ok(());
        }

        fs::create_dir_all(p)?;
        Ok(())
    }

    pub fn symlink(&self, src: &PathBuf, dst: &PathBuf) -> Result<()> {
        self.log(format!("link {} -> {}", src.display(), dst.display()));
        if self.dry {
            return Ok(());
        }

        std::os::unix::fs::symlink(&src, &dst)?;

        Ok(())
    }
    pub fn copy(&self, src: &PathBuf, dst: &PathBuf) -> Result<()> {
        self.log(format!("copy {} -> {}", src.display(), dst.display()));
        if self.dry {
            return Ok(());
        }

        if ! dst.exists() {
            let dst_parent = dst.parent().unwrap().to_owned();
            self.mkpath(&dst_parent)?;
            self.mkpath(&dst)?;
        }


        fs_extra::copy_items(&vec![src], &dst, &fs_extra::dir::CopyOptions::new())?;

        Ok(())
    }
}
