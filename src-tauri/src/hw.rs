//! Donanım ayarları: sysfs yolu keşfi, beyaz liste, doğrulama ve yazma.
//! Yollar: reports/ozellik-haritasi.md

use serde::Serialize;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::winkey;

#[derive(Clone, Copy, PartialEq)]
pub enum Kind {
    Toggle,
    Level,
}

pub struct Def {
    pub id: &'static str,
    pub kind: Kind,
}

/// Yazılabilen her şey burada. Listede olmayan kimlik reddedilir.
pub const DEFS: &[Def] = &[
    Def { id: "conservation", kind: Kind::Toggle },
    Def { id: "rapid_charge", kind: Kind::Toggle },
    Def { id: "usb_charging", kind: Kind::Toggle },
    Def { id: "gsync", kind: Kind::Toggle },
    Def { id: "overdrive", kind: Kind::Toggle },
    Def { id: "fn_lock", kind: Kind::Toggle },
    Def { id: "win_key", kind: Kind::Toggle },
    Def { id: "touchpad", kind: Kind::Toggle },
    Def { id: "camera", kind: Kind::Toggle },
    Def { id: "kbd_backlight", kind: Kind::Level },
    Def { id: "logo_led", kind: Kind::Toggle },
    Def { id: "ioport_led", kind: Kind::Toggle },
    Def { id: "fan_max", kind: Kind::Toggle },
    Def { id: "instant_boot_ac", kind: Kind::Toggle },
    Def { id: "instant_boot_usb_pd", kind: Kind::Toggle },
];

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FeatureState {
    pub id: String,
    pub available: bool,
    pub value: Option<i64>,
    pub max: Option<i64>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProfileState {
    pub current: Option<String>,
    pub choices: Vec<String>,
}

pub fn read_trim(p: &Path) -> Option<String> {
    fs::read_to_string(p).ok().map(|s| s.trim().to_string())
}

fn existing(p: &str) -> Option<PathBuf> {
    let p = PathBuf::from(p);
    p.exists().then_some(p)
}

fn ideapad_base() -> Option<PathBuf> {
    fs::read_dir("/sys/bus/platform/drivers/ideapad_acpi")
        .ok()?
        .flatten()
        .map(|e| e.path())
        .find(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("VPC2004"))
        })
}

fn legion_base() -> Option<PathBuf> {
    [
        "/sys/module/legion_laptop/drivers/platform:legion/legion",
        "/sys/module/legion_laptop/drivers/platform:legion/PNP0C09:00",
    ]
    .iter()
    .find_map(|p| existing(p))
}

fn led(name_suffix: &str) -> Option<PathBuf> {
    fs::read_dir("/sys/class/leds")
        .ok()?
        .flatten()
        .map(|e| e.path())
        .find(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.ends_with(name_suffix))
        })
        .map(|p| p.join("brightness"))
        .filter(|p| p.exists())
}

/// Dahili touchpad'in kernel düzeyinde kapatma dosyası (masaüstünden bağımsız).
fn touchpad_inhibit() -> Option<PathBuf> {
    fs::read_dir("/sys/class/input")
        .ok()?
        .flatten()
        .map(|e| e.path())
        .find(|p| read_trim(&p.join("name")).is_some_and(|n| n.ends_with("Touchpad")))
        .map(|p| p.join("inhibited"))
        .filter(|p| p.exists())
}

/// Dahili USB kamera: yapılandırma tanımlayıcısında video arayüzü (sınıf 0x0e) olan aygıt.
/// `authorized` = 0 aygıtı sistemden ayırır; tanımlayıcılar kapalıyken de okunabilir.
fn usb_camera() -> Option<PathBuf> {
    fs::read_dir("/sys/bus/usb/devices")
        .ok()?
        .flatten()
        .map(|e| e.path())
        .find(|p| {
            let Ok(d) = fs::read(p.join("descriptors")) else { return false };
            let mut i = 0;
            while i + 5 < d.len() && d[i] > 0 {
                if d[i + 1] == 4 && d[i + 5] == 0x0e {
                    return read_trim(&p.join("removable")).as_deref() != Some("removable");
                }
                i += d[i] as usize;
            }
            false
        })
        .map(|p| p.join("authorized"))
}

/// Yeni çekirdeklerde pilin `charge_types` dosyası: "Fast Standard [Long_Life]".
fn charge_types() -> Option<PathBuf> {
    fs::read_dir("/sys/class/power_supply")
        .ok()?
        .flatten()
        .map(|e| e.path().join("charge_types"))
        .find(|p| read_trim(p).is_some_and(|s| s.split_whitespace().any(|t| t.trim_matches(['[', ']']) == "Fast")))
}

fn is_charge_types(p: &Path) -> bool {
    p.file_name().is_some_and(|n| n == "charge_types")
}

/// `inhibited` dosyası tersine çalışır: 1 = aygıt kapalı.
fn is_inhibit(p: &Path) -> bool {
    p.file_name().is_some_and(|n| n == "inhibited")
}

fn is_winkey_mark(p: &Path) -> bool {
    p == Path::new(winkey::MARK)
}

pub fn read_value(path: &Path) -> Option<i64> {
    if is_winkey_mark(path) {
        return Some(winkey::enabled() as i64);
    }
    let raw = read_trim(path)?;
    if is_inhibit(path) {
        return Some((raw == "0") as i64);
    }
    if is_charge_types(path) {
        return Some(raw.contains("[Fast]") as i64);
    }
    raw.parse().ok()
}

fn write_value(path: &Path, v: i64) -> std::io::Result<()> {
    if is_winkey_mark(path) {
        return winkey::apply(v == 1);
    }
    if is_inhibit(path) {
        return fs::write(path, if v == 1 { "0" } else { "1" });
    }
    if is_charge_types(path) {
        return fs::write(path, if v == 1 { "Fast" } else { "Standard" });
    }
    fs::write(path, v.to_string())
}

pub fn path_of(id: &str) -> Option<PathBuf> {
    let ide = ideapad_base();
    let leg = legion_base();
    let at = |b: &Option<PathBuf>, f: &str| b.as_ref().map(|b| b.join(f)).filter(|p| p.exists());
    match id {
        "conservation" => at(&ide, "conservation_mode"),
        "rapid_charge" => at(&leg, "rapidcharge").or_else(charge_types),
        "usb_charging" => at(&ide, "usb_charging"),
        "gsync" => at(&leg, "gsync"),
        "overdrive" => at(&leg, "overdrive"),
        "fn_lock" => at(&ide, "fn_lock"),
        // Elle doğrulanmış yollar önce; Legion sürücüsünün WMI yolu yedek.
        "win_key" => winkey::available().then(|| PathBuf::from(winkey::MARK)).or_else(|| at(&leg, "winkey")),
        "touchpad" => at(&ide, "touchpad").or_else(touchpad_inhibit).or_else(|| at(&leg, "touchpad")),
        // Bazı modellerde (ör. 15ARH05) camera_power hep 0 okunur ve yazmayı yok sayar;
        // bu yüzden önce USB aygıtını ayırma yolunu dene.
        "camera" => usb_camera().or_else(|| at(&ide, "camera_power")),
        "kbd_backlight" => led("::kbd_backlight"),
        "logo_led" => existing("/sys/class/leds/platform::ylogo/brightness"),
        "ioport_led" => existing("/sys/class/leds/platform::ioport/brightness"),
        // Bazı modellerde (ör. EUCN) dosya var ama okuma/yazma ENODEV döner.
        "fan_max" => at(&leg, "fan_fullspeed").filter(|p| read_value(p).is_some()),
        "instant_boot_ac" => at(&leg, "instant_boot_ac"),
        "instant_boot_usb_pd" => at(&leg, "instant_boot_usb_pd"),
        _ => None,
    }
}

fn max_of(id: &str, path: &Path) -> Option<i64> {
    match def(id)?.kind {
        Kind::Toggle => Some(1),
        Kind::Level => read_trim(&path.with_file_name("max_brightness"))?.parse().ok(),
    }
}

fn def(id: &str) -> Option<&'static Def> {
    DEFS.iter().find(|d| d.id == id)
}

pub fn features() -> Vec<FeatureState> {
    DEFS.iter()
        .map(|d| match path_of(d.id) {
            Some(p) => FeatureState {
                id: d.id.into(),
                available: true,
                value: read_value(&p),
                max: max_of(d.id, &p),
            },
            None => FeatureState { id: d.id.into(), available: false, value: None, max: None },
        })
        .collect()
}

fn profile_paths() -> Option<(PathBuf, PathBuf)> {
    let legacy = PathBuf::from("/sys/firmware/acpi/platform_profile");
    if legacy.exists() {
        return Some((legacy, PathBuf::from("/sys/firmware/acpi/platform_profile_choices")));
    }
    let dir = fs::read_dir("/sys/class/platform-profile").ok()?.flatten().next()?.path();
    Some((dir.join("profile"), dir.join("choices")))
}

pub fn profile() -> ProfileState {
    match profile_paths() {
        Some((cur, choices)) => ProfileState {
            current: read_trim(&cur),
            choices: read_trim(&choices)
                .map(|s| s.split_whitespace().map(String::from).collect())
                .unwrap_or_default(),
        },
        None => ProfileState { current: None, choices: vec![] },
    }
}

/// Değeri doğrular; geçersizse yazmadan reddeder.
pub fn validate(id: &str, value: &str) -> Result<i64, String> {
    let d = def(id).ok_or_else(|| format!("bilinmeyen ayar: {id}"))?;
    let v: i64 = value.parse().map_err(|_| format!("geçersiz değer: {value}"))?;
    let path = path_of(id).ok_or("bu ayar bu cihazda yok")?;
    let max = max_of(id, &path).unwrap_or(if d.kind == Kind::Toggle { 1 } else { 0 });
    if v < 0 || v > max {
        return Err(format!("değer 0–{max} aralığında olmalı"));
    }
    Ok(v)
}

/// Yetki gerektirmeden, doğrudan yazar. Helper da bunu çağırır.
pub fn apply_feature(id: &str, value: &str) -> std::io::Result<()> {
    let v = validate(id, value).map_err(|e| std::io::Error::new(ErrorKind::InvalidInput, e))?;
    // Pil koruma ve hızlı şarj aynı anda açık olamaz.
    if v == 1 {
        let other = match id {
            "conservation" => Some("rapid_charge"),
            "rapid_charge" => Some("conservation"),
            _ => None,
        };
        if let Some(p) = other.and_then(path_of) {
            if read_value(&p) == Some(1) {
                write_value(&p, 0)?;
            }
        }
    }
    let path = path_of(id).unwrap();
    write_value(&path, v)?;
    // Bazı firmware'ler yazmayı hatasız kabul edip uygulamaz; geri okuyarak doğrula.
    if read_value(&path) != Some(v) {
        return Err(rejected());
    }
    Ok(())
}

fn rejected() -> std::io::Error {
    std::io::Error::other("Donanım bu değişikliği kabul etmedi")
}

pub fn apply_profile(value: &str) -> std::io::Result<()> {
    let (cur, _) = profile_paths()
        .ok_or_else(|| std::io::Error::new(ErrorKind::NotFound, "güç modu desteklenmiyor"))?;
    if !profile().choices.iter().any(|c| c == value) {
        return Err(std::io::Error::new(ErrorKind::InvalidInput, format!("geçersiz mod: {value}")));
    }
    fs::write(&cur, value)?;
    if read_trim(&cur).as_deref() != Some(value) {
        return Err(rejected());
    }
    Ok(())
}

/// Önce doğrudan dener; izin yoksa pkexec ile helper'ı çağırır.
pub fn write_with_elevation(
    direct: impl FnOnce() -> std::io::Result<()>,
    helper_args: &[&str],
) -> Result<(), String> {
    match direct() {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == ErrorKind::PermissionDenied => elevate(helper_args),
        Err(e) => Err(e.to_string()),
    }
}

fn elevate(args: &[&str]) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let out = Command::new("pkexec")
        .arg(exe)
        .arg("helper")
        .args(args)
        .output()
        .map_err(|e| format!("pkexec çalıştırılamadı: {e}"))?;
    match out.status.code() {
        Some(0) => Ok(()),
        Some(126) | Some(127) => Err("İzin verilmedi".into()),
        _ => Err(String::from_utf8_lossy(&out.stderr).trim().to_string()),
    }
}
