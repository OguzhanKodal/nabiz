// Tauri içinde Rust'a gider; düz tarayıcıda (tasarım için) sahte veriye düşer.
import type { KbdInfo, KbdState, Snapshot, SystemInfo } from "./types";
import * as mock from "./mock";

const inTauri = "__TAURI_INTERNALS__" in window;

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!inTauri) return mock.handle(cmd, args) as T;
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(cmd, args);
}

export const api = {
  snapshot: () => call<Snapshot>("snapshot"),
  systemInfo: () => call<SystemInfo>("system_info"),
  setFeature: (id: string, value: number) => call<void>("set_feature", { id, value: String(value) }),
  setProfile: (value: string) => call<void>("set_profile", { value }),
  keyboard: () => call<KbdInfo>("get_keyboard"),
  setKeyboard: (state: KbdState) => call<void>("set_keyboard", { state }),
};
