use crate::watcher::FileChange;

use std::sync::mpsc::{channel, Receiver};
use super::{WatchDog, FileChangeNotification};
use core::panic;
use std::ffi::{CString, OsString};
use std::io::Error;
use std::os::raw::{c_char, c_int, c_uint, c_void};
use std::os::unix::ffi::OsStringExt;
use std::os::unix::io::RawFd;
use std::process::exit;
use std::slice;
use std::{mem, usize};
use std::thread;

// Main logic
// __________________________________________________________

static mut FILE_DESCRIPTOR: Option<RawFd> = None;

pub fn watch(watch_dog: WatchDog) -> ! {
    // Watch a directory for changes using inotify
    //   -> https://www.man7.org/linux/man-pages/man7/inotify.7.html
    //   -> Or `man inotify`

    let path = watch_dog.dir.as_path().to_str().expect("No directory to watch was provided");
    println!("Watching directory: {}", path);
    let path = CString::new(path).expect("CString::new failed");

    let fd = unsafe { inotify_init() };
    if fd == -1 {
        let error = Error::last_os_error();
        panic!("Failed to create inotify instance: {:?}", error);
    }

    let watch_descriptor = unsafe { inotify_add_watch(fd, path.as_ptr(), INOTIFY_FLAGS_IN_ALL_EVENTS) };
    if watch_descriptor == -1 {
        let error = Error::last_os_error();
        panic!("Failed to create inotify instance: {:?}", error);
    }

    // Graceful exit: Better way of passing the file descriptor ?
    unsafe { FILE_DESCRIPTOR = Some(fd); }
    unsafe { signal(SIGINT, sigint_handler as usize); }

    let (tx, rx): (
        std::sync::mpsc::Sender<FileChangeNotification>,
        Receiver<FileChangeNotification>,
    ) = channel();
    thread::spawn(move || process_events(&rx, watch_dog.callback));


    // Read events
    let mut buffer = [0u8; BUFFER_LEN];
    loop {
        // Operation block until something happen
        let bytes_read = unsafe { read(fd, buffer.as_mut_ptr() as *mut c_void, BUFFER_LEN) };
        if bytes_read == -1 {
            unsafe {
                inotify_add_watch(fd, path.as_ptr(), INOTIFY_FLAGS_IN_ALL_EVENTS);
            }
        }
        process_buffer(&buffer, bytes_read);
    }
}

// C bindings
// __________________________________________________________

const INOTIFY_FLAGS_IN_ALL_EVENTS: u32 = 0xFFF;
const BUFFER_LEN: usize = 1024;
const IN_ACCESS: u32 = 0x00000001;
const IN_MODIFY: u32 = 0x00000002;
const IN_ATTRIB: u32 = 0x00000004;  // Metadata changed
const IN_CLOSE_WRITE: u32 = 0x00000008;
const IN_CLOSE_NOWRITE: u32 = 0x00000010;
const IN_CLOSE: u32 = IN_CLOSE_WRITE | IN_CLOSE_NOWRITE;
const IN_OPEN: u32 = 0x00000020;
const IN_MOVED_FROM: u32 = 0x00000040;
const IN_MOVED_TO: u32 = 0x00000080;
const IN_MOVE: u32 = IN_MOVED_FROM | IN_MOVED_TO;
const IN_CREATE: u32 = 0x00000100;
const IN_DELETE: u32 = 0x00000200;
const IN_DELETE_SELF: u32 = 0x00000400;
const IN_MOVE_SELF: u32 = 0x00000800;


#[repr(C)]
pub struct InotifyEvent {
    pub wd: RawFd,      // Watch descr.
    pub mask: c_uint,   // Descr. event
    pub cookie: c_uint, // Unique cookie for rename event
    pub len: c_uint,    // Name field
    pub name: [u8; 0],  // c_char or c_uchar ?
}

pub type SighandlerT = usize;
pub const SIGINT: c_int = 2;

// FFI
extern "C" {
    fn inotify_init() -> RawFd;
    fn inotify_add_watch(fd: c_int, pathname: *const c_char, mask: c_uint) -> c_int;
    fn read(fd: c_int, buf: *mut c_void, count: usize) -> isize;
    fn close(fd: c_int) -> c_int;
    pub fn signal(signum: c_int, handler: SighandlerT) -> SighandlerT;
}

// Graceful exit
extern "C" fn sigint_handler(_: c_int) {
    println!(" [INFO] Received Ctrl+C signal. Cleaning up...");
    if let Some(fd) = unsafe { FILE_DESCRIPTOR } {
        unsafe {
            close(fd);
        }
    }
    exit(0);
}

// Buffer Processing 
// __________________________________________________________

impl FileChangeNotification {
    unsafe fn from_buffer(buffer: &[u8], buffer_len: isize) -> Vec<FileChangeNotification> {
        let mut notifs = Vec::new();
        let mut current_offset: *const u8 = buffer.as_ptr();
        // Loop over all events in the buffer
        loop {
            let notif_ptr: *const InotifyEvent = current_offset as *const InotifyEvent;
            // Check if the pointer goes beyond the buffer length
            if current_offset.offset(mem::size_of::<InotifyEvent>() as isize) > buffer.as_ptr().offset(buffer_len) {
                break;
            }
            // Process the buffer
            let wd = (*notif_ptr).wd;
            let mask = (*notif_ptr).mask;
            let cookie = (*notif_ptr).cookie;
            let len = (*notif_ptr).len;
            let encoded_path = slice::from_raw_parts((*notif_ptr).name.as_ptr(), len as usize);
            let name_bytes: Vec<u8> = encoded_path.to_owned();
            let path = OsString::from_vec(name_bytes);
            current_offset = current_offset.offset(mem::size_of::<InotifyEvent>() as isize + len as isize);
            if mask == IN_DELETE_SELF {
                panic!("The watched directory has been deleted");
            } else if mask == IN_MOVE_SELF {
                panic!("The watched directory has been moved");
            }
            // Build the notif
            let notif = FileChangeNotification {
                file: path,
                action: match mask {
                    IN_MODIFY => FileChange::Modified,
                    IN_ATTRIB => FileChange::Modified,
                    IN_CLOSE_WRITE => FileChange::Modified,
                    IN_MOVED_FROM => FileChange::RenamedOldName,
                    IN_MOVED_TO => FileChange::RenamedNewName,
                    IN_CREATE => FileChange::Created,
                    IN_DELETE => FileChange::Deleted,
                    _ => FileChange::Other,
                }
            };
            notifs.push(notif);
        }
        return notifs
    }
}

fn process_buffer(buffer: &[u8], buffer_len: isize) {
    unsafe {
        let notifs = FileChangeNotification::from_buffer(buffer, buffer_len);
        for notif in notifs {
            println!("{}", notif);
        }
    }
}
fn process_events(
    rx: &std::sync::mpsc::Receiver<FileChangeNotification>,
    callback: Box<dyn Fn(&FileChangeNotification)>,
) {
    while let Ok(event) = rx.recv() {
        // should process each file after a timeout period has passed with no further updates
        callback(&event);
    }
}

