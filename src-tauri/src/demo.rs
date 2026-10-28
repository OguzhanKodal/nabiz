//! Linux dışında (veya NABIZ_DEMO=1) arayüzü geliştirmek için sahte donanım.

use crate::hw::{FeatureState, ProfileState, DEFS};
use crate::sensors::{Battery, Fan, Gpu, Sensors, SystemInfo};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn enabled() -> bool {
    cfg!(not(target_os = "linux")) || std::env::var_os("NABIZ_DEMO").is_some()
}

struct State {
    values: HashMap<&'static str, i64>,
    profile: String,
}

static STATE: Mutex<Option<State>> = Mutex::new(None);

fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
    let mut g = STATE.lock().unwrap();
    let s = g.get_or_insert_with(|| State {
        values: HashMap::from([
            ("conservation", 1),
            ("rapid_charge", 0),
            ("usb_charging", 0),
            ("gsync", 0),
            ("overdrive", 1),
            ("fn_lock", 0),
            ("win_key", 1),
            ("touchpad", 1),
            ("camera", 1),
            ("kbd_backlight", 1),
            ("logo_led", 1),
            ("ioport_led", 0),
            ("fan_max", 0),
            ("instant_boot_ac", 0),
        ]),
        profile: "balanced".into(),
    });
    f(s)
}

fn t() -> f32 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as f32 / 1000.0
}

fn load() -> f32 {
    with(|s| match s.profile.as_str() {
        "quiet" => 0.55,
        "performance" => 1.35,
        _ => 1.0,
    })
}

pub fn features() -> Vec<FeatureState> {
    with(|s| {
        DEFS.iter()
            .map(|d| {
                let v = s.values.get(d.id).copied();
                FeatureState {
                    id: d.id.into(),
                    available: v.is_some(),
                    value: v,
                    max: v.map(|_| if d.id == "kbd_backlight" { 2 } else { 1 }),
                }
            })
            .collect()
    })
}

pub fn profile() -> ProfileState {
    with(|s| ProfileState {
        current: Some(s.profile.clone()),
        choices: ["quiet", "balanced", "performance"].map(String::from).to_vec(),
    })
}

pub fn set_feature(id: &str, value: &str) -> Result<(), String> {
    let v: i64 = value.parse().map_err(|_| "geçersiz değer")?;
    with(|s| {
        if !s.values.contains_key(id) {
            return Err("bu ayar bu cihazda yok".to_string());
        }
        if v == 1 && id == "conservation" {
            s.values.insert("rapid_charge", 0);
        }
        if v == 1 && id == "rapid_charge" {
            s.values.insert("conservation", 0);
        }
        let key = DEFS.iter().find(|d| d.id == id).unwrap().id;
        s.values.insert(key, v);
        Ok(())
    })
}

pub fn set_profile(value: &str) -> Result<(), String> {
    with(|s| s.profile = value.into());
    Ok(())
}

pub fn sensors() -> Sensors {
    let t = t();
    let k = load();
    let wave = |f: f32, p: f32| (t * f + p).sin();
    let fan_max = with(|s| s.values.get("fan_max") == Some(&1));
    let fan = |base: f32, p: f32| {
        if fan_max { 4900 } else { (base * k + 260.0 * wave(0.35, p)).max(0.0) as u32 }
    };
    Sensors {
        cpu_temp: Some(46.0 * k.powf(0.45) + 6.0 * wave(0.25, 0.0) + 2.0 * wave(1.3, 1.0)),
        cpu_usage: Some((22.0 * k + 12.0 * wave(0.6, 0.4) + 6.0 * wave(2.1, 0.0)).clamp(1.0, 100.0)),
        cpu_freq_mhz: Some((2600.0 * k.powf(0.3) + 400.0 * wave(0.8, 2.0)) as u32),
        mem_used_gb: Some(6.8 + 0.4 * wave(0.05, 0.0)),
        mem_total_gb: Some(15.5),
        ic_temp: Some(41.0 + 2.0 * wave(0.1, 3.0)),
        fans: vec![
            Fan { label: "CPU fanı".into(), rpm: fan(2300.0, 0.0), max: 5000 },
            Fan { label: "GPU fanı".into(), rpm: fan(2100.0, 1.7), max: 5000 },
        ],
        fans_unreliable: false,
        gpu: Gpu {
            present: true,
            name: Some("NVIDIA GeForce GTX 1650 Ti".into()),
            state: Some(if k > 1.1 { "active" } else { "suspended" }.into()),
            temp: Some(42.0 * k.powf(0.5) + 3.0 * wave(0.2, 0.7)),
            util: (k > 1.1).then(|| 35.0 + 20.0 * wave(0.9, 0.0)),
            power: (k > 1.1).then(|| 28.0 + 8.0 * wave(0.7, 0.3)),
        },
        battery: Battery {
            present: true,
            percent: Some(60),
            status: Some("Not charging".into()),
            ac_online: true,
            power: Some(14.2 * k + 2.0 * wave(0.4, 0.0)),
            health: Some(87.4),
            cycles: Some(312),
            design_wh: Some(80.0),
            full_wh: Some(69.9),
        },
    }
}

pub fn system() -> SystemInfo {
    SystemInfo {
        model: Some("Legion 5 15IMH05H".into()),
        machine_type: Some("82AU".into()),
        vendor: Some("LENOVO".into()),
        bios: Some("EFCN54WW".into()),
        kernel: Some("7.3.0-cachyos".into()),
        os: Some("CachyOS".into()),
        cpu: Some("Intel(R) Core(TM) i7-10750H CPU @ 2.60GHz".into()),
        legion_driver: true,
        ideapad_driver: true,
        demo: true,
    }
}
