use std::path::PathBuf;
use std::{ptr, u16, u32};
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::sync::mpsc::{channel, Receiver};
use std::thread;
use std::os::windows::ffi::OsStrExt;


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

#[repr(C)]
struct FILE_NOTIFY_INFORMATION {
    next_entry_offset: u32,
    action: u32,
    file_name_length: u32,
    file_name: [u16; 1],
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


pub fn win_watch(dir: &str) {
    // Watch the current directory recursively
    let mut current_dir: Vec<u16> = vec![0; MAX_PATH];

    let input_str = "C:\\Users\\zzzgu\\Documents\\repo\\shepherd\\data";
    let path_buf = PathBuf::from(input_str);
    current_dir = path_buf
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();

    println!("Current working directory: {:?}", String::from_utf16_lossy(&current_dir));
    
    // let directory = CString::new(current_dir_u8).expect("CString conversion failed");
    let directory_handle = unsafe {
        CreateFileW(
            current_dir.as_ptr(),
            FILE_LIST_DIRECTORY,    // FILE_LIST_DIRECTORY
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            ptr::null_mut(),
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS ,
            ptr::null_mut(),
        )
    };

    if directory_handle.is_null() {
        println!("Error opening directory: {}", unsafe { GetLastError() });
        panic!("Failed to open directory");
    }

    // For ReadDirectoryChangesW results
    let mut buffer = Vec::with_capacity(BUFFER_SIZE as usize);

    // To process the directory change notifications
    let (tx, rx): (std::sync::mpsc::Sender<OsString>, Receiver<OsString>) = channel();
    thread::spawn(move || {
        loop {
            process_events(&rx);
        }
    });

    // Main loop to receive directory change notifications
    loop {
        let mut bytes_returned: u32 = 0;
        println!("Listening to changes");
        let result = unsafe {
            ReadDirectoryChangesW(
                directory_handle,
                buffer.as_mut_ptr() as *mut std::ffi::c_void,
                BUFFER_SIZE,
                1, // Recursive
                FILE_NOTIFY_CHANGE_LAST_WRITE | FILE_NOTIFY_CHANGE_CREATION | FILE_NOTIFY_CHANGE_FILE_NAME,
                &mut bytes_returned,
                ptr::null_mut(),
                ptr::null_mut(),
            )
        };

        if result == 0 {
            let error_code = unsafe { GetLastError() };
            panic!("ReadDirectoryChangesW failed {}", error_code);
        }

        process_buffer(&buffer, &tx);
    }

    // Todo: graceful shutdown
    #[warn(unreachable_code)]
    unsafe {
        CloseHandle(directory_handle);
    }
}

fn process_buffer(buffer: &[u8], tx: &std::sync::mpsc::Sender<OsString>) {
    let mut offset: usize = 0;

    while offset < buffer.len() {
        let info = unsafe { &*(buffer.as_ptr().add(offset) as *const FILE_NOTIFY_INFORMATION) };
        let file_name = OsString::from_wide(&info.file_name[0..info.file_name_length as usize]);
        tx.send(file_name).unwrap();

        offset += info.next_entry_offset as usize;

        if info.next_entry_offset == 0 {
            break;
        }
    }
}

fn process_events(rx: &std::sync::mpsc::Receiver<OsString>) {
    while let Ok(event) = rx.recv() {
        println!("File or directory changed: {:?}", event);
    }
}
