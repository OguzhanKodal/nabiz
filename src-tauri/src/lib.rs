mod demo;
pub mod helper;
mod hw;
mod keyboard;
mod sensors;
mod winkey;

use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Snapshot {
    sensors: sensors::Sensors,
    features: Vec<hw::FeatureState>,
    profile: hw::ProfileState,
}

#[tauri::command]
fn snapshot() -> Snapshot {
    if demo::enabled() {
        return Snapshot { sensors: demo::sensors(), features: demo::features(), profile: demo::profile() };
    }
    Snapshot { sensors: sensors::read(), features: hw::features(), profile: hw::profile() }
}

#[tauri::command]
fn system_info() -> sensors::SystemInfo {
    if demo::enabled() {
        return demo::system();
    }
    sensors::system()
}

#[tauri::command]
async fn set_feature(id: String, value: String) -> Result<(), String> {
    if demo::enabled() {
        return demo::set_feature(&id, &value);
    }
    hw::validate(&id, &value)?;
    hw::write_with_elevation(|| hw::apply_feature(&id, &value), &["set", &id, &value])
}

#[tauri::command]
async fn set_profile(value: String) -> Result<(), String> {
    if demo::enabled() {
        return demo::set_profile(&value);
    }
    let res = hw::write_with_elevation(|| hw::apply_profile(&value), &["profile", &value]);
    eprintln!("nabiz: güç modu {value} → {res:?} (şimdi: {:?})", hw::profile().current);
    res
}

#[tauri::command]
fn get_keyboard() -> keyboard::KbdInfo {
    keyboard::KbdInfo {
        available: demo::enabled() || keyboard::device().is_some(),
        state: keyboard::saved(),
    }
}

#[tauri::command]
async fn set_keyboard(state: keyboard::KbdState) -> Result<(), String> {
    keyboard::validate(&state)?;
    if !demo::enabled() {
        let (sp, br) = (state.speed.to_string(), state.brightness.to_string());
        let mut args = vec!["kbd", state.effect.as_str(), sp.as_str(), br.as_str()];
        args.extend(state.colors.iter().map(String::as_str));
        hw::write_with_elevation(|| keyboard::apply(&state), &args)?;
    }
    keyboard::save(&state);
    Ok(())
}

/// `nabiz dump`: arayüzün gördüğü her şeyi JSON olarak basar (teşhis için).
pub fn dump() {
    let _ = snapshot(); // CPU yükü iki ölçüm arası farktan hesaplanır
    std::thread::sleep(std::time::Duration::from_millis(500));
    let out = serde_json::json!({ "system": system_info(), "snapshot": snapshot() });
    println!("{}", serde_json::to_string_pretty(&out).unwrap());
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![snapshot, system_info, set_feature, set_profile, get_keyboard, set_keyboard])
        .run(tauri::generate_context!())
        .expect("Nabız başlatılamadı");
}
