use io_ring::{IoToken, RingProxy};
use io_uring::{opcode, types};
use std::os::unix::io::RawFd;
use std::path::Path;

pub struct InotifyWatcher {
    fd: RawFd,
    wd: i32,
    ring: RingProxy,
    read_token: Option<IoToken>,
    read_buf: Box<[u8; 4096]>,
}

impl InotifyWatcher {
    // Creates a new InotifyWatcher to monitor changes in the specified directory.
    pub fn new(dir_path: &Path, ring: RingProxy) -> Result<Self, String> {
        let fd = unsafe { libc::inotify_init1(libc::IN_CLOEXEC | libc::IN_NONBLOCK) };
        if fd < 0 {
            return Err("Failed to initialize inotify".to_string());
        }

        let path_str = dir_path.to_str().ok_or("Invalid path")?;
        let c_path = std::ffi::CString::new(path_str).map_err(|_| "Failed to create CString")?;

        let mask = libc::IN_MODIFY | libc::IN_CREATE | libc::IN_MOVED_TO;
        let wd = unsafe { libc::inotify_add_watch(fd, c_path.as_ptr(), mask) };
        if wd < 0 {
            unsafe {
                libc::close(fd);
            }
            return Err("Failed to add inotify watch".to_string());
        }

        let mut watcher = Self {
            fd,
            wd,
            ring,
            read_token: None,
            read_buf: Box::new([0u8; 4096]),
        };

        watcher.submit_read();
        Ok(watcher)
    }

    // Submits an asynchronous read request for inotify events via io_uring.
    pub fn submit_read(&mut self) {
        if self.read_token.is_some() {
            return;
        }

        let fd = self.fd;
        let buf_ptr = self.read_buf.as_mut_ptr();
        let buf_len = self.read_buf.len() as u32;

        // SAFETY: read_buf is heap-allocated (Box) and does not move.
        let sqe = opcode::Read::new(types::Fd(fd), buf_ptr, buf_len).build();
        self.read_token = Some(self.ring.push(sqe));
    }

    // Processes completed read event and returns true if settings.toml was modified.
    pub fn process_event(&mut self, token: IoToken, result: i32) -> bool {
        if Some(token) != self.read_token {
            return false;
        }
        self.read_token = None;

        let mut changed = false;
        let n = result;
        if n > 0 {
            let mut offset = 0;
            while offset < n as usize {
                let event =
                    unsafe { &*(self.read_buf.as_ptr().add(offset) as *const libc::inotify_event) };

                if event.len > 0 {
                    let name_ptr = unsafe {
                        self.read_buf
                            .as_ptr()
                            .add(offset + std::mem::size_of::<libc::inotify_event>())
                    };
                    let name_str =
                        unsafe { std::ffi::CStr::from_ptr(name_ptr as *const libc::c_char) };
                    if name_str
                        .to_str()
                        .map(|s| s == "settings.toml")
                        .unwrap_or(false)
                    {
                        changed = true;
                    }
                }

                offset += std::mem::size_of::<libc::inotify_event>() + event.len as usize;
            }
        }

        // Re-submit the read operation for subsequent events
        self.submit_read();

        changed
    }
}

impl Drop for InotifyWatcher {
    // Cleans up the inotify watch and closes the file descriptor.
    fn drop(&mut self) {
        unsafe {
            libc::inotify_rm_watch(self.fd, self.wd);
            libc::close(self.fd);
        }
    }
}
