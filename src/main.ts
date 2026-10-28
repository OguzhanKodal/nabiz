import "@fontsource-variable/inter";
import "./style.css";
import { api } from "./api";
import { icon } from "./icons";
import { fanLabel, lang, setLang, t, trError, type Lang } from "./i18n";
import appIcon from "../packaging/icon.svg";
import type { FeatureState, KbdInfo, KbdState, Snapshot, SystemInfo } from "./types";

// ---------- Meta ----------

type ViewId = "overview" | "performance" | "battery" | "display" | "input" | "system";

const VIEWS: { id: ViewId; icon: string }[] = [
  { id: "overview", icon: icon.home },
  { id: "performance", icon: icon.gauge },
  { id: "battery", icon: icon.battery },
  { id: "display", icon: icon.monitor },
  { id: "input", icon: icon.keyboard },
  { id: "system", icon: icon.info },
];

type ProfileKey = "quiet" | "balanced" | "performance" | "custom";
const PROFILES: Record<string, { key: ProfileKey; icon: string; color: string }> = {
  // Legion'larda firmware'in "Quiet" modu low-power olarak görünür.
  "low-power": { key: "quiet", icon: icon.leaf, color: "#5b9dff" },
  quiet: { key: "quiet", icon: icon.leaf, color: "#5b9dff" },
  balanced: { key: "balanced", icon: icon.scale, color: "#3ee0b0" },
  "balanced-performance": { key: "custom", icon: icon.sparkles, color: "#b58cff" },
  performance: { key: "performance", icon: icon.flame, color: "#ff5d4f" },
  custom: { key: "custom", icon: icon.sparkles, color: "#ffb547" },
};
const profLabel = (k: ProfileKey) => t(`profile.${k}`);
const profDesc = (k: ProfileKey) => t(`profile.${k}.desc`);

const FEATURE_ICONS: Record<string, string> = {
  conservation: icon.shield,
  rapid_charge: icon.bolt,
  usb_charging: icon.usb,
  gsync: icon.gpu,
  overdrive: icon.monitorBolt,
  fn_lock: icon.lock,
  win_key: icon.windows,
  touchpad: icon.touchpad,
  camera: icon.camera,
  kbd_backlight: icon.sun,
  logo_led: icon.logo,
  ioport_led: icon.ports,
  fan_max: icon.fan,
  instant_boot_ac: icon.plug,
  instant_boot_usb_pd: icon.plug,
};
type FeatureId =
  | "conservation" | "rapid_charge" | "usb_charging" | "gsync" | "overdrive" | "fn_lock" | "win_key" | "touchpad"
  | "camera" | "kbd_backlight" | "logo_led" | "ioport_led" | "fan_max" | "instant_boot_ac" | "instant_boot_usb_pd";
const fmeta = (id: string) => ({
  label: t(`f.${id as FeatureId}`),
  desc: t(`f.${id as FeatureId}.desc`),
  icon: FEATURE_ICONS[id],
});

const QUICK = ["conservation", "fn_lock", "camera", "touchpad", "overdrive", "win_key"];

// ---------- Durum ----------

let snap: Snapshot | null = null;
let sys: SystemInfo | null = null;
let kbd: KbdInfo | null = null;
let view: ViewId = loadView();
let structureKey = "";
const busy = new Set<string>();
const HISTORY = 80;
const hist = { cpu: [] as number[], gpu: [] as number[] };

function loadView(): ViewId {
  const h = location.hash.slice(1) as ViewId;
  if (VIEWS.some((x) => x.id === h)) return h;
  try {
    const v = localStorage.getItem("nabiz.view") as ViewId | null;
    if (v && VIEWS.some((x) => x.id === v)) return v;
  } catch {}
  return "overview";
}

const feat = (id: string): FeatureState | undefined => snap?.features.find((f) => f.id === id);
const on = (id: string) => feat(id)?.value === 1;
const anyAvail = (ids: string[]) => ids.some((id) => feat(id)?.available);

// Lenovo'ya özgü ipuçları ve sürücüler yalnız Lenovo cihazlarda gösterilir.
const isLenovo = () => !!sys?.vendor?.toUpperCase().includes("LENOVO");
const isLegionLike = () => isLenovo() && /legion|loq|gaming/i.test(sys?.model ?? "");
// Lenovo pazarlama adını product_version'da tutar; diğer üreticiler product_name'de.
const modelName = () => (isLenovo() ? sys?.model ?? sys?.machineType : sys?.machineType ?? sys?.model) ?? null;

// ---------- Biçim ----------

const n = (v: number | null | undefined, d = 0) => (v == null || Number.isNaN(v) ? "—" : v.toFixed(d));
const esc = (s: string) => s.replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]!);

function tempColor(t: number | null): string {
  if (t == null) return "var(--muted)";
  if (t < 60) return "var(--ok)";
  if (t < 80) return "var(--warn)";
  return "var(--hot)";
}

function batteryStatus(s: Snapshot): string {
  const b = s.sensors.battery;
  if (!b.present) return t("batt.none");
  switch (b.status) {
    case "Charging": return t("batt.charging");
    case "Discharging": return t("batt.discharging");
    case "Full": return t("batt.full");
    case "Not charging": return on("conservation") ? t("batt.held") : t("batt.notCharging");
    default: return b.acOnline ? t("batt.plugged") : t("batt.discharging");
  }
}

function gpuState(s: Snapshot): string {
  const g = s.sensors.gpu;
  if (!g.present) return t("gpu.none");
  if (g.state === "active") return t("gpu.active");
  if (g.state === "suspended") return t("gpu.suspended");
  return g.state ?? t("gpu.unknown");
}

// ---------- Parçalar ----------

function gauge(key: string, label: string, ic: string): string {
  const r = 52, c = 2 * Math.PI * r, arc = c * 0.75;
  return `
  <div class="gauge" data-gauge="${key}">
    <svg viewBox="0 0 128 128">
      <circle class="g-track" cx="64" cy="64" r="${r}" stroke-dasharray="${arc} ${c}" transform="rotate(135 64 64)"/>
      <circle class="g-fill" cx="64" cy="64" r="${r}" stroke-dasharray="0 ${c}" data-arc="${arc}" data-c="${c}" transform="rotate(135 64 64)"/>
    </svg>
    <div class="g-center">
      <span class="g-ic">${ic}</span>
      <span class="g-val"><b data-bind="${key}Temp">—</b><small>°C</small></span>
      <span class="g-label">${label}</span>
    </div>
  </div>`;
}

function toggleRow(id: string, opts: { note?: string } = {}): string {
  const m = fmeta(id);
  const f = feat(id);
  const avail = !!f?.available;
  return `
  <div class="row ${avail ? "" : "is-off"}" data-row="${id}">
    <span class="row-ic">${m.icon}</span>
    <div class="row-text">
      <span class="row-title">${m.label}${avail ? "" : `<span class="tag">${t("common.unsupported")}</span>`}</span>
      <span class="row-desc">${m.desc}${opts.note ? ` <em>${opts.note}</em>` : ""}</span>
    </div>
    <label class="switch">
      <input type="checkbox" data-feature="${id}" ${avail ? "" : "disabled"} ${on(id) ? "checked" : ""}/>
      <span></span>
    </label>
  </div>`;
}

function levelRow(id: string): string {
  const m = fmeta(id);
  const f = feat(id);
  const max = f?.max ?? 0;
  const avail = !!f?.available && max > 0;
  const names = max === 2 ? [t("common.off"), t("common.low"), t("common.high")] : null;
  const opts = avail
    ? Array.from({ length: max + 1 }, (_, i) => `<button data-level="${id}" data-v="${i}">${names ? names[i] : i}</button>`).join("")
    : "";
  return `
  <div class="row ${avail ? "" : "is-off"}">
    <span class="row-ic">${m.icon}</span>
    <div class="row-text">
      <span class="row-title">${m.label}${avail ? "" : `<span class="tag">${t("common.unsupported")}</span>`}</span>
      <span class="row-desc">${m.desc}</span>
    </div>
    <div class="seg seg-sm">${opts}</div>
  </div>`;
}

function profileSwitcher(big = false): string {
  const choices = snap?.profile.choices ?? [];
  if (!choices.length) {
    return `<div class="empty">${icon.alert}<span>${t("profile.unsupported")}</span></div>`;
  }
  return `<div class="modes ${big ? "modes-big" : ""}">${choices
    .map((c) => {
      const p = PROFILES[c];
      const m = p ? { label: profLabel(p.key), desc: profDesc(p.key), icon: p.icon, color: p.color } : { label: c, desc: "", icon: icon.gauge, color: "#9aa4b2" };
      return `<button class="mode" data-profile="${c}" style="--mc:${m.color}">
        <span class="mode-ic">${m.icon}</span>
        <span class="mode-label">${m.label}</span>
        <span class="mode-desc">${m.desc}</span>
      </button>`;
    })
    .join("")}</div>`;
}

function fansCard(): string {
  const fans = snap?.sensors.fans ?? [];
  const body = fans.length
    ? fans
        .map(
          (f, i) => `
      <div class="fan">
        <span class="fan-ic" data-spin="${i}">${icon.fan}</span>
        <div class="fan-body">
          <div class="fan-top"><span>${esc(fanLabel(f.label))}</span><b><span data-fan-rpm="${i}">—</span> <small>RPM</small></b></div>
          <div class="bar"><i data-fan-bar="${i}"></i></div>
        </div>
      </div>`,
        )
        .join("")
    : snap?.sensors.fansUnreliable
      ? `<div class="empty small">${icon.alert}<span>${t("fan.unreliable")}</span></div>`
      : `<div class="empty small">${icon.alert}<span>${t(isLenovo() ? "fan.unreadable" : "fan.unreadableGeneric")}</span></div>`;
  return `<div class="card"><div class="card-h"><span>${t("fan.title")}</span></div>${body}</div>`;
}

function batteryVisual(): string {
  return `
  <div class="batt">
    <div class="batt-shell"><div class="batt-fill" data-batt-fill></div><span class="batt-bolt" data-batt-bolt>${icon.bolt}</span></div>
    <div class="batt-cap"></div>
  </div>`;
}

function stat(label: string, bind: string, unit = ""): string {
  return `<div class="stat"><span class="stat-l">${label}</span><b><span data-bind="${bind}">—</span>${unit ? `<small>${unit}</small>` : ""}</b></div>`;
}

function chartCard(): string {
  return `
  <div class="card span-2">
    <div class="card-h"><span>${t("card.history")}</span>
      <span class="legend"><i style="--c:var(--accent)"></i>CPU <i style="--c:#8fa3ff"></i>GPU</span>
    </div>
    <svg class="chart" viewBox="0 0 400 120" preserveAspectRatio="none">
      <defs>
        <linearGradient id="gCpu" x1="0" x2="0" y1="0" y2="1"><stop offset="0" stop-color="var(--accent)" stop-opacity=".35"/><stop offset="1" stop-color="var(--accent)" stop-opacity="0"/></linearGradient>
      </defs>
      <g class="grid">${[30, 60, 90].map((y) => `<line x1="0" x2="400" y1="${y}" y2="${y}"/>`).join("")}</g>
      <path data-chart="cpuArea" fill="url(#gCpu)"/>
      <path data-chart="cpu" class="line" stroke="var(--accent)"/>
      <path data-chart="gpu" class="line" stroke="#8fa3ff"/>
    </svg>
    <div class="chart-scale"><span>100°</span><span>50°</span><span>0°</span></div>
  </div>`;
}

// ---------- Ekranlar ----------

function viewOverview(): string {
  return `
  <div class="card span-3 hero">
    <div class="card-h"><span>${t("card.power")}</span>${isLenovo() ? `<span class="hint">${t("card.power.hint")}</span>` : ""}</div>
    ${profileSwitcher()}
  </div>
  <div class="card gauge-card">
    ${gauge("cpu", t("stat.cpu"), icon.chip)}
    <div class="mini-stats">${stat(t("stat.load"), "cpuUsage", "%")}${stat(t("stat.freq"), "cpuFreq", "GHz")}</div>
  </div>
  <div class="card gauge-card">
    ${gauge("gpu", t("stat.gpu"), icon.gpu)}
    <div class="mini-stats">${stat(t("stat.state"), "gpuStateShort")}${stat(t("stat.power"), "gpuPower", "W")}</div>
  </div>
  <div class="card batt-card">
    <div class="card-h"><span>${t("card.battery")}</span><span class="pill" data-bind="acState">—</span></div>
    <div class="batt-row">${batteryVisual()}<div><b class="big"><span data-bind="battPct">—</span><small>%</small></b><span class="muted" data-bind="battStatus">—</span></div></div>
    <div class="mini-stats">${stat(t("stat.power"), "battPower", "W")}${stat(t("stat.health"), "battHealth", "%")}</div>
  </div>
  ${chartCard()}
  ${fansCard()}
  ${
    anyAvail(QUICK)
      ? `<div class="card span-3">
    <div class="card-h"><span>${t("card.quick")}</span></div>
    <div class="quick">${QUICK.filter((id) => feat(id)?.available).map((id) => quickTile(id)).join("")}</div>
  </div>`
      : ""
  }`;
}

function quickTile(id: string): string {
  const m = fmeta(id);
  const avail = !!feat(id)?.available;
  return `<button class="tile" data-tile="${id}" ${avail ? "" : "disabled"}>
    <span class="tile-ic">${m.icon}</span><span class="tile-label">${m.label}</span>
    <span class="tile-state" data-tile-state="${id}">${avail ? "" : t("common.none")}</span>
  </button>`;
}

function viewPerformance(): string {
  return `
  <div class="card span-3 hero">
    <div class="card-h"><span>${t("card.power")}</span>${isLenovo() ? `<span class="hint">${t("card.power.hint")}</span>` : ""}</div>
    ${profileSwitcher(true)}
  </div>
  <div class="card span-2">
    <div class="card-h"><span>${t("card.load")}</span></div>
    <div class="stats-grid">
      ${stat(t("stat.cpuLoad"), "cpuUsage", "%")}${stat(t("stat.avgFreq"), "cpuFreq", "GHz")}
      ${stat(t("stat.cpuTemp"), "cpuTemp", "°C")}${stat(t("stat.mem"), "mem", "GB")}
      ${stat(t("stat.gpuTemp"), "gpuTemp", "°C")}${stat(t("stat.ec"), "icTemp", "°C")}
    </div>
  </div>
  ${fansCard()}
  ${
    anyAvail(["fan_max", "instant_boot_ac", "instant_boot_usb_pd"])
      ? `<div class="card span-3">
    <div class="card-h"><span>${t("card.fanBoot")}</span></div>
    ${toggleRow("fan_max")}${toggleRow("instant_boot_ac")}${toggleRow("instant_boot_usb_pd")}
  </div>`
      : ""
  }`;
}

function viewBattery(): string {
  const charge = feat("conservation")?.available || feat("rapid_charge")?.available;
  return `
  <div class="card span-3 batt-hero">
    <div class="batt-hero-left">${batteryVisual()}
      <div><b class="huge"><span data-bind="battPct">—</span><small>%</small></b>
      <span class="muted" data-bind="battStatus">—</span></div>
    </div>
    <div class="stats-grid">
      ${stat(t("stat.nowPower"), "battPower", "W")}${stat(t("stat.battHealth"), "battHealth", "%")}
      ${stat(t("stat.cycles"), "battCycles")}${stat(t("stat.capacity"), "battCap", "Wh")}
    </div>
  </div>
  <div class="card span-3">
    <div class="card-h"><span>${t("card.charge")}</span></div>
    ${
      charge
        ? `<div class="seg seg-wide">
        <button data-charge="normal">${icon.battery}<span>${t("charge.normal")}<small>${t("charge.normal.sub")}</small></span></button>
        <button data-charge="conservation" ${feat("conservation")?.available ? "" : "disabled"}>${icon.shield}<span>${t("charge.conservation")}<small>${t("charge.conservation.sub")}</small></span></button>
        <button data-charge="rapid" ${feat("rapid_charge")?.available ? "" : "disabled"}>${icon.bolt}<span>${t("charge.rapid")}<small>${t("charge.rapid.sub")}</small></span></button>
      </div>
      <p class="note">${t("charge.tip")}</p>`
        : `<div class="empty">${icon.alert}<span>${t(isLenovo() ? "charge.missing" : "charge.missingGeneric")}</span></div>`
    }
  </div>
  <div class="card span-3">${toggleRow("usb_charging")}</div>`;
}

function viewDisplay(): string {
  return `
  <div class="card span-3 gpu-hero">
    <span class="gpu-hero-ic">${icon.gpu}</span>
    <div><span class="muted">${t("display.dgpu")}</span><b class="big" data-bind="gpuName">—</b><span class="pill" data-bind="gpuState">—</span></div>
    <div class="stats-grid">${stat(t("stat.temp"), "gpuTemp", "°C")}${stat(t("stat.util"), "gpuUtil", "%")}${stat(t("stat.power"), "gpuPower", "W")}</div>
  </div>
  ${
    anyAvail(["gsync", "overdrive"])
      ? `<div class="card span-3">
    ${toggleRow("gsync", { note: t("f.gsync.note") })}
    ${toggleRow("overdrive")}
  </div>`
      : ""
  }
  <p class="note span-3">${t("display.note")}</p>`;
}

const KBD_EFFECTS: { id: KbdState["effect"]; label: () => string }[] = [
  { id: "static", label: () => t("fx.static") },
  { id: "breath", label: () => t("fx.breath") },
  { id: "smooth", label: () => t("fx.smooth") },
  { id: "wave-left", label: () => t("fx.waveLeft") },
  { id: "wave-right", label: () => t("fx.waveRight") },
  { id: "off", label: () => t("common.off") },
];

const KBD_PRESETS: string[][] = [
  ["3ee0b0", "3ee0b0", "3ee0b0", "3ee0b0"],
  ["5b9dff", "5b9dff", "5b9dff", "5b9dff"],
  ["ff5d4f", "ff5d4f", "ff5d4f", "ff5d4f"],
  ["b58cff", "b58cff", "b58cff", "b58cff"],
  ["ffffff", "ffffff", "ffffff", "ffffff"],
  ["ff5d4f", "ff8a3d", "ffb547", "ffd86b"],
  ["1e6bff", "2f9dff", "3ec8e0", "3ee0b0"],
  ["ff4d6d", "ffb547", "3ee0b0", "5b9dff"],
];

function kbdCard(): string {
  if (!kbd?.available) {
    if (!isLegionLike()) return "";
    return `<div class="card span-3">${`<div class="empty">${icon.alert}<span>${t("kbd.missing")}</span></div>`}</div>`;
  }
  const zones = [0, 1, 2, 3]
    .map(
      (z) => `<label class="zone" data-zone-box="${z}" title="${t("kbd.zone", { n: z + 1 })}">
        <input type="color" data-zone="${z}"/>
        <span class="keys">${"<i></i>".repeat(z === 3 ? 16 : 28)}</span>
        <span class="zone-n">${z + 1}</span>
      </label>`,
    )
    .join("");
  return `
  <div class="card span-3 kbd-card">
    <div class="card-h"><span>${t("kbd.title")}</span><span class="hint">${t("kbd.hint")}</span></div>
    <div class="kbd" data-kbd>${zones}</div>
    <div class="kbd-controls">
      <div class="seg seg-fx">${KBD_EFFECTS.map((e) => `<button data-fx="${e.id}">${e.label()}</button>`).join("")}</div>
      <div class="presets">${KBD_PRESETS.map(
        (p, i) => `<button class="swatch" data-preset="${i}" title="${t(`preset.${i}` as "preset.0")}" style="background:linear-gradient(90deg,${p.map((c, j) => `#${c} ${j * 25}% ${(j + 1) * 25}%`).join(",")})"></button>`,
      ).join("")}</div>
      <div class="kbd-row">
        <span class="muted">${t("kbd.brightness")}</span>
        <div class="seg seg-sm"><button data-kbright="1">${t("common.low")}</button><button data-kbright="2">${t("common.high")}</button></div>
        <span class="muted" data-speed-label>${t("kbd.speed")}</span>
        <div class="seg seg-sm" data-speed-seg>${[1, 2, 3, 4].map((s) => `<button data-kspeed="${s}">${s}</button>`).join("")}</div>
      </div>
    </div>
  </div>`;
}

function updateKbd() {
  if (!kbd?.available) return;
  const s = kbd.state;
  const box = document.querySelector<HTMLElement>("[data-kbd]");
  if (!box) return;
  box.dataset.fx = s.effect;
  box.style.setProperty("--kb-dim", s.brightness === 1 ? "0.55" : "1");
  box.style.setProperty("--kb-speed", `${[0, 4, 2.6, 1.6, 0.9][s.speed]}s`);
  s.colors.forEach((c, i) => {
    const z = document.querySelector<HTMLElement>(`[data-zone-box="${i}"]`);
    z?.style.setProperty("--zc", `#${c}`);
    const inp = document.querySelector<HTMLInputElement>(`[data-zone="${i}"]`);
    if (inp && document.activeElement !== inp) inp.value = `#${c}`;
  });
  document.querySelectorAll<HTMLElement>("[data-fx]").forEach((b) => b.classList.toggle("active", b.dataset.fx === s.effect));
  document.querySelectorAll<HTMLElement>("[data-kbright]").forEach((b) => b.classList.toggle("active", Number(b.dataset.kbright) === s.brightness));
  document.querySelectorAll<HTMLElement>("[data-kspeed]").forEach((b) => b.classList.toggle("active", Number(b.dataset.kspeed) === s.speed));
  const animated = s.effect !== "static" && s.effect !== "off";
  document.querySelector("[data-speed-seg]")?.classList.toggle("is-hidden", !animated);
  document.querySelector("[data-speed-label]")?.classList.toggle("is-hidden", !animated);
}

let kbdTimer = 0;
// Renk seçerken klavyeye saniyede en fazla ~7 komut gider.
function pushKbd(now = false) {
  updateKbd();
  clearTimeout(kbdTimer);
  kbdTimer = window.setTimeout(
    async () => {
      try {
        await api.setKeyboard(kbd!.state);
      } catch (e) {
        toast(trError(e), "err");
      }
    },
    now ? 0 : 140,
  );
}

function viewInput(): string {
  return `
  ${kbdCard()}
  <div class="card span-3">
    <div class="card-h"><span>${t("card.keyboard")}</span></div>
    ${toggleRow("fn_lock")}${toggleRow("win_key")}${kbd?.available ? "" : levelRow("kbd_backlight")}
  </div>
  <div class="card span-3">
    <div class="card-h"><span>${t("card.privacy")}</span></div>
    ${toggleRow("touchpad")}${toggleRow("camera")}
  </div>
  ${
    anyAvail(["logo_led", "ioport_led"])
      ? `<div class="card span-3">
    <div class="card-h"><span>${t("card.lights")}</span></div>
    ${toggleRow("logo_led")}${toggleRow("ioport_led")}
  </div>`
      : ""
  }`;
}

const INSTALL_CMD =
  "sudo pacman -S --needed dkms git && git clone --depth 1 https://github.com/johnfanv2/LenovoLegionLinux.git && cd LenovoLegionLinux/kernel_module && sudo make dkms && echo 'options legion_laptop enable_platformprofile=0' | sudo tee /etc/modprobe.d/legion_laptop.conf && sudo modprobe legion_laptop";

function viewSystem(): string {
  const s = sys;
  const item = (k: string, v: string | null | undefined) => `<div class="kv"><span>${k}</span><b>${esc(v ?? "—")}</b></div>`;
  const drv = (name: string, okay: boolean, desc: string) => `
    <div class="row">
      <span class="row-ic ${okay ? "ok" : "bad"}">${okay ? icon.check : icon.alert}</span>
      <div class="row-text"><span class="row-title">${name}</span><span class="row-desc">${desc}</span></div>
      <span class="pill ${okay ? "pill-ok" : "pill-bad"}">${okay ? t("sys.installed") : t("common.none")}</span>
    </div>`;
  return `
  <div class="card span-3">
    <div class="card-h"><span>${t("card.device")}</span>${s?.demo ? `<span class="pill">${t("sys.demo")}</span>` : ""}</div>
    <div class="kv-grid">
      ${item(t("sys.model"), modelName())}${isLenovo() ? item(t("sys.machine"), s?.machineType) : item(t("sys.vendor"), s?.vendor)}
      ${item(t("sys.cpu"), s?.cpu)}${item("BIOS", s?.bios)}
      ${item(t("sys.os"), s?.os)}${item(t("sys.kernel"), s?.kernel)}
    </div>
  </div>
  ${isLenovo() ? `<div class="card span-3">
    <div class="card-h"><span>${t("card.drivers")}</span></div>
    ${drv("ideapad_laptop", !!s?.ideapadDriver, t("sys.ideapad"))}
    ${drv("legion_laptop", !!s?.legionDriver, t("sys.legion"))}
    ${
      isLegionLike() && !s?.legionDriver
        ? `<div class="install">
        <p>${t("sys.install")}</p>
        <div class="cmd"><code>${INSTALL_CMD}</code><button class="icon-btn" data-copy="${INSTALL_CMD}" title="${t("common.copy")}">${icon.copy}</button></div>
        <p class="muted">${t("sys.installNote")}</p>
      </div>`
        : ""
    }
  </div>` : ""}
  <p class="note span-3">${t("sys.disclaimer")}</p>`;
}

const RENDER: Record<ViewId, () => string> = {
  overview: viewOverview,
  performance: viewPerformance,
  battery: viewBattery,
  display: viewDisplay,
  input: viewInput,
  system: viewSystem,
};

// ---------- Kabuk ----------

function shell() {
  document.querySelector("#app")!.innerHTML = `
  <div class="app">
    <aside class="nav">
      <div class="brand"><img class="brand-ic" src="${appIcon}" alt="" /><div><b>Nabız</b><small data-bind="model">—</small></div></div>
      <nav>${VIEWS.map((v) => `<button class="nav-item" data-view="${v.id}">${v.icon}<span>${t(`view.${v.id}`)}</span></button>`).join("")}</nav>
      <div class="nav-foot">
        <div class="seg seg-sm seg-lang">${(["tr", "en"] as Lang[]).map((l) => `<button data-lang="${l}" class="${l === lang ? "active" : ""}">${l.toUpperCase()}</button>`).join("")}</div>
        <div class="now-mode"><span class="dot"></span><span data-bind="profileLabel">—</span></div>
      </div>
    </aside>
    <main>
      <header class="top"><div><h1 data-title></h1><p data-subtitle></p></div><span class="live"><i></i>${t("common.live")}</span></header>
      <section class="grid" id="view"></section>
    </main>
    <div class="toasts" id="toasts"></div>
  </div>`;
}

function renderView() {
  const v = VIEWS.find((x) => x.id === view)!;
  document.querySelector("[data-title]")!.textContent = t(`view.${v.id}`);
  document.querySelector("[data-subtitle]")!.textContent = t(`view.${v.id}.sub`);
  document.querySelectorAll<HTMLElement>("[data-view]").forEach((b) => b.classList.toggle("active", b.dataset.view === view));
  const el = document.getElementById("view")!;
  el.innerHTML = RENDER[view]();
  el.classList.remove("enter");
  void el.offsetWidth;
  el.classList.add("enter");
  update();
}

// Kullanılabilir özellik kümesi değişirse (ör. sürücü yüklendi) ekranı yeniden kur.
function keyOf(s: Snapshot) {
  return s.features.map((f) => `${f.id}:${f.available ? 1 : 0}:${f.max}`).join() + s.profile.choices.join() + s.sensors.fans.length + s.sensors.fansUnreliable;
}

// ---------- Canlı güncelleme ----------

function linePath(data: number[], close = false): string {
  if (data.length < 2) return "";
  const step = 400 / (HISTORY - 1);
  const off = (HISTORY - data.length) * step;
  const pts = data.map((v, i) => [off + i * step, 120 - Math.max(0, Math.min(100, v)) * 1.2]);
  let d = `M${pts[0][0]},${pts[0][1]}`;
  for (let i = 1; i < pts.length; i++) {
    const [x0, y0] = pts[i - 1], [x1, y1] = pts[i];
    const cx = (x0 + x1) / 2;
    d += ` C${cx},${y0} ${cx},${y1} ${x1},${y1}`;
  }
  if (close) d += ` L${pts[pts.length - 1][0]},120 L${pts[0][0]},120 Z`;
  return d;
}

function update() {
  if (!snap) return;
  const s = snap.sensors;
  const b = s.battery;
  const prof = snap.profile.current ?? "";
  const pm = PROFILES[prof];
  document.documentElement.style.setProperty("--accent", pm?.color ?? "#3ee0b0");

  const binds: Record<string, string> = {
    model: modelName() ?? t("common.laptop"),
    profileLabel: pm ? t("profile.mode", { label: profLabel(pm.key) }) : t("profile.none"),
    cpuTemp: n(s.cpuTemp),
    gpuTemp: n(s.gpu.temp),
    cpuUsage: n(s.cpuUsage),
    cpuFreq: s.cpuFreqMhz ? (s.cpuFreqMhz / 1000).toFixed(2) : "—",
    mem: s.memUsedGb != null && s.memTotalGb ? `${s.memUsedGb.toFixed(1)} / ${s.memTotalGb.toFixed(0)}` : "—",
    icTemp: n(s.icTemp),
    gpuName: s.gpu.name ?? (s.gpu.present ? "NVIDIA GPU" : t("gpu.notFound")),
    gpuState: gpuState(snap),
    gpuStateShort: !s.gpu.present ? t("gpu.short.none") : s.gpu.state === "active" ? t("gpu.short.active") : t("gpu.short.sleep"),
    gpuUtil: n(s.gpu.util),
    gpuPower: n(s.gpu.power, 1),
    battPct: n(b.percent),
    battStatus: batteryStatus(snap),
    battPower: n(b.power, 1),
    battHealth: n(b.health, 0),
    battCycles: b.cycles != null ? String(b.cycles) : "—",
    battCap: b.fullWh != null && b.designWh != null ? `${b.fullWh.toFixed(1)} / ${b.designWh.toFixed(0)}` : "—",
    acState: b.acOnline ? t("batt.ac") : t("batt.onBattery"),
  };
  document.querySelectorAll<HTMLElement>("[data-bind]").forEach((el) => {
    const v = binds[el.dataset.bind!];
    if (v !== undefined && el.textContent !== v) el.textContent = v;
  });

  // göstergeler
  const g: Record<string, number | null> = { cpu: s.cpuTemp, gpu: s.gpu.temp };
  document.querySelectorAll<HTMLElement>("[data-gauge]").forEach((el) => {
    const t = g[el.dataset.gauge!];
    const fill = el.querySelector<SVGCircleElement>(".g-fill")!;
    const arc = Number(fill.dataset.arc), c = Number(fill.dataset.c);
    const frac = t == null ? 0 : Math.max(0, Math.min(1, (t - 20) / 80));
    fill.setAttribute("stroke-dasharray", `${arc * frac} ${c}`);
    el.style.setProperty("--gc", tempColor(t));
  });

  // fanlar
  s.fans.forEach((f, i) => {
    const rpm = document.querySelector(`[data-fan-rpm="${i}"]`);
    if (rpm) rpm.textContent = String(f.rpm);
    const bar = document.querySelector<HTMLElement>(`[data-fan-bar="${i}"]`);
    if (bar) bar.style.width = `${Math.min(100, (f.rpm / f.max) * 100)}%`;
    const spin = document.querySelector<HTMLElement>(`[data-spin="${i}"]`);
    if (spin) spin.style.setProperty("--spin", f.rpm > 0 ? `${Math.max(0.25, 2400 / f.rpm)}s` : "0s");
    spin?.classList.toggle("still", f.rpm === 0);
  });

  // pil görseli
  document.querySelectorAll<HTMLElement>("[data-batt-fill]").forEach((el) => {
    el.style.width = `${b.percent ?? 0}%`;
    el.classList.toggle("low", (b.percent ?? 100) <= 20);
  });
  document.querySelectorAll<HTMLElement>("[data-batt-bolt]").forEach((el) => el.classList.toggle("show", b.status === "Charging"));

  // grafik
  const chart = (k: string, d: string) => document.querySelector(`[data-chart="${k}"]`)?.setAttribute("d", d);
  chart("cpu", linePath(hist.cpu));
  chart("cpuArea", linePath(hist.cpu, true));
  chart("gpu", linePath(hist.gpu));

  // seçimler
  document.querySelectorAll<HTMLElement>("[data-profile]").forEach((el) => el.classList.toggle("active", el.dataset.profile === prof));
  document.querySelectorAll<HTMLInputElement>("[data-feature]").forEach((el) => {
    if (!busy.has(el.dataset.feature!)) el.checked = on(el.dataset.feature!);
  });
  document.querySelectorAll<HTMLElement>("[data-tile]").forEach((el) => {
    const id = el.dataset.tile!;
    if (!feat(id)?.available) return;
    el.classList.toggle("active", on(id));
    const st = el.querySelector("[data-tile-state]");
    if (st) st.textContent = on(id) ? t("common.on") : t("common.off");
  });
  document.querySelectorAll<HTMLElement>("[data-level]").forEach((el) => {
    el.classList.toggle("active", feat(el.dataset.level!)?.value === Number(el.dataset.v));
  });
  updateKbd();
  const charge = on("rapid_charge") ? "rapid" : on("conservation") ? "conservation" : "normal";
  document.querySelectorAll<HTMLElement>("[data-charge]").forEach((el) => el.classList.toggle("active", el.dataset.charge === charge));
}

async function poll() {
  try {
    snap = await api.snapshot();
    const s = snap.sensors;
    if (s.cpuTemp != null) hist.cpu.push(s.cpuTemp);
    if (s.gpu.temp != null) hist.gpu.push(s.gpu.temp);
    hist.cpu.splice(0, Math.max(0, hist.cpu.length - HISTORY));
    hist.gpu.splice(0, Math.max(0, hist.gpu.length - HISTORY));
    const k = keyOf(snap);
    if (k !== structureKey) {
      structureKey = k;
      renderView();
    } else update();
  } catch (e) {
    console.error(e);
  }
}

// ---------- Eylemler ----------

function toast(msg: string, kind: "ok" | "err" = "ok") {
  const box = document.getElementById("toasts")!;
  const t = document.createElement("div");
  t.className = `toast ${kind}`;
  t.innerHTML = `${kind === "ok" ? icon.check : icon.alert}<span>${esc(msg)}</span>`;
  box.appendChild(t);
  setTimeout(() => t.classList.add("out"), 2600);
  setTimeout(() => t.remove(), 3000);
}

async function setFeature(id: string, value: number) {
  busy.add(id);
  try {
    await api.setFeature(id, value);
    if (id === "gsync") toast(t("toast.reboot"));
  } catch (e) {
    toast(trError(e), "err");
  } finally {
    busy.delete(id);
    await poll();
  }
}

async function setCharge(mode: string) {
  if (mode === "normal") {
    if (on("conservation")) await setFeature("conservation", 0);
    if (on("rapid_charge")) await setFeature("rapid_charge", 0);
  } else await setFeature(mode === "rapid" ? "rapid_charge" : "conservation", 1);
}

document.addEventListener("click", async (ev) => {
  const btn = (ev.target as HTMLElement).closest<HTMLElement>("button");
  if (!btn || btn.hasAttribute("disabled")) return;
  const d = btn.dataset;
  if (d.lang) {
    setLang(d.lang as Lang);
    shell();
    renderView();
  } else if (d.view) {
    view = d.view as ViewId;
    try { localStorage.setItem("nabiz.view", view); } catch {}
    renderView();
  } else if (d.profile) {
    try {
      await api.setProfile(d.profile);
      await poll();
    } catch (e) {
      toast(trError(e), "err");
    }
  } else if (d.tile) {
    await setFeature(d.tile, on(d.tile) ? 0 : 1);
  } else if (d.level) {
    await setFeature(d.level, Number(d.v));
  } else if (d.charge) {
    await setCharge(d.charge);
  } else if (d.fx && kbd) {
    kbd.state.effect = d.fx as KbdState["effect"];
    pushKbd(true);
  } else if (d.preset && kbd) {
    kbd.state.colors = [...KBD_PRESETS[Number(d.preset)]];
    if (kbd.state.effect === "off" || kbd.state.effect === "smooth" || kbd.state.effect.startsWith("wave")) kbd.state.effect = "static";
    pushKbd(true);
  } else if (d.kbright && kbd) {
    kbd.state.brightness = Number(d.kbright);
    pushKbd(true);
  } else if (d.kspeed && kbd) {
    kbd.state.speed = Number(d.kspeed);
    pushKbd(true);
  } else if (d.copy) {
    await navigator.clipboard?.writeText(d.copy).catch(() => {});
    toast(t("toast.copied"));
  }
});

document.addEventListener("change", (ev) => {
  const el = ev.target as HTMLInputElement;
  if (el.dataset.feature) setFeature(el.dataset.feature, el.checked ? 1 : 0);
});

// Renk seçici sürüklenirken klavye canlı değişir.
document.addEventListener("input", (ev) => {
  const el = ev.target as HTMLInputElement;
  if (el.dataset.zone == null || !kbd) return;
  kbd.state.colors[Number(el.dataset.zone)] = el.value.slice(1);
  if (kbd.state.effect !== "static" && kbd.state.effect !== "breath") kbd.state.effect = "static";
  pushKbd();
});

// ---------- Başlat ----------

setLang(lang);
shell();
api.systemInfo().then((s) => {
  sys = s;
  // Üreticiye bağlı ipuçları (Fn + Q, sürücüler) sistem bilgisi gelince belli olur.
  renderView();
});
api.keyboard().then((k) => {
  kbd = k;
  if (view === "input") renderView();
});
poll();
// Pencere gizliyken (simge durumunda) donanımı sorgulama.
setInterval(() => document.hidden || poll(), 2000);
