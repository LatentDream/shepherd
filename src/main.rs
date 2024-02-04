use std::{path::{Path, PathBuf}, thread::sleep, time};
use watcher::win_watch;

mod watcher;

fn main() {
    
    println!("The sherperd is here! 🐑");
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
    let args: Vec<String> = std::env::args().collect();
    let dir = &args[1];  // ! Can panic!
    // let sub = args[2].parse().unwrap();  // ! Can panic!
    let watch_dog: WatcherDog = WatcherDog {
        dir: PathBuf::from(dir),
        sub: false,
        callback: Box::new(display_change),
    };

    #[cfg(target_family = "windows")]
    {
        win_watch(dir);
        // subscribe_to_change_windows(watch_dog);
    }
    // TODO: Watchdog for Linux: https://www.man7.org/linux/man-pages/man7/inotify.7.html
    // TODO: Fall back watchdog when limited, e.g. polling

}

fn display_change(path: &Path) -> () {
    println!("Wild sheep detected! → {:?}", path)
}

struct WatcherDog {
    dir: PathBuf,
    sub: bool,
    callback: Box<dyn Fn(&Path) -> ()>,
}

fn subscribe_to_change_windows(watch_dog: WatcherDog) -> ! {
     
    // TODO: Watchdog for Windows: https://learn.microsoft.com/en-us/windows/win32/fileio/obtaining-directory-change-notifications
    // Win impl only for now | Todo
    loop {
        println!("Watching the sheep ... in {}", watch_dog.dir.display());
        sleep(time::Duration::from_secs(5));
        
    }

}
