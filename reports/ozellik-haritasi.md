# Özellik haritası — Lenovo Vantage → Linux (2026-10-27)

Kaynaklar:
- LenovoLegionLinux (johnfanv2) — çekirdek modülü `legion_laptop` + PyQt6 GUI + `legiond`. https://github.com/johnfanv2/LenovoLegionLinux
  - Dosya yolları `python/legion_linux/legion_linux/legion.py` içinden alındı.
- Lenovo Legion Toolkit (Windows, arşivlendi 2025-07-24) — Vantage'ın özellik listesi için referans. https://github.com/BartoszCichecki/LenovoLegionToolkit
- L5P-Keyboard-RGB (4JX) — 4 bölgeli RGB klavye, Rust, GPL-3.0, USB vid `048d`. https://github.com/4JX/L5P-Keyboard-RGB
- Çekirdek: `ideapad_laptop` (ana çekirdekte), `lenovo-wmi-gamezone` / `lenovo-wmi-other` (yeni çekirdekler; platform profili + firmware-attributes).

`$LEGION` = `/sys/module/legion_laptop/drivers/platform:legion/legion` (çekirdek ≥ 7.0) veya `.../PNP0C09:00` (eski).
`$IDEAPAD` = `/sys/bus/platform/drivers/ideapad_acpi/VPC2004:00`.

| Vantage özelliği | Linux karşılığı | Sürücü | Nabız v0.1 |
|---|---|---|---|
| Güç modu (Sessiz/Dengeli/Performans/Özel), Fn+Q | `/sys/firmware/acpi/platform_profile` (+ `_choices`) | ideapad / legion / wmi-gamezone | ✅ |
| Pil koruma (≈%60'ta tut) | `$IDEAPAD/conservation_mode` | ideapad | ✅ |
| Hızlı şarj | `$LEGION/rapidcharge` | legion | ✅ (korumayla karşılıklı dışlamalı) |
| Her zaman açık USB şarjı | `$IDEAPAD/usb_charging` | ideapad | ✅ |
| Hibrit mod / sadece dGPU (G-Sync) | `$LEGION/gsync` (1 = hibrit açık; 15ARH05/EUCN üzerinde doğrulandı) | legion | ✅ (yeniden başlatma gerekir) |
| Ekran OverDrive | `$LEGION/overdrive` | legion | ✅ |
| Fn kilidi | `$IDEAPAD/fn_lock` | ideapad | ✅ |
| Windows tuşu kilidi | `$LEGION/winkey` | legion | ✅ |
| Touchpad aç/kapa | `$IDEAPAD/touchpad` veya `$LEGION/touchpad` | ideapad/legion | ✅ |
| Kamera gizlilik (güç) | `$IDEAPAD/camera_power` | ideapad | ✅ |
| Klavye ışığı (beyaz) | `/sys/class/leds/*kbd_backlight/brightness` | ideapad | ✅ |
| Y logo / arka port ışığı | `/sys/class/leds/platform::ylogo`, `platform::ioport` | legion | ✅ |
| Fanı tam hız (toz temizleme) | `$LEGION/fan_fullspeed` | legion | ✅ |
| Prize takınca açıl (Instant boot) | `$LEGION/instant_boot_ac`, `instant_boot_usb_pd` | legion | ✅ |
| Sıcaklık + fan devri | hwmon: `coretemp`/`k10temp`, `legion_hwmon` (temp1 CPU, temp2 GPU, fan1/2) | — | ✅ |
| dGPU uyuyor mu | `/sys/bus/pci/devices/<nvidia>/power/runtime_status` | — | ✅ (uyurken nvidia-smi çağrılmaz) |
| Pil sağlığı, döngü, güç çekişi | `/sys/class/power_supply/BAT*` | — | ✅ |
| Fan eğrisi düzenleme | `$LEGION/hwmon/hwmon*/pwmN_auto_pointM_*` | legion | ⏳ v0.2 |
| CPU/GPU güç limitleri, GPU OC | `$LEGION/cpu_*_powerlimit`, `gpu_*`, `cpu_oc`, `gpu_oc` | legion / wmi-other | ⏳ v0.3 (riskli) |
| 4 bölge RGB klavye | USB HID (L5P-Keyboard-RGB protokolü) | — | ⏳ v0.3 |
| Açılış logosu | EFI değişkenleri `LBLDVC-…`, `LBLDESP-…` | — | ⏳ belki |
| Sürücü/BIOS güncelleme, garanti | Lenovo sunucuları | — | ✖ (fwupd'ye yönlendir) |
| Otomasyon (prize takınca mod değiştir) | `legiond` mantığı | — | ⏳ v0.2 |

Uyarılar (LLL README): bazı modellerde fan tablosu yazmak ani kapanmaya yol açabiliyor; fan eğrisi güç modu değişiminde/uykuda sıfırlanabiliyor. Bu yüzden fan eğrisi v0.1'de yalnız okunur.
