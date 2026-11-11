import type { Action, Dashboard, Settings } from "./types";

export type View = "overview" | "clouds" | "devices" | "local" | "optimise" | "settings";

export interface Ctx {
  d: Dashboard;
  settings: Settings | null;
  busy: Record<string, boolean>;
  /** Runs a backend task with a busy flag and error banner. */
  task: (key: string, fn: () => Promise<unknown>) => Promise<void>;
  /** Runs a finding/button action. `key` drives that button's spinner. */
  act: (a: Action, key?: string) => void;
  go: (v: View) => void;
  setSettings: (s: Settings) => void;
  version: string;
  checkUpdates: () => Promise<void>;
}
