//! Salt okunur izleme: sıcaklık, fan, CPU, bellek, GPU, pil, sistem bilgisi.

use crate::hw::read_trim;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

static FANS_BROKEN: AtomicBool = AtomicBool::new(false);
static GPU_CACHE: Mutex<Option<(Instant, Gpu)>> = Mutex::new(None);

#[derive(Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct Fan {
    pub label: String,
    pub rpm: u32,
    pub max: u32,
}

#[derive(Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct Gpu {
    pub present: bool,
    pub name: Option<String>,
    /// "active" | "suspended" | ...
    pub state: Option<String>,
    pub temp: Option<f32>,
    pub util: Option<f32>,
    pub power: Option<f32>,
}

#[derive(Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct Battery {
    pub present: bool,
    pub percent: Option<u8>,
    pub status: Option<String>,
    pub ac_online: bool,
    /// Watt; şarj olurken giren, deşarjda çekilen güç.
    pub power: Option<f32>,
    pub health: Option<f32>,
    pub cycles: Option<u32>,
    pub design_wh: Option<f32>,
    pub full_wh: Option<f32>,
}

#[derive(Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct Sensors {
    pub cpu_temp: Option<f32>,
    pub cpu_usage: Option<f32>,
    pub cpu_freq_mhz: Option<u32>,
    pub mem_used_gb: Option<f32>,
    pub mem_total_gb: Option<f32>,
    pub ic_temp: Option<f32>,
    pub fans: Vec<Fan>,
    /// Sensör sıcakken de 0 gösteriyor; değerleri gösterilmez.
    pub fans_unreliable: bool,
    pub gpu: Gpu,
    pub battery: Battery,
}

#[derive(Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct SystemInfo {
    pub model: Option<String>,
    pub machine_type: Option<String>,
    pub vendor: Option<String>,
    pub bios: Option<String>,
    pub kernel: Option<String>,
    pub os: Option<String>,
    pub cpu: Option<String>,
    pub legion_driver: bool,
    pub ideapad_driver: bool,
    pub demo: bool,
}

fn num<T: std::str::FromStr>(p: &Path) -> Option<T> {
    read_trim(p)?.parse().ok()
}

fn hwmons() -> Vec<(String, PathBuf)> {
    fs::read_dir("/sys/class/hwmon")
        .map(|rd| {
            rd.flatten()
                .filter_map(|e| {
                    let p = e.path();
                    Some((read_trim(&p.join("name"))?, p))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn milli(p: &Path) -> Option<f32> {
    num::<f32>(p).map(|v| v / 1000.0)
}

static CPU_PREV: Mutex<Option<(u64, u64)>> = Mutex::new(None);

fn cpu_usage() -> Option<f32> {
    let stat = fs::read_to_string("/proc/stat").ok()?;
    let vals: Vec<u64> = stat
        .lines()
        .next()?
        .split_whitespace()
        .skip(1)
        .filter_map(|v| v.parse().ok())
        .collect();
    let idle = vals.get(3)? + vals.get(4).unwrap_or(&0);
    let total: u64 = vals.iter().sum();
    let mut prev = CPU_PREV.lock().ok()?;
    let usage = prev.and_then(|(pt, pi)| {
        let dt = total.saturating_sub(pt);
        (dt > 0).then(|| 100.0 * (1.0 - (idle.saturating_sub(pi)) as f32 / dt as f32))
    });
    *prev = Some((total, idle));
    usage
}

fn cpu_freq() -> Option<u32> {
    let freqs: Vec<u32> = fs::read_dir("/sys/devices/system/cpu")
        .ok()?
        .flatten()
        .filter_map(|e| num::<u32>(&e.path().join("cpufreq/scaling_cur_freq")))
        .collect();
    (!freqs.is_empty()).then(|| freqs.iter().sum::<u32>() / freqs.len() as u32 / 1000)
}

fn meminfo() -> (Option<f32>, Option<f32>) {
    let Ok(s) = fs::read_to_string("/proc/meminfo") else { return (None, None) };
    let field = |k: &str| {
        s.lines()
            .find(|l| l.starts_with(k))
            .and_then(|l| l.split_whitespace().nth(1))
            .and_then(|v| v.parse::<f32>().ok())
            .map(|kb| kb / 1024.0 / 1024.0)
    };
    match (field("MemTotal:"), field("MemAvailable:")) {
        (Some(t), Some(a)) => (Some(t - a), Some(t)),
        (t, _) => (None, t),
    }
}

fn nvidia_pci() -> Option<PathBuf> {
    fs::read_dir("/sys/bus/pci/devices").ok()?.flatten().map(|e| e.path()).find(|p| {
        read_trim(&p.join("vendor")).as_deref() == Some("0x10de")
            && read_trim(&p.join("class")).is_some_and(|c| c.starts_with("0x03"))
    })
}

fn gpu(legion_gpu_temp: Option<f32>) -> Gpu {
    let Some(pci) = nvidia_pci() else {
        return Gpu { temp: legion_gpu_temp, ..Default::default() };
    };
    let state = read_trim(&pci.join("power/runtime_status"));
    let mut g = Gpu { present: true, state: state.clone(), temp: legion_gpu_temp, ..Default::default() };
    // Kart uyurken nvidia-smi çağırmak onu uyandırır; yalnız aktifken sor.
    // Boştaysa (kullanım %0) 20 sn'de bir sor ki sürekli sorgu kartı uyanık tutmasın.
    if state.as_deref() == Some("active") {
        let mut cache = GPU_CACHE.lock().unwrap();
        if let Some((at, c)) = cache.as_ref() {
            if c.util == Some(0.0) && at.elapsed() < Duration::from_secs(20) {
                return Gpu { present: true, state, ..c.clone() };
            }
        }
        if let Ok(out) = Command::new("nvidia-smi")
            .args([
                "--query-gpu=name,temperature.gpu,utilization.gpu,power.draw",
                "--format=csv,noheader,nounits",
            ])
            .output()
        {
            let line = String::from_utf8_lossy(&out.stdout);
            let parts: Vec<&str> = line.trim().split(", ").collect();
            if parts.len() == 4 {
                g.name = Some(parts[0].to_string());
                g.temp = parts[1].parse().ok().or(g.temp);
                g.util = parts[2].parse().ok();
                g.power = parts[3].parse().ok();
            }
        }
        *cache = Some((Instant::now(), g.clone()));
    }
    g
}

fn battery() -> Battery {
    let mut b = Battery::default();
    let Ok(rd) = fs::read_dir("/sys/class/power_supply") else { return b };
    for p in rd.flatten().map(|e| e.path()) {
        match read_trim(&p.join("type")).as_deref() {
            Some("Mains") => b.ac_online |= read_trim(&p.join("online")).as_deref() == Some("1"),
            Some("Battery") if !b.present => {
                b.present = true;
                b.percent = num(&p.join("capacity"));
                b.status = read_trim(&p.join("status"));
                b.cycles = num(&p.join("cycle_count")).filter(|c: &u32| *c > 0);
                let volt = num::<f32>(&p.join("voltage_now")).map(|v| v / 1e6);
                let (full, design) = match (
                    num::<f32>(&p.join("energy_full")),
                    num::<f32>(&p.join("energy_full_design")),
                ) {
                    (Some(f), Some(d)) => (Some(f / 1e6), Some(d / 1e6)),
                    _ => {
                        // charge_* (µAh) → Wh, tasarım gerilimiyle
                        let vd = num::<f32>(&p.join("voltage_min_design")).map(|v| v / 1e6).or(volt);
                        let f = num::<f32>(&p.join("charge_full"));
                        let d = num::<f32>(&p.join("charge_full_design"));
                        (f.zip(vd).map(|(f, v)| f / 1e6 * v), d.zip(vd).map(|(d, v)| d / 1e6 * v))
                    }
                };
                b.full_wh = full;
                b.design_wh = design;
                b.health = full.zip(design).filter(|(_, d)| *d > 0.0).map(|(f, d)| (f / d * 100.0).min(100.0));
                b.power = num::<f32>(&p.join("power_now"))
                    .map(|w| w / 1e6)
                    .or_else(|| num::<f32>(&p.join("current_now")).zip(volt).map(|(c, v)| c / 1e6 * v))
                    .filter(|w| *w > 0.0);
            }
            _ => {}
        }
    }
    b
}

pub fn read() -> Sensors {
    let mons = hwmons();
    let find = |name: &str| mons.iter().find(|(n, _)| n == name).map(|(_, p)| p.clone());
    let legion = find("legion_hwmon");

    let cpu_temp = ["coretemp", "k10temp", "zenpower"]
        .iter()
        .find_map(|n| find(n))
        .and_then(|p| milli(&p.join("temp1_input")))
        .or_else(|| legion.as_ref().and_then(|p| milli(&p.join("temp1_input"))));

    let fans: Vec<Fan> = match &legion {
        Some(p) => (1..=2)
            .filter_map(|i| {
                let rpm = num::<u32>(&p.join(format!("fan{i}_input")))?;
                Some(Fan {
                    label: if i == 1 { "CPU fanı".into() } else { "GPU fanı".into() },
                    rpm,
                    max: num(&p.join(format!("fan{i}_max"))).filter(|m| *m > 0).unwrap_or(5000),
                })
            })
            .collect(),
        None => mons
            .iter()
            .flat_map(|(_, p)| {
                (1..=4).filter_map(move |i| {
                    Some(Fan {
                        label: format!("Fan {i}"),
                        rpm: num(&p.join(format!("fan{i}_input")))?,
                        max: 5000,
                    })
                })
            })
            .collect(),
    };

    // Fanlar 75 °C üstünde hâlâ 0 gösteriyorsa sensör yalan söylüyordur (ör. yogafan, 15ARH05).
    let all_zero = !fans.is_empty() && fans.iter().all(|f| f.rpm == 0);
    if all_zero && cpu_temp.is_some_and(|t| t > 75.0) {
        FANS_BROKEN.store(true, Ordering::Relaxed);
    } else if !all_zero {
        FANS_BROKEN.store(false, Ordering::Relaxed);
    }
    let fans_unreliable = FANS_BROKEN.load(Ordering::Relaxed);

    let (mem_used_gb, mem_total_gb) = meminfo();
    Sensors {
        cpu_temp,
        cpu_usage: cpu_usage(),
        cpu_freq_mhz: cpu_freq(),
        mem_used_gb,
        mem_total_gb,
        ic_temp: legion.as_ref().and_then(|p| milli(&p.join("temp3_input"))),
        fans: if fans_unreliable { vec![] } else { fans },
        fans_unreliable,
        gpu: gpu(legion.as_ref().and_then(|p| milli(&p.join("temp2_input")))),
        battery: battery(),
    }
}

pub fn system() -> SystemInfo {
    let dmi = |f: &str| read_trim(&Path::new("/sys/class/dmi/id").join(f)).filter(|s| !s.is_empty());
    let os = fs::read_to_string("/etc/os-release").ok().and_then(|s| {
        s.lines()
            .find_map(|l| l.strip_prefix("PRETTY_NAME="))
            .map(|v| v.trim_matches('"').to_string())
    });
    let cpu = fs::read_to_string("/proc/cpuinfo").ok().and_then(|s| {
        s.lines()
            .find(|l| l.starts_with("model name"))
            .and_then(|l| l.split(':').nth(1))
            .map(|v| v.trim().to_string())
    });
    SystemInfo {
        model: dmi("product_version"),
        machine_type: dmi("product_name"),
        vendor: dmi("sys_vendor"),
        bios: dmi("bios_version"),
        kernel: read_trim(Path::new("/proc/sys/kernel/osrelease")),
        os,
        cpu,
        legion_driver: Path::new("/sys/module/legion_laptop").exists(),
        ideapad_driver: Path::new("/sys/module/ideapad_laptop").exists(),
        demo: false,
    }
}
