use std::fmt;
use std::path::PathBuf;
use std::{ptr, u16, u32};
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::sync::mpsc::{channel, Receiver};
use std::thread;
use std::os::windows::ffi::OsStrExt;
use std::slice;
use std::mem;

// Great blog post on the subject: https://qualapps.blogspot.com/2010/05/understanding-readdirectorychangesw_19.html

const BUFFER_SIZE: u32 = 4096;
const MAX_PATH: usize = 260; // Max path length in Windows
const FILE_SHARE_READ: u32 = 0x00000001;
const FILE_SHARE_WRITE: u32 = 0x00000002;
const FILE_SHARE_DELETE: u32 = 0x00000004;
const OPEN_EXISTING: u32 = 3;
const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x02000000;
const FILE_LIST_DIRECTORY: u32  = 0x0001;
const FILE_NOTIFY_CHANGE_LAST_WRITE: u32 = 0x00000010;
const FILE_NOTIFY_CHANGE_CREATION: u32  = 0x00000040;
const FILE_NOTIFY_CHANGE_FILE_NAME: u32 = 0x00000001;
type FILE_ACTION = u32;
const FILE_ACTION_ADDED: FILE_ACTION = 1u32;
const FILE_ACTION_MODIFIED: FILE_ACTION = 3u32;
const FILE_ACTION_REMOVED: FILE_ACTION = 2u32;
const FILE_ACTION_RENAMED_NEW_NAME: FILE_ACTION = 5u32;
const FILE_ACTION_RENAMED_OLD_NAME: FILE_ACTION = 4u32;


#[repr(C)]
pub struct FILE_NOTIFY_INFORMATION {
    pub next_entry_offset: u32,
    pub action: FILE_ACTION,
    pub file_name_length: u32,
    pub file_name: [u16; 1],
}


extern "system" {
    fn ReadDirectoryChangesW(
        directory: *mut std::ffi::c_void,
        buffer: *mut std::ffi::c_void,
        buffer_size: u32,
        recursive: i32,
        filter: u32,
        bytes_returned: *mut u32,
        overlapped: *mut std::ffi::c_void,
        completion_routine: *mut std::ffi::c_void,
    ) -> i32;

    fn CreateFileW(
        file_name: *const u16,
        desired_access: u32,
        share_mode: u32,
        security_attributes: *mut std::ffi::c_void,
        creation_disposition: u32,
        flags_and_attributes: u32,
        template_file: *mut std::ffi::c_void,
    ) -> *mut std::ffi::c_void;

    pub fn GetLastError() -> u32;

    fn CloseHandle(handle: *mut std::ffi::c_void) -> i32;
}


#[link(name = "kernel32")]
extern "system" {
    fn GetModuleFileNameW(hModule: *mut std::ffi::c_void, lpFilename: *mut u16, nSize: u32) -> u32;
}


pub fn watch(dir: &str, with_sub_tree: bool) {

    let mut current_dir: Vec<u16> = vec![0; MAX_PATH];

    let path_buf = PathBuf::from(dir);
    current_dir = path_buf
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();

    println!("Current working directory: {:?}", String::from_utf16_lossy(&current_dir));
    
    let directory_handle = unsafe {
        // Warning: the handle change the dir state to "in use" so it can't be deleted
        CreateFileW(
            current_dir.as_ptr(),
            FILE_LIST_DIRECTORY,    // FILE_LIST_DIRECTORY
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            ptr::null_mut(),
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS,
            ptr::null_mut(),
        )
    };

    if directory_handle.is_null() {
        println!("Error opening directory: {}", unsafe { GetLastError() });
        panic!("Failed to open directory");
    }


    let (tx, rx): (std::sync::mpsc::Sender<FileNotifyInfo>, Receiver<FileNotifyInfo>) = channel();
    thread::spawn(move || {
        loop {
            process_events(&rx);
        }
    });

    // Main loop to receive directory change notifications
    let mut buffer: Vec<u8> = vec![0; BUFFER_SIZE as usize];  // Data buffer → If overflow, all notif are lost
    let watch_subtree = if with_sub_tree {1} else {0};
    loop {
        let mut bytes_returned: u32 = 0;
        let result = unsafe {
            ReadDirectoryChangesW(
                directory_handle,
                buffer.as_mut_ptr() as *mut std::ffi::c_void,
                BUFFER_SIZE,
                watch_subtree ,
                FILE_NOTIFY_CHANGE_LAST_WRITE 
                    | FILE_NOTIFY_CHANGE_CREATION 
                    | FILE_NOTIFY_CHANGE_FILE_NAME,
                &mut bytes_returned,
                ptr::null_mut(),
                ptr::null_mut(),
            )
        };


        if result == 0 {
            let error_code = unsafe { GetLastError() };
            panic!("ReadDirectoryChangesW failed {}", error_code);
        }

        println!("A sheep is on the run {}", result);
        println!("  | Bytes returned → {}", bytes_returned);

        // Convert the byte slice to a string
        process_buffer(&buffer.clone(), bytes_returned, &tx);

    }

    // Todo: graceful shutdown
    #[warn(unreachable_code)]
    unsafe {
        CloseHandle(directory_handle);
    }
}


pub struct FileNotifyInfo {
    pub action: FILE_ACTION,
    pub file_name: OsString,
}


impl fmt::Display for FileNotifyInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let change = match self.action {
            FILE_ACTION_ADDED => "added",
            FILE_ACTION_MODIFIED => "modified",
            FILE_ACTION_REMOVED => "removed",
            FILE_ACTION_RENAMED_NEW_NAME => "renamed new name",
            FILE_ACTION_RENAMED_OLD_NAME => "renamed old name",
            _ => "unknown",
        };

        write!(f, "Action: {}, File Name: {}", change, self.file_name.to_string_lossy())
    }
}

impl FileNotifyInfo {
    
    unsafe fn from_buffer(buffer: &[u8]) -> Vec<FileNotifyInfo> {

        let mut notifs = Vec::new();
        
        let mut current_offset: *const u8 = buffer.as_ptr();
        let mut notif_ptr: *const FILE_NOTIFY_INFORMATION = mem::transmute(current_offset);
        loop {
            // filename length is size in bytes, so / 2
            let len = (*notif_ptr).file_name_length as usize / 2;
            let encoded_path: &[u16] = slice::from_raw_parts((*notif_ptr).file_name.as_ptr(), len);
            // Todo? prepend root to get a full path
            let path = OsString::from_wide(encoded_path);
            notifs.push(FileNotifyInfo {
                action: (*notif_ptr).action,
                file_name: path,
            });
            if (*notif_ptr).next_entry_offset == 0 {
                break;
            }
            current_offset = current_offset.offset((*notif_ptr).next_entry_offset as isize);
            notif_ptr = mem::transmute(current_offset);
        }

        return notifs;
    }

}


fn process_buffer(buffer: &[u8], bytes_returned: u32, tx: &std::sync::mpsc::Sender<FileNotifyInfo>) {
    if bytes_returned == 0 {
        return;
    }

    let notifs = unsafe { FileNotifyInfo::from_buffer(buffer) };
    for notif in notifs {
        let _ = tx.send(notif);
    }

}


fn process_events(rx: &std::sync::mpsc::Receiver<FileNotifyInfo>) {
    while let Ok(event) = rx.recv() {
        // should process each file after a timeout period has passed with no further updates
        println!("{}", event);
    }
}
