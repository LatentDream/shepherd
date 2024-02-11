use std::ffi::OsString;
use std::path::PathBuf;
use std::fmt;

pub struct WatchDog {
    pub dir: PathBuf,
    pub watch_sub_dir: bool,
    pub callback: Box<dyn Fn(&FileChangeNotification) + Send>,
}

pub enum FileChange {
    Created,
    Modified,
    Deleted,
    RenamedNewName,
    RenamedOldName,
    Unknow,
    Other,
}

pub struct FileChangeNotification {
    pub action: FileChange,
    pub file: OsString,
}

impl fmt::Display for FileChangeNotification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let change = match self.action {
            FileChange::Created => "Created",
            FileChange::Modified => "modified",
            FileChange::Deleted => "deleted",
            FileChange::RenamedNewName => "renamed new name",
            FileChange::RenamedOldName => "renamed old name",
            FileChange::Other => "other",
            FileChange::Unknow => "unknown",
        };
        write!(
            f,
            "Action: {}, File Name: {}",
            change,
            self.file.to_string_lossy()
        )
    }
}
