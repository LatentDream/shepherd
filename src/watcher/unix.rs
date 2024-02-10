use super::WatchDog;
use std::ffi::CString;
use std::io::Error;
use std::os::raw::{c_char, c_int, c_uint, c_void};
use std::os::unix::io::RawFd;

pub fn watch(watch_dog: WatchDog) -> ! {
    // Watch a directory for changes using inotify
    //   -> https://www.man7.org/linux/man-pages/man7/inotify.7.html
    //   -> Or `man inotify`

    let path = CString::new("./data").expect("CString::new failed");

    let fd = unsafe { inotify_init() };
    if fd == -1 {
        let error = Error::last_os_error();
        panic!("Failed to create inotify instance: {:?}", error);
    }

    let watch_descriptor = unsafe {
        inotify_add_watch(fd, path.as_ptr(), INOTIFY_FLAGS_IN_ALL_EVENTS)
    };

    if watch_descriptor == -1 {
        let error = Error::last_os_error();
        panic!("Failed to create inotify instance: {:?}", error);
    }

    let mut buffer = [0u8; BUFFER_LEN];

    // Read events
    loop {
        let bytes_read = unsafe { read(fd, buffer.as_mut_ptr() as *mut c_void, BUFFER_LEN) };
        if bytes_read == -1 {
            // Potential problem resending the same path ?
            unsafe { inotify_add_watch(fd, path.as_ptr(), INOTIFY_FLAGS_IN_ALL_EVENTS); }
        }
        // TODO: Process events
        println!("Something happened!");

    }
}

const INOTIFY_FLAGS_IN_ALL_EVENTS: u32 = 0xFFF;
const BUFFER_LEN: usize = 1024;

#[repr(C)]
pub struct InotifyEvent {
    pub wd: RawFd,
    pub mask: c_uint,
    pub cookie: c_uint,
    pub len: c_uint,
    pub name: [u8; 0], // c_char or c_uchar ?
}

// FFI
extern "C" {
    fn inotify_init() -> c_int;
    fn inotify_add_watch(fd: c_int, pathname: *const c_char, mask: c_uint) -> c_int;
    fn read(fd: c_int, buf: *mut c_void, count: usize) -> isize;
}

