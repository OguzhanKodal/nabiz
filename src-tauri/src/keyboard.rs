//! 4 bölgeli RGB klavye (Legion 5 / Ideapad Gaming 2020–2024, ITE 048d:c9xx).
//! Protokol: 33 baytlık HID feature report, L5P-Keyboard-RGB (GPL-3.0) projesinden.

use crate::hw::read_trim;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const PIDS: &[&str] = &["C955", "C963", "C965", "C973", "C975", "C983", "C984", "C985", "C993", "C994", "C995"];

#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct KbdState {
    /// static | breath | smooth | wave-left | wave-right | off
    pub effect: String,
    pub speed: u8,
    pub brightness: u8,
    /// 4 bölge, "rrggbb"
    pub colors: Vec<String>,
}

impl Default for KbdState {
    fn default() -> Self {
        KbdState {
            effect: "static".into(),
            speed: 2,
            brightness: 2,
            colors: vec!["3ee0b0".into(); 4],
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KbdInfo {
    pub available: bool,
    pub state: KbdState,
}

/// Doğru arayüzü bulur: aynı USB aygıtının birden çok hidraw'ı var; renk komutunu
/// yalnız 0xFF89 kullanım sayfalı olan kabul eder.
pub fn device() -> Option<PathBuf> {
    fs::read_dir("/sys/class/hidraw").ok()?.flatten().find_map(|e| {
        let dev = e.path().join("device");
        let id = read_trim(&dev.join("uevent"))?
            .lines()
            .find_map(|l| l.strip_prefix("HID_ID=").map(str::to_uppercase))?;
        let known = id.contains(":0000048D:") && PIDS.iter().any(|p| id.ends_with(&format!(":0000{p}")));
        let desc = fs::read(dev.join("report_descriptor")).ok()?;
        let vendor_page = desc.windows(3).any(|w| w == [0x06, 0x89, 0xff]);
        (known && vendor_page).then(|| PathBuf::from("/dev").join(e.file_name()))
    })
}

fn hex(s: &str) -> Option<[u8; 3]> {
    if s.len() != 6 {
        return None;
    }
    let b = |i: usize| u8::from_str_radix(&s[i..i + 2], 16).ok();
    Some([b(0)?, b(2)?, b(4)?])
}

pub fn validate(s: &KbdState) -> Result<(), String> {
    let effects = ["static", "breath", "smooth", "wave-left", "wave-right", "off"];
    if !effects.contains(&s.effect.as_str()) {
        return Err(format!("geçersiz efekt: {}", s.effect));
    }
    if !(1..=4).contains(&s.speed) || !(1..=2).contains(&s.brightness) {
        return Err("hız 1–4, parlaklık 1–2 olmalı".into());
    }
    if s.colors.len() != 4 || s.colors.iter().any(|c| hex(c).is_none()) {
        return Err("4 bölge için rrggbb renk gerekli".into());
    }
    Ok(())
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn payload(s: &KbdState) -> [u8; 33] {
    let mut p = [0u8; 33];
    p[0] = 0xcc;
    p[1] = 0x16;
    p[2] = match s.effect.as_str() {
        "breath" => 0x03,
        "smooth" => 0x06,
        "wave-left" => {
            p[19] = 1;
            0x04
        }
        "wave-right" => {
            p[18] = 1;
            0x04
        }
        _ => 0x01,
    };
    p[3] = s.speed;
    p[4] = s.brightness;
    // "off" = tüm bölgeler siyah, sabit
    if matches!(s.effect.as_str(), "static" | "breath") {
        for (i, c) in s.colors.iter().enumerate() {
            p[5 + i * 3..8 + i * 3].copy_from_slice(&hex(c).unwrap());
        }
    }
    p
}

#[cfg(target_os = "linux")]
pub fn apply(s: &KbdState) -> std::io::Result<()> {
    use std::os::fd::AsRawFd;
    validate(s).map_err(std::io::Error::other)?;
    let path = device().ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "RGB klavye bulunamadı"))?;
    let f = fs::OpenOptions::new().read(true).write(true).open(path)?;
    let buf = payload(s);
    // HIDIOCSFEATURE(len) = _IOC(_IOC_READ | _IOC_WRITE, 'H', 0x06, len)
    let req = (3u64 << 30) | ((buf.len() as u64) << 16) | ((b'H' as u64) << 8) | 0x06;
    let r = unsafe { libc::ioctl(f.as_raw_fd(), req as _, buf.as_ptr()) };
    if r < 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(not(target_os = "linux"))]
pub fn apply(_: &KbdState) -> std::io::Result<()> {
    Err(std::io::Error::other("yalnız Linux"))
}

fn config_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("nabiz").join("keyboard.json"))
}

/// Klavyeden durum okunamaz; son uygulanan ayar kullanıcının config'inde tutulur.
pub fn saved() -> KbdState {
    config_path()
        .and_then(|p| fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .filter(|s| validate(s).is_ok())
        .unwrap_or_default()
}

pub fn save(s: &KbdState) {
    if let Some(p) = config_path() {
        let _ = fs::create_dir_all(p.parent().unwrap());
        let _ = fs::write(p, serde_json::to_string_pretty(s).unwrap());
    }
}
