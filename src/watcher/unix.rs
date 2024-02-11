use super::WatchDog;
use std::ffi::{CString, OsString};
use std::io::Error;
use std::mem;
use std::os::raw::{c_char, c_int, c_uint, c_void};
use std::os::unix::ffi::OsStringExt;
use std::os::unix::io::RawFd;
use std::slice;

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

    let watch_descriptor =
        unsafe { inotify_add_watch(fd, path.as_ptr(), INOTIFY_FLAGS_IN_ALL_EVENTS) };

    if watch_descriptor == -1 {
        let error = Error::last_os_error();
        panic!("Failed to create inotify instance: {:?}", error);
    }

    let mut buffer = [0u8; BUFFER_LEN];

    // Read events
    loop {
        let bytes_read = unsafe { read(fd, buffer.as_mut_ptr() as *mut c_void, BUFFER_LEN) };
        if bytes_read == -1 {
            // Operation block until something happen
            // Potential problem resending the same path ?
            unsafe {
                inotify_add_watch(fd, path.as_ptr(), INOTIFY_FLAGS_IN_ALL_EVENTS);
            }
        }
        process_buffer(&buffer, bytes_read);
    }
}

const INOTIFY_FLAGS_IN_ALL_EVENTS: u32 = 0xFFF;
const BUFFER_LEN: usize = 1024;

#[repr(C)]
pub struct InotifyEvent {
    pub wd: RawFd,      // Watch descr.
    pub mask: c_uint,   // Descr. event
    pub cookie: c_uint, // Unique cookie for rename event
    pub len: c_uint,    // Name field
    pub name: [u8; 0],  // c_char or c_uchar ?
}

// FFI
extern "C" {
    fn inotify_init() -> c_int;
    fn inotify_add_watch(fd: c_int, pathname: *const c_char, mask: c_uint) -> c_int;
    fn read(fd: c_int, buf: *mut c_void, count: usize) -> isize;
}

fn process_buffer(buffer: &[u8], buffer_len: isize) {
    unsafe {
        let mut current_offset: *const u8 = buffer.as_ptr();
        // Loop over all events in the buffer
        loop {
            let notif_ptr: *const InotifyEvent = current_offset as *const InotifyEvent;
            // Check if the pointer goes beyond the buffer length
            if current_offset.offset(mem::size_of::<InotifyEvent>() as isize) > buffer.as_ptr().offset(buffer_len) {
                break;
            }

            let wd = (*notif_ptr).wd;
            let mask = (*notif_ptr).mask;
            let cookie = (*notif_ptr).cookie;
            let len = (*notif_ptr).len;
            let encoded_path = slice::from_raw_parts((*notif_ptr).name.as_ptr(), len as usize);
            let name_bytes: Vec<u8> = encoded_path.to_owned();
            let path = OsString::from_vec(name_bytes);
            println!("{} → {}", path.to_string_lossy(), mask);
            current_offset = current_offset.offset(mem::size_of::<InotifyEvent>() as isize + len as isize);
        }
    }
}
