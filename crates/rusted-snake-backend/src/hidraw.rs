use std::fs::{File, OpenOptions};
use std::io;
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::thread::sleep;
use std::time::Duration;

use anyhow::{Context, Result, bail};

const REPORT: usize = 91;

#[derive(Clone, Debug)]
pub struct Reply {
    pub status: i32,
    pub args: Vec<u8>,
}

impl Reply {
    pub fn ok(&self) -> bool {
        self.status == 0x02
    }
}

/// Raw Razer feature reports over `/dev/hidrawN` (interface 0), the same
/// 90-byte report OpenRazer uses.
pub struct Hidraw {
    file: File,
    path: PathBuf,
    transaction_id: u8,
}

impl Hidraw {
    pub fn open(vid: u16, pid: u16) -> Result<Option<Self>> {
        let Some(path) = find_node(vid, pid)? else {
            return Ok(None);
        };
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .with_context(|| {
                format!(
                    "cannot open {} ({}) — install the Rusted Snake udev rule or grant access to this mouse",
                    path.display(),
                    io::Error::last_os_error()
                )
            })?;
        Ok(Some(Self {
            file,
            path,
            transaction_id: 0x1f,
        }))
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn set_transaction_id(&mut self, transaction_id: u8) {
        self.transaction_id = transaction_id;
    }

    pub fn send(
        &mut self,
        class: u8,
        id: u8,
        size: usize,
        args: &[u8],
        echo: usize,
    ) -> Result<Reply> {
        let matching: Vec<usize> = (0..echo.min(args.len())).collect();
        self.send_matching(class, id, size, args, &matching)
    }

    pub fn send_matching(
        &mut self,
        class: u8,
        id: u8,
        size: usize,
        args: &[u8],
        echo: &[usize],
    ) -> Result<Reply> {
        let mut report = [0u8; REPORT];
        report[2] = self.transaction_id;
        report[6] = size as u8;
        report[7] = class;
        report[8] = id;
        let copy = args.len().min(REPORT - 9);
        report[9..9 + copy].copy_from_slice(&args[..copy]);
        let mut crc = 0u8;
        for value in &report[3..89] {
            crc ^= value;
        }
        report[89] = crc;

        let mut last = None;
        for attempt in 0..3 {
            if attempt > 0 {
                sleep(Duration::from_millis(15));
            }
            self.ioctl_write(&mut report)?;
            for read in 0..60 {
                sleep(match read {
                    0 => Duration::from_millis(20),
                    1..=9 => Duration::from_millis(5),
                    _ => Duration::from_millis(20),
                });
                let mut response = [0u8; REPORT];
                let result = unsafe {
                    libc::ioctl(self.file.as_raw_fd(), request(0x07), response.as_mut_ptr())
                };
                if result < 0 {
                    bail!(
                        "read from {} failed: {}",
                        self.path.display(),
                        io::Error::last_os_error()
                    );
                }
                let status = response[1] as i32;
                if status == 0x01 || response[7] != class || response[8] != id {
                    continue;
                }
                if !echo.iter().all(|index| {
                    let value = args.get(*index).copied().unwrap_or(0);
                    response.get(9 + index).copied().unwrap_or(0) == value
                }) {
                    continue;
                }
                let args_len = size.min(80);
                let reply = Reply {
                    status,
                    args: response[9..9 + args_len].to_vec(),
                };
                last = Some(reply.clone());
                if status != 0x00 {
                    return Ok(reply);
                }
            }
        }
        Ok(last.unwrap_or(Reply {
            status: -1,
            args: vec![0; size.min(80)],
        }))
    }

    fn ioctl_write(&self, report: &mut [u8; REPORT]) -> Result<()> {
        let result =
            unsafe { libc::ioctl(self.file.as_raw_fd(), request(0x06), report.as_mut_ptr()) };
        if result < 0 {
            bail!(
                "write to {} failed: {}",
                self.path.display(),
                io::Error::last_os_error()
            );
        }
        Ok(())
    }
}

fn request(number: u8) -> libc::c_ulong {
    ((3u64 << 30) | ((REPORT as u64) << 16) | ((b'H' as u64) << 8) | number as u64) as libc::c_ulong
}

fn find_node(vid: u16, pid: u16) -> Result<Option<PathBuf>> {
    let root = Path::new("/sys/class/hidraw");
    if !root.is_dir() {
        return Ok(None);
    }
    let wanted = format!("{vid:08X}:{pid:08X}");
    let mut entries: Vec<_> = std::fs::read_dir(root)
        .context("cannot read /sys/class/hidraw")?
        .filter_map(|entry| entry.ok())
        .collect();
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let device = entry.path().join("device");
        let Ok(uevent) = std::fs::read_to_string(device.join("uevent")) else {
            continue;
        };
        if !uevent.to_uppercase().contains(&wanted) {
            continue;
        }
        let Ok(real) = device.canonicalize() else {
            continue;
        };
        let interface = real
            .parent()
            .and_then(|parent| parent.file_name())
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        if interface.ends_with(":1.0") {
            return Ok(Some(Path::new("/dev").join(entry.file_name())));
        }
    }
    Ok(None)
}
