use std::{
    env,
    path::{Path, PathBuf},
    process::ExitCode,
};
use watcher::windows;

mod watcher;

struct Command {
    name: &'static str,
    description: &'static str,
    run: fn(&str, env::Args) -> ExitCode,
}

const COMMANDS: &[Command] = &[
    Command {
        name: "help",
        description: "Show this help",
        run: help,
    },
    Command {
        name: "watch",
        description: "Watch a directory. Usage: watch <dir> <bool-watch-sub>",
        run: watch,
    },
];

fn help(program: &str, _args: env::Args) -> ExitCode {
    usage(program);
    ExitCode::SUCCESS
}

fn usage(program: &str) {
    println!("");
    eprintln!("Usage: {program} <command>");
    eprintln!("\nCommands:");
    for cmd in COMMANDS.iter() {
        eprintln!(
            "    {name} - {description}",
            name = cmd.name,
            description = cmd.description
        );
    }
}

struct WatcherDog {
    dir: PathBuf,
    sub: bool,
    callback: Box<dyn Fn(&Path)>,
}

fn watch_extract_params(_args: env::Args) {
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
    // → Insert option in WatchDog to pass it to the watcher
    unimplemented!()
}

fn dummy_fn(path: &Path) {
    println!("{:?}", path);
}

fn watch(_program: &str, args: env::Args) -> ExitCode {
    let mut args = args;
    let dir = args.next().expect("directory or file");
    let sub: bool = match args
        .next()
        .unwrap_or_else(|| "false".to_string())
        .to_lowercase()
        .as_str()
    {
        "true" => true,
        "false" => false,
        "0" => false,
        "1" => true,
        _ => {
            eprintln!("Invalid value for watch sub-tree argument. Expected true or false");
            return ExitCode::FAILURE;
        }
    };

    let _watch_dog: WatcherDog = WatcherDog {
        dir: PathBuf::from(dir.clone()),
        sub,
        callback: Box::new(dummy_fn),
    };

    #[cfg(target_family = "windows")]
    {
        windows::watch(&dir, true);
    }
    #[cfg(target_family = "unix")]
    {
        // https://www.man7.org/linux/man-pages/man7/inotify.7.html
        unimplemented!();
    }
    ExitCode::SUCCESS
}

fn main() -> ExitCode {
    let mut args = env::args();

    let program_name = args.next().expect("🐑");
    if let Some(command_name) = args.next() {
        // Process cmd
        if let Some(command) = COMMANDS.iter().find(|command| command.name == command_name) {
            (command.run)(&program_name, args);
        } else {
            usage(&program_name);
            eprintln!("ERROR: command {command_name} is unknown");
            return ExitCode::FAILURE;
        }
    } else {
        // Show usage
        eprintln!("[ERROR] No command is provided");
        usage(&program_name);
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}
