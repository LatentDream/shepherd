mod watcher;

#[cfg(target_family = "windows")]
mod windows;
#[cfg(target_family = "windows")]
pub use windows::watch;


#[cfg(target_family = "unix")]
mod unix;
#[cfg(target_family = "unix")]
pub use unix::watch;


pub use watcher::*;
