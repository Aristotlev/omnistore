import { useCallback, useEffect, useRef, useState } from "react";
import { Cloud, Database, Download, HardDrive, LayoutDashboard, RefreshCw, Settings as SettingsIcon, Smartphone, Sparkles } from "lucide-react";
import "./styles.css";
import { api, checkForUpdate, DEMO, onDashboard, onEvent, type PendingUpdate } from "./api";
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
  { id: "clouds", label: "Clouds", icon: Cloud, title: "Cloud storage", sub: "iCloud plan, usage and cloud apps on this PC" },
  { id: "devices", label: "Devices", icon: Smartphone, title: "Devices", sub: "PC, Mac, iPhone, iPad and backups" },
  { id: "local", label: "This device", icon: HardDrive, title: "This device", sub: "Drives, hardware, cloud apps and large files" },
  { id: "optimise", label: "Optimise", icon: Sparkles, title: "Optimise", sub: "What to fix, free up and stop paying for" },
  { id: "settings", label: "Settings", icon: SettingsIcon, title: "Settings", sub: "Preferences and updates" },
];

const SIX_HOURS = 6 * 3600 * 1000;

export default function App() {
  const [d, setD] = useState<Dashboard>(EMPTY);
  const [settings, setSettings] = useState<Settings | null>(null);
  const [view, setView] = useState<View>("overview");
  const [busy, setBusy] = useState<Record<string, boolean>>({});
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [toast, setToast] = useState<string | null>(null);
  const [version, setVersion] = useState("");
  const [update, setUpdate] = useState<PendingUpdate | null>(null);
  const [updatePct, setUpdatePct] = useState<number | null>(null);
  const toastTimer = useRef<number | undefined>(undefined);

  const showToast = useCallback((msg: string) => {
    setToast(msg);
    window.clearTimeout(toastTimer.current);
    toastTimer.current = window.setTimeout(() => setToast(null), 4000);
  }, []);

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

  const act = useCallback(
    (a: Action, key?: string) => {
      if (a.kind === "icloud_signin") return void task("icloud", api.icloudConnect);
      void task(key ?? `act:${a.kind}`, async () => {
        const out = await api.runAction(a);
        if (DEMO && a.kind === "empty_recycle_bin") showToast("Recycle Bin emptied. 4.8 GB freed.");
        return out;
      });
    },
    [task, showToast],
  );

  const checkUpdates = useCallback(async () => {
    try {
      const u = await checkForUpdate();
      setUpdate(u);
      if (!u) showToast("OmniStore is up to date.");
    } catch (e) {
      setError(`Update check failed: ${e}`);
    }
  }, [showToast]);

  useEffect(() => {
    api.getDashboard().then((dash) => {
      setD(dash);
      const stale = !dash.local?.scannedAt || Date.now() - new Date(dash.local.scannedAt).getTime() > 3600e3;
      if (stale) task("local", api.scanLocal);
    });
    api.getSettings().then(setSettings);
    api.appVersion().then(setVersion);

    const silentCheck = () => checkForUpdate().then(setUpdate).catch(() => {});
    silentCheck();
    const timer = window.setInterval(silentCheck, SIX_HOURS);

    const offD = onDashboard(setD);
    const offT = onEvent("toast", showToast);
    const offI = onEvent("icloud://status", (s) => {
      if (s === "needs_login") {
        setNotice("Sign in to iCloud in the window that just opened. Use a passkey or approve on your iPhone.");
      } else {
        setBusy((b) => ({ ...b, icloud: false }));
        setNotice(null);
        if (s === "connected") showToast("iCloud connected.");
        if (s.startsWith("error")) setError(`iCloud ${s}`);
      }
    });
    return () => { offD(); offT(); offI(); window.clearInterval(timer); };
  }, [task, showToast]);

  // Silent iCloud refresh can take a while; never leave the spinner stuck.
  useEffect(() => {
    if (!busy.icloud) return;
    const t = setTimeout(() => setBusy((b) => ({ ...b, icloud: false })), 180_000);
    return () => clearTimeout(t);
  }, [busy.icloud]);

  const installUpdate = async () => {
    if (!update) return;
    setUpdatePct(0);
    try {
      await update.install(setUpdatePct);
    } catch (e) {
      setUpdatePct(null);
      setError(`Update failed: ${e}`);
    }
  };

  const c: Ctx = { d, settings, busy, task, act, go: setView, setSettings, version, checkUpdates };
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
          {DEMO ? "Demo data (browser preview)" : `v${version} · updated ${ago(d.updatedAt)}`}
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

        {update && (
          <div className="banner update">
            <Download size={16} />
            {updatePct === null
              ? <>OmniStore {update.version} is available.</>
              : <>Installing OmniStore {update.version}… {updatePct}%. The app restarts by itself.</>}
            {updatePct === null && <button className="btn primary" style={{ marginLeft: "auto" }} onClick={installUpdate}>Update now</button>}
          </div>
        )}
        {error && <div className="banner err">{error}<button className="btn ghost" style={{ marginLeft: "auto" }} onClick={() => setError(null)}>Dismiss</button></div>}
        {notice && <div className="banner"><Spinner />{notice}</div>}

        {view === "overview" && <Overview c={c} />}
        {view === "clouds" && <Clouds c={c} />}
        {view === "devices" && <Devices c={c} />}
        {view === "local" && <ThisDevice c={c} />}
        {view === "optimise" && <Optimise c={c} />}
        {view === "settings" && <SettingsView c={c} />}
      </main>
      {toast && <div className="toast">{toast}</div>}
    </div>
  );
}
