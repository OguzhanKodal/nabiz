//! `pkexec nabiz helper ...` ile root olarak çalışan küçük giriş noktası.
//! Yalnız hw.rs beyaz listesindeki kimlikleri kabul eder; ham yol almaz.

use crate::{hw, keyboard};

pub fn run(args: &[String]) -> i32 {
    let res = match args {
        [cmd, id, value] if cmd == "set" => hw::apply_feature(id, value),
        [cmd, value] if cmd == "profile" => hw::apply_profile(value),
        [cmd, effect, speed, brightness, c1, c2, c3, c4] if cmd == "kbd" => {
            match (speed.parse(), brightness.parse()) {
                (Ok(speed), Ok(brightness)) => keyboard::apply(&keyboard::KbdState {
                    effect: effect.clone(),
                    speed,
                    brightness,
                    colors: vec![c1.clone(), c2.clone(), c3.clone(), c4.clone()],
                }),
                _ => Err(std::io::Error::other("geçersiz hız/parlaklık")),
            }
        }
        _ => {
            eprintln!("kullanım: nabiz helper set <ayar> <değer> | profile <mod> | kbd <efekt> <hız> <parlaklık> <renk×4>");
            return 2;
        }
    };
    match res {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("{e}");
            1
        }
    }
}
