use std::{
    env,
    path::PathBuf,
    process::ExitCode,
};
use watcher::{ WatchDog, FileChangeNotification};
#[cfg(target_family = "windows")]
use watcher::windows;
#[cfg(target_family = "unix")]
use watcher::unix;

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

fn watch_extract_params(_args: env::Args) {
    // Accept a directory as an argument
    // Options: Watch subdirectorie
    // Options: Ignore hidden files
    // Options: Ignore files [file1, file2, ...]
    // Options: Ignore directories [dir1, dir2, ...]
    // → Insert option in WatchDog to pass it to the watcher
    unimplemented!()
}

fn print_change(change: &FileChangeNotification) {
    println!("{}", change);
}

fn watch(_program: &str, args: env::Args) -> ExitCode {
    let mut args = args;
    let dir = args.next().unwrap_or_else(|| {
        println!("No watch directory provided. Using current directory. To specefy a directory use: watch <dir> <is-watching-sub-tree>");
        ".".to_string()
    });
    let sub: bool = match args
        .next()
        .unwrap_or_else(|| {
            println!("No watch is-watching-sub-tree provided. Using false. To specefy a sub-tree use: watch <dir> <sub-tree>");
            "false".to_string()
        })
        .to_lowercase()
        .as_str()
    {
        "true" => true,
        "false" => false,
        "0" => false,
        "1" => true,
        _ => {
            eprintln!("Invalid value for watch is-watching-sub-tree argument. Expected true or false");
            return ExitCode::FAILURE;
        }
    };

    let watch_dog: WatchDog = WatchDog {
        dir: PathBuf::from(dir.clone()),
        watch_sub_dir: sub,
        callback: Box::new(print_change),
    };

    #[cfg(target_family = "windows")]
    {
        windows::watch(watch_dog);
    }
    #[cfg(target_family = "unix")]
    {
        unix::watch(watch_dog);
    }
    ExitCode::FAILURE
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
