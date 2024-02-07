use std::ffi::OsString;
use std::path::PathBuf;
use std::fmt;

pub struct WatchDog {
    pub dir: PathBuf,
    pub watch_sub_dir: bool,
    pub callback: Box<dyn Fn(&FileChangeNotification) + Send>,
}

pub enum FileChange {
    Added,
    Modified,
    Removed,
    RenamedNewName,
    RenamedOldName,
    Unknow,
}

pub struct FileChangeNotification {
    pub action: FileChange,
    pub file: OsString,
}

impl fmt::Display for FileChangeNotification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let change = match self.action {
            FileChange::Added => "added",
            FileChange::Modified => "modified",
            FileChange::Removed => "removed",
            FileChange::RenamedNewName => "renamed new name",
            FileChange::RenamedOldName => "renamed old name",
            _ => "unknown",
        };
        write!(
            f,
            "Action: {}, File Name: {}",
            change,
            self.file.to_string_lossy()
        )
    }
}
