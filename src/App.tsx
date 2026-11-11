import { useCallback, useEffect, useState } from "react";
import { Cloud, Database, HardDrive, LayoutDashboard, RefreshCw, Settings as SettingsIcon, Smartphone, Sparkles } from "lucide-react";
import "./styles.css";
import { api, DEMO, onDashboard, onEvent } from "./api";
import { Spinner } from "./components";
import { ago } from "./format";
import type { Action, Dashboard, Settings } from "./types";
import type { Ctx, View } from "./ctx";
import Overview from "./views/Overview";
import Clouds from "./views/Clouds";
import Devices from "./views/Devices";
import ThisDevice from "./views/ThisDevice";
import Optimise from "./views/Optimise";
import SettingsView from "./views/SettingsView";

const EMPTY: Dashboard = {
  devices: [], largeFiles: [], findings: [], history: [],
  totals: { cloudUsed: 0, cloudCapacity: 0, localUsed: 0, localCapacity: 0, monthlySpendEur: 0, reclaimableBytes: 0, potentialSavingsEur: 0 },
};

const NAV: { id: View; label: string; icon: typeof Cloud; title: string; sub: string }[] = [
  { id: "overview", label: "Overview", icon: LayoutDashboard, title: "Storage overview", sub: "Every cloud plan, device and drive in one place" },
  { id: "clouds", label: "Clouds", icon: Cloud, title: "Cloud accounts", sub: "Plans, usage and overage" },
  { id: "devices", label: "Devices", icon: Smartphone, title: "Devices", sub: "PC, Mac, iPhone, iPad and backups" },
  { id: "local", label: "This device", icon: HardDrive, title: "This device", sub: "Drives, hardware, sync apps and large files" },
  { id: "optimise", label: "Optimise", icon: Sparkles, title: "Optimise", sub: "What to fix, free up and stop paying for" },
  { id: "settings", label: "Settings", icon: SettingsIcon, title: "Settings", sub: "Sign-in setup and preferences" },
];

export default function App() {
  const [d, setD] = useState<Dashboard>(EMPTY);
  const [settings, setSettings] = useState<Settings | null>(null);
  const [view, setView] = useState<View>("overview");
  const [busy, setBusy] = useState<Record<string, boolean>>({});
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  const task = useCallback(async (key: string, fn: () => Promise<unknown>) => {
    setBusy((b) => ({ ...b, [key]: true }));
    setError(null);
    try {
      const out = await fn();
      if (out && typeof out === "object" && "findings" in (out as object)) setD(out as Dashboard);
      if (Array.isArray(out) && out.length) setError(out.join(" · "));
    } catch (e) {
      setError(String(e));
    } finally {
      // iCloud finishes asynchronously through an event, so its spinner stops there.
      if (key !== "icloud") setBusy((b) => ({ ...b, [key]: false }));
    }
  }, []);

  const act = useCallback((a: Action) => void task("action", () => api.runAction(a)), [task]);

  useEffect(() => {
    api.getDashboard().then((dash) => {
      setD(dash);
      const stale = !dash.local?.scannedAt || Date.now() - new Date(dash.local.scannedAt).getTime() > 3600e3;
      if (stale) task("local", api.scanLocal);
    });
    api.getSettings().then(setSettings);
    const offD = onDashboard(setD);
    const offI = onEvent("icloud://status", (s) => {
      if (s === "needs_login") {
        setNotice("Sign in to iCloud in the window that just opened.");
      } else {
        setBusy((b) => ({ ...b, icloud: false }));
        setNotice(null);
        if (s.startsWith("error")) setError(`iCloud ${s}`);
      }
    });
    const offS = onEvent("scan://progress", (s) => setNotice(s || null));
    return () => { offD(); offI(); offS(); };
  }, [task]);

  // Silent iCloud refresh can take a while; never leave the spinner stuck.
  useEffect(() => {
    if (!busy.icloud) return;
    const t = setTimeout(() => setBusy((b) => ({ ...b, icloud: false })), 120_000);
    return () => clearTimeout(t);
  }, [busy.icloud]);

  const c: Ctx = { d, settings, busy, task, act, go: setView, setSettings };
  const nav = NAV.find((n) => n.id === view)!;
  const urgent = d.findings.filter((f) => f.severity === "critical").length;
  const anyBusy = busy.refresh || busy.local;

  return (
    <div className="app">
      <aside className="side">
        <div className="brand"><div className="brand-mark"><Database size={17} /></div>OmniStore</div>
        {NAV.map((n) => (
          <button key={n.id} className={`nav ${view === n.id ? "active" : ""}`} onClick={() => setView(n.id)}>
            <n.icon size={17} />{n.label}
            {n.id === "optimise" && urgent > 0 && <span className="count">{urgent}</span>}
          </button>
        ))}
        <div className="side-foot">
          {DEMO ? "Demo data (browser preview)" : `Updated ${ago(d.updatedAt)}`}
        </div>
      </aside>

      <main className="main">
        <div className="head">
          <div>
            <h1>{nav.title}</h1>
            <p>{nav.sub}</p>
          </div>
          <button className="btn primary" disabled={!!anyBusy} onClick={() => task("refresh", api.refreshAll)}>
            {anyBusy ? <Spinner /> : <RefreshCw size={14} />}Refresh all
          </button>
        </div>

        {error && <div className="banner err">{error}<button className="btn ghost" style={{ marginLeft: "auto" }} onClick={() => setError(null)}>Dismiss</button></div>}
        {notice && <div className="banner"><Spinner />{notice}</div>}

        {view === "overview" && <Overview c={c} />}
        {view === "clouds" && <Clouds c={c} />}
        {view === "devices" && <Devices c={c} />}
        {view === "local" && <ThisDevice c={c} />}
        {view === "optimise" && <Optimise c={c} />}
        {view === "settings" && <SettingsView c={c} />}
      </main>
    </div>
  );
}
