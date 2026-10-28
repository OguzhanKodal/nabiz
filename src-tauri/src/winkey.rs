//! Legion sürücüsü yokken Windows tuşunu kapatma: dahili klavyenin tuş haritasında
//! Meta tuşlarını KEY_RESERVED'e çevirir (EVIOCSKEYCODE_V2). Eski eşlemeler
//! /run altında saklanır; yeniden başlatınca harita da işaret de sıfırlanır.

use std::fs;
use std::path::{Path, PathBuf};

pub const MARK: &str = "/run/nabiz/winkey-off";

/// Dahili klavyelerin olay aygıtları: i8042 (bus 0x11) ya da ITE (048d) denetleyici.
pub fn devices() -> Vec<PathBuf> {
    let Ok(dir) = fs::read_dir("/sys/class/input") else { return vec![] };
    let rd = |p: &Path| fs::read_to_string(p).map(|s| s.trim().to_string()).unwrap_or_default();
    let mut out: Vec<PathBuf> = dir
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with("input")))
        .filter(|p| {
            let internal = rd(&p.join("id/bustype")) == "0011" || rd(&p.join("id/vendor")) == "048d";
            internal && rd(&p.join("name")).to_lowercase().contains("keyboard")
        })
        .filter_map(|p| {
            fs::read_dir(&p).ok()?.flatten().find_map(|e| {
                let n = e.file_name().into_string().ok()?;
                n.starts_with("event").then(|| PathBuf::from("/dev/input").join(n))
            })
        })
        .collect();
    out.sort();
    out
}

pub fn available() -> bool {
    !devices().is_empty()
}

pub fn enabled() -> bool {
    !Path::new(MARK).exists()
}

#[cfg(target_os = "linux")]
mod imp {
    use super::*;
    use std::os::fd::AsRawFd;

    const KEY_LEFTMETA: u32 = 125;
    const KEY_RIGHTMETA: u32 = 126;

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Entry {
        flags: u8,
        len: u8,
        index: u16,
        keycode: u32,
        scancode: [u8; 32],
    }
    const BY_INDEX: u8 = 1;
    // EVIOCGKEYCODE_V2 = _IOR('E', 0x04, entry), EVIOCSKEYCODE_V2 = _IOW('E', 0x04, entry)
    const SIZE: u64 = std::mem::size_of::<Entry>() as u64;
    const GET: u64 = (2 << 30) | (SIZE << 16) | ((b'E' as u64) << 8) | 0x04;
    const SET: u64 = (1 << 30) | (SIZE << 16) | ((b'E' as u64) << 8) | 0x04;

    fn hex(b: &[u8]) -> String {
        b.iter().map(|x| format!("{x:02x}")).collect()
    }

    fn unhex(s: &str) -> Option<Vec<u8>> {
        (0..s.len()).step_by(2).map(|i| u8::from_str_radix(s.get(i..i + 2)?, 16).ok()).collect()
    }

    fn set(f: &fs::File, scancode: &[u8], keycode: u32) -> std::io::Result<()> {
        let mut e = Entry { flags: 0, len: scancode.len() as u8, index: 0, keycode, scancode: [0; 32] };
        e.scancode[..scancode.len()].copy_from_slice(scancode);
        if unsafe { libc::ioctl(f.as_raw_fd(), SET as _, &e) } < 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(())
    }

    /// Satır biçimi: "<aygıt> <scancode-hex> <keycode>"
    pub fn disable() -> std::io::Result<()> {
        if !enabled() {
            return Ok(());
        }
        let mut saved = String::new();
        for dev in devices() {
            let f = fs::OpenOptions::new().read(true).write(true).open(&dev)?;
            let mut hits = vec![];
            for index in 0..=u16::MAX {
                let mut e = Entry { flags: BY_INDEX, len: 0, index, keycode: 0, scancode: [0; 32] };
                if unsafe { libc::ioctl(f.as_raw_fd(), GET as _, &mut e) } < 0 {
                    break;
                }
                if e.keycode == KEY_LEFTMETA || e.keycode == KEY_RIGHTMETA {
                    hits.push((e.scancode[..e.len as usize].to_vec(), e.keycode));
                }
            }
            for (sc, kc) in hits {
                set(&f, &sc, 0)?;
                saved += &format!("{} {} {kc}\n", dev.display(), hex(&sc));
            }
        }
        fs::create_dir_all(Path::new(MARK).parent().unwrap())?;
        fs::write(MARK, saved)
    }

    pub fn enable() -> std::io::Result<()> {
        let Ok(saved) = fs::read_to_string(MARK) else { return Ok(()) };
        for line in saved.lines() {
            let mut it = line.split_whitespace();
            let (Some(dev), Some(sc), Some(kc)) = (it.next(), it.next().and_then(unhex), it.next()) else { continue };
            let Ok(kc) = kc.parse() else { continue };
            // Yalnız kendi yazdığımız aygıtlara dokun.
            if !devices().iter().any(|d| d.as_os_str() == dev) {
                continue;
            }
            let f = fs::OpenOptions::new().read(true).write(true).open(dev)?;
            set(&f, &sc, kc)?;
        }
        fs::remove_file(MARK)
    }
}

#[cfg(target_os = "linux")]
pub fn apply(on: bool) -> std::io::Result<()> {
    if on { imp::enable() } else { imp::disable() }
}

#[cfg(not(target_os = "linux"))]
pub fn apply(_: bool) -> std::io::Result<()> {
    Err(std::io::Error::other("yalnız Linux"))
}
