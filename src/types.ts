export interface Fan {
  label: string;
  rpm: number;
  max: number;
}

export interface Gpu {
  present: boolean;
  name: string | null;
  state: string | null;
  temp: number | null;
  util: number | null;
  power: number | null;
}

export interface Battery {
  present: boolean;
  percent: number | null;
  status: string | null;
  acOnline: boolean;
  power: number | null;
  health: number | null;
  cycles: number | null;
  designWh: number | null;
  fullWh: number | null;
}

export interface Sensors {
  cpuTemp: number | null;
  cpuUsage: number | null;
  cpuFreqMhz: number | null;
  memUsedGb: number | null;
  memTotalGb: number | null;
  icTemp: number | null;
  fans: Fan[];
  fansUnreliable: boolean;
  gpu: Gpu;
  battery: Battery;
}

export interface FeatureState {
  id: string;
  available: boolean;
  value: number | null;
  max: number | null;
}

export interface Snapshot {
  sensors: Sensors;
  features: FeatureState[];
  profile: { current: string | null; choices: string[] };
}

export interface KbdState {
  effect: "static" | "breath" | "smooth" | "wave-left" | "wave-right" | "off";
  speed: number;
  brightness: number;
  colors: string[];
}

export interface KbdInfo {
  available: boolean;
  state: KbdState;
}

export interface SystemInfo {
  model: string | null;
  machineType: string | null;
  vendor: string | null;
  bios: string | null;
  kernel: string | null;
  os: string | null;
  cpu: string | null;
  legionDriver: boolean;
  ideapadDriver: boolean;
  demo: boolean;
}
