// Rust demo.rs'in tarayıcı karşılığı; yalnız `npm run dev` ile tasarım yaparken kullanılır.
import type { FeatureState, KbdState, Snapshot, SystemInfo } from "./types";

// `?demo=generic`: Lenovo olmayan sıradan bir laptop (yalnız markadan bağımsız özellikler).
const generic = new URLSearchParams(location.search).get("demo") === "generic";

const values: Record<string, number> = generic
  ? { win_key: 1, touchpad: 1, camera: 1 }
  : {
      conservation: 1, rapid_charge: 0, usb_charging: 0, gsync: 1, overdrive: 1,
      fn_lock: 0, win_key: 1, touchpad: 1, camera: 1, kbd_backlight: 1,
      logo_led: 1, ioport_led: 0, fan_max: 0, instant_boot_ac: 0,
    };
const ids = [
  "conservation", "rapid_charge", "usb_charging", "gsync", "overdrive", "fn_lock", "win_key",
  "touchpad", "camera", "kbd_backlight", "logo_led", "ioport_led", "fan_max",
  "instant_boot_ac", "instant_boot_usb_pd",
];
let profile = "balanced";
let kbd: KbdState = { effect: "static", speed: 2, brightness: 2, colors: ["5b9dff", "3ee0b0", "ffb547", "ff5d4f"] };

function snapshot(): Snapshot {
  const t = performance.now() / 1000;
  const k = profile === "quiet" ? 0.55 : profile === "performance" ? 1.35 : 1;
  const w = (f: number, p: number) => Math.sin(t * f + p);
  const fan = (b: number, p: number) => (values.fan_max ? 4900 : Math.max(0, Math.round(b * k + 260 * w(0.35, p))));
  const features: FeatureState[] = ids.map((id) => ({
    id,
    available: id in values,
    value: id in values ? values[id] : null,
    max: id in values ? (id === "kbd_backlight" ? 2 : 1) : null,
  }));
  return {
    sensors: {
      cpuTemp: 46 * k ** 0.45 + 6 * w(0.25, 0) + 2 * w(1.3, 1),
      cpuUsage: Math.min(100, Math.max(1, 22 * k + 12 * w(0.6, 0.4) + 6 * w(2.1, 0))),
      cpuFreqMhz: Math.round(2600 * k ** 0.3 + 400 * w(0.8, 2)),
      memUsedGb: 6.8 + 0.4 * w(0.05, 0),
      memTotalGb: 15.5,
      icTemp: 41 + 2 * w(0.1, 3),
      fans: generic
        ? []
        : [
            { label: "CPU fanı", rpm: fan(2300, 0), max: 5000 },
            { label: "GPU fanı", rpm: fan(2100, 1.7), max: 5000 },
          ],
      fansUnreliable: false,
      gpu: {
        present: !generic,
        name: "NVIDIA GeForce GTX 1650 Ti",
        state: k > 1.1 ? "active" : "suspended",
        temp: generic ? null : 42 * k ** 0.5 + 3 * w(0.2, 0.7),
        util: k > 1.1 ? 35 + 20 * w(0.9, 0) : null,
        power: k > 1.1 ? 28 + 8 * w(0.7, 0.3) : null,
      },
      battery: {
        present: true, percent: 60, status: "Not charging", acOnline: true,
        power: 14.2 * k + 2 * w(0.4, 0), health: 87.4, cycles: 312, designWh: 80, fullWh: 69.9,
      },
    },
    features,
    profile: { current: profile, choices: ["quiet", "balanced", "performance"] },
  };
}

const system: SystemInfo = generic
  ? {
      model: null, machineType: "HP Pavilion Laptop 15-eg0xxx", vendor: "HP", bios: "F.25",
      kernel: "7.3.0-cachyos", os: "CachyOS", cpu: "11th Gen Intel(R) Core(TM) i5-1135G7 @ 2.40GHz",
      legionDriver: false, ideapadDriver: false, demo: true,
    }
  : {
      model: "Legion 5 15IMH05H", machineType: "82AU", vendor: "LENOVO", bios: "EFCN54WW",
      kernel: "7.3.0-cachyos", os: "CachyOS", cpu: "Intel(R) Core(TM) i7-10750H CPU @ 2.60GHz",
      legionDriver: true, ideapadDriver: true, demo: true,
    };

export async function handle(cmd: string, args?: Record<string, unknown>): Promise<unknown> {
  await new Promise((r) => setTimeout(r, 60));
  switch (cmd) {
    case "snapshot":
      return snapshot();
    case "system_info":
      return system;
    case "set_feature": {
      const id = String(args!.id);
      const v = Number(args!.value);
      if (!(id in values)) throw "bu ayar bu cihazda yok";
      if (v === 1 && id === "conservation") values.rapid_charge = 0;
      if (v === 1 && id === "rapid_charge") values.conservation = 0;
      values[id] = v;
      return;
    }
    case "set_profile":
      profile = String(args!.value);
      return;
    case "get_keyboard":
      return { available: !generic, state: kbd };
    case "set_keyboard":
      kbd = args!.state as KbdState;
      return;
  }
  throw `bilinmeyen komut: ${cmd}`;
}
