// Windows'ta konsol penceresi açılmasın
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let args: Vec<String> = std::env::args().collect();
    // pkexec ile root olarak çağrıldığında arayüz açılmaz, yalnız ayarı yazar.
    if args.get(1).map(String::as_str) == Some("helper") {
        std::process::exit(nabiz_lib::helper::run(&args[2..]));
    }
    if args.get(1).map(String::as_str) == Some("dump") {
        return nabiz_lib::dump();
    }
    nabiz_lib::run()
}
