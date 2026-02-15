//! Force `SO_REUSEPORT` on sockets owned by other processes.
//!
//! Scans `/proc/net/{tcp,udp}{,6}` to find sockets bound to a given port,
//! then uses `pidfd_open(2)` + `pidfd_getfd(2)` to duplicate each socket
//! into our process and set `SO_REUSEPORT` on it.
//!
//! Requires `CAP_SYS_PTRACE` (or root) and Linux 5.6+.

use std::fs;
use std::io;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};

const PROC_SOURCES: [(&str, bool); 4] = [
    ("/proc/net/tcp", true),
    ("/proc/net/tcp6", true),
    ("/proc/net/udp", false),
    ("/proc/net/udp6", false),
];

/// TCP_LISTEN state in `/proc/net/tcp`.
const TCP_LISTEN: u32 = 0x0A;

/// Force `SO_REUSEPORT` on all existing sockets bound to `port`.
pub(crate) fn force_reuse_port(port: u16) -> io::Result<()> {
    for &(path, is_tcp) in &PROC_SOURCES {
        for inode in find_inodes(path, port, is_tcp)? {
            if let Some((pid, fd)) = find_pid_fd(inode)? {
                set_reuse_port(pid, fd)?;
            }
        }
    }
    Ok(())
}

/// Parse `/proc/net/{tcp,udp}{,6}` for all sockets matching `port`.
///
/// For TCP only LISTEN sockets are matched; for UDP any bound socket qualifies.
fn find_inodes(path: &str, port: u16, is_tcp: bool) -> io::Result<Vec<u64>> {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };

    let mut inodes = Vec::new();

    // Fields (whitespace-separated):
    //  [0] sl  [1] local_addr:port  [2] rem_addr:port  [3] state
    //  [4] tx:rx  [5] tr:tm  [6] retrnsmt  [7] uid  [8] timeout  [9] inode
    for line in content.lines().skip(1) {
        let mut fields = line.split_whitespace();

        let _sl = fields.next();
        let local = match fields.next() {
            Some(s) => s,
            None => continue,
        };
        let _remote = fields.next();
        let state = match fields.next() {
            Some(s) => s,
            None => continue,
        };

        let local_port = local
            .rsplit(':')
            .next()
            .and_then(|s| u16::from_str_radix(s, 16).ok())
            .unwrap_or(0);
        if local_port != port {
            continue;
        }

        if is_tcp {
            let st = u32::from_str_radix(state, 16).unwrap_or(0);
            if st != TCP_LISTEN {
                continue;
            }
        }

        // nth(5) skips fields [4]–[8] and returns [9] (inode)
        if let Some(inode) = fields.nth(5).and_then(|s| s.parse::<u64>().ok())
            && inode > 0
        {
            inodes.push(inode);
        }
    }

    Ok(inodes)
}

/// Scan `/proc/<pid>/fd/<fd>` symlinks for one pointing to `socket:[<inode>]`.
fn find_pid_fd(inode: u64) -> io::Result<Option<(u32, RawFd)>> {
    let target = format!("socket:[{inode}]");

    let proc_dir = match fs::read_dir("/proc") {
        Ok(d) => d,
        Err(_) => return Ok(None),
    };

    for entry in proc_dir.flatten() {
        let pid: u32 = match entry.file_name().to_str().and_then(|s| s.parse().ok()) {
            Some(p) => p,
            None => continue,
        };

        let fd_dir = match fs::read_dir(format!("/proc/{pid}/fd")) {
            Ok(d) => d,
            Err(_) => continue,
        };

        for fd_entry in fd_dir.flatten() {
            let link = match fs::read_link(fd_entry.path()) {
                Ok(l) => l,
                Err(_) => continue,
            };

            if link.to_str() == Some(&target)
                && let Some(fd) = fd_entry.file_name().to_str().and_then(|s| s.parse().ok())
            {
                return Ok(Some((pid, fd)));
            }
        }
    }

    Ok(None)
}

/// Duplicate a socket fd from another process via `pidfd_getfd` and set `SO_REUSEPORT`.
fn set_reuse_port(pid: u32, fd: RawFd) -> io::Result<()> {
    unsafe {
        let raw_pfd = libc::syscall(libc::SYS_pidfd_open, pid as libc::pid_t, 0u32);
        if raw_pfd < 0 {
            return Err(io::Error::last_os_error());
        }
        let pfd = OwnedFd::from_raw_fd(raw_pfd as RawFd);

        let raw_sfd = libc::syscall(libc::SYS_pidfd_getfd, pfd.as_raw_fd(), fd, 0u32);
        if raw_sfd < 0 {
            return Err(io::Error::last_os_error());
        }
        let sfd = OwnedFd::from_raw_fd(raw_sfd as RawFd);

        let reuse: libc::c_int = 1;
        let ret = libc::setsockopt(
            sfd.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_REUSEPORT,
            &raw const reuse as *const libc::c_void,
            std::mem::size_of::<libc::c_int>() as libc::socklen_t,
        );
        if ret < 0 {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(())
}
