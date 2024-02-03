use std::{path::Path, thread::sleep, time};

fn main() {
    println!("The stalker is finding the perfect bushes to hide in...");

    // Accept a directory as an argument
    // - Args: -d, --dir
    // - Default: current directory
    // Options: Watch subdirectorie
    // - Args: -s, --sub
    // - Default: false
    // Options: Ignore hidden files
    // - Args: -i, --ignore
    // - Default: false
    // Options: Ignore files
    // - Args: -f, --file
    // - Default: false
    // Options: Ignore directories
    // - Args: -D, --dir
    // - Default: false

    // TODO: cmd args parser
    
    // TODO: Watchdog for Windows: https://learn.microsoft.com/en-us/windows/win32/fileio/obtaining-directory-change-notifications
    // TODO: Watchdog for Linux: https://www.man7.org/linux/man-pages/man7/inotify.7.html
    // TODO: Fall back watchdog when limited, e.g. polling
}

pub struct WatcherDog<'a> {
    pub dir: &'a Path,
    pub sub: bool,
    pub callback: Box<dyn Fn(Path) -> ()>,
}

fn watch(watch_dog: WatcherDog) -> ! {
    
    // Win impl only for now | Todo
    loop {
        println!("Watching the sheep...");
        sleep(time::Duration::from_secs(5));
    }

}
