import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Action, Dashboard, Settings } from "./types";
import { demoDashboard, demoSettings } from "./demo";

/** Outside the Tauri shell (plain browser / `npm run dev`) the UI runs on demo data. */
export const DEMO = !isTauri();

const wait = (ms: number) => new Promise((r) => setTimeout(r, ms));

export const api = {
  getDashboard: () => (DEMO ? Promise.resolve(demoDashboard()) : invoke<Dashboard>("get_dashboard")),
  getSettings: () => (DEMO ? Promise.resolve(demoSettings()) : invoke<Settings>("get_settings")),
  saveSettings: (input: Partial<Settings>) =>
    DEMO ? Promise.resolve({ ...demoSettings(), ...input }) : invoke<Settings>("save_settings", { input }),
  scanLocal: () => (DEMO ? wait(700).then(demoDashboard) : invoke<Dashboard>("scan_local")),
  deepScan: () => (DEMO ? wait(1200).then(demoDashboard) : invoke<Dashboard>("deep_scan")),
  refreshAll: () => (DEMO ? wait(900).then(() => [] as string[]) : invoke<string[]>("refresh_all")),
  icloudConnect: () => (DEMO ? wait(300) : invoke<void>("icloud_connect")),
  icloudRefresh: () => (DEMO ? wait(300) : invoke<void>("icloud_refresh")),
  icloudDisconnect: () => (DEMO ? Promise.resolve(demoDashboard()) : invoke<Dashboard>("icloud_disconnect")),
  googleConnect: () => (DEMO ? wait(600).then(demoDashboard) : invoke<Dashboard>("google_connect")),
  googleRefresh: () => (DEMO ? wait(600).then(demoDashboard) : invoke<Dashboard>("google_refresh")),
  googleDisconnect: () => (DEMO ? Promise.resolve(demoDashboard()) : invoke<Dashboard>("google_disconnect")),
  runAction: (action: Action) => (DEMO ? Promise.resolve(console.info("demo action", action)) : invoke<void>("run_action", { action })),
};

export function onDashboard(cb: (d: Dashboard) => void) {
  if (DEMO) return () => {};
  const p = listen<Dashboard>("dashboard://updated", (e) => cb(e.payload));
  return () => void p.then((un) => un());
}

export function onEvent(name: string, cb: (payload: string) => void) {
  if (DEMO) return () => {};
  const p = listen<string>(name, (e) => cb(e.payload));
  return () => void p.then((un) => un());
}
