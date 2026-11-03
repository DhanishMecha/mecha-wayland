//! A `CLOCK_REALTIME` timerfd armed with `TFD_TIMER_CANCEL_ON_SET`: it
//! never expires, and turns readable whenever the wall clock is set
//! discontinuously (`clock_settime`, an NTP step, resume from suspend).
//! Poll it on the ring like any other fd.
#![allow(unsafe_code)]

use std::io;
use std::mem;
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, OwnedFd};

/// Readable after the wall clock is set; [`ClockSetFd::reset`] clears it.
#[derive(Debug)]
pub struct ClockSetFd {
    fd: OwnedFd,
}

impl ClockSetFd {
    pub fn new() -> io::Result<Self> {
        // SAFETY: plain syscall; a non-negative result is a new fd we own.
        let raw = unsafe {
            libc::timerfd_create(libc::CLOCK_REALTIME, libc::TFD_NONBLOCK | libc::TFD_CLOEXEC)
        };
        if raw < 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: `raw` was just returned by `timerfd_create` and is owned
        // by nothing else.
        let me = ClockSetFd {
            fd: unsafe { OwnedFd::from_raw_fd(raw) },
        };
        me.arm()?;
        Ok(me)
    }

    /// Whether the wall clock was set since the last reset. Re-arms the
    /// timer when it was, so the next set is seen too.
    pub fn reset(&self) -> io::Result<bool> {
        let mut ticks = 0u64;
        // SAFETY: reads at most 8 bytes into a live `u64`.
        let n = unsafe {
            libc::read(
                self.fd.as_raw_fd(),
                (&raw mut ticks).cast(),
                mem::size_of::<u64>(),
            )
        };
        if n >= 0 {
            return Ok(false);
        }
        let err = io::Error::last_os_error();
        match err.raw_os_error() {
            Some(libc::ECANCELED) => self.arm().map(|()| true),
            Some(libc::EAGAIN) => Ok(false),
            _ => Err(err),
        }
    }

    /// An absolute deadline at the end of time, cancelled on clock set.
    fn arm(&self) -> io::Result<()> {
        // SAFETY: itimerspec is a plain C struct; all-zero is valid.
        let mut spec: libc::itimerspec = unsafe { mem::zeroed() };
        spec.it_value.tv_sec = libc::time_t::MAX;
        // SAFETY: `spec` is a valid itimerspec; the old value is not wanted.
        let r = unsafe {
            libc::timerfd_settime(
                self.fd.as_raw_fd(),
                libc::TFD_TIMER_ABSTIME | libc::TFD_TIMER_CANCEL_ON_SET,
                &spec,
                std::ptr::null_mut(),
            )
        };
        if r < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
}

impl AsFd for ClockSetFd {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.fd.as_fd()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_fd_reports_no_set() {
        let fd = ClockSetFd::new().unwrap();
        assert!(!fd.reset().unwrap());
    }
}
