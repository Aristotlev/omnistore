import { useEffect, useState } from "react";
import { Download } from "lucide-react";
import { Card, Spinner } from "../components";
import type { Ctx } from "../ctx";
import { api, DEMO } from "../api";

export default function SettingsView({ c }: { c: Ctx }) {
  const s = c.settings;
  const [deviceName, setDeviceName] = useState("");
  const [minMb, setMinMb] = useState(500);
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    if (!s) return;
    setDeviceName(s.deviceName);
    setMinMb(s.largeFileMinMb);
  }, [s]);

  const save = () =>
    c.task("settings", async () => {
      const next = await api.saveSettings({ deviceName, largeFileMinMb: minMb });
      c.setSettings(next);
      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
    });

  return (
    <div className="grid g2">
      <Card title="General">
        <div className="field">
          <label>Device name (shown on your other devices)</label>
          <input value={deviceName} onChange={(e) => setDeviceName(e.target.value)} />
        </div>
        <div className="field">
          <label>Large file threshold (MB)</label>
          <input type="number" min={50} value={minMb} onChange={(e) => setMinMb(Number(e.target.value))} />
        </div>
        <div className="row">
          <button className="btn primary" onClick={save}>Save</button>
          {saved && <span className="muted small">Saved</span>}
          {DEMO && <span className="muted small">Demo mode: settings are not stored</span>}
        </div>
      </Card>

      <div className="grid" style={{ alignContent: "start" }}>
        <Card title="Updates" icon={<Download size={16} />} sub={`Version ${c.version || "…"}`}>
          <p className="muted small" style={{ marginTop: 0 }}>OmniStore checks for updates when it opens and every 6 hours. Updates install in one click.</p>
          <button className="btn" disabled={!!c.busy.update} onClick={() => c.task("update", c.checkUpdates)}>
            {c.busy.update && <Spinner />}Check for updates
          </button>
        </Card>
        <Card title="Privacy">
          <p className="muted small" style={{ margin: 0 }}>
            Everything is scanned on this device. Your Apple password is typed into Apple's own page only.
            OmniStore has no account, no server and no API keys. Devices share storage through your own iCloud Drive or OneDrive folder.
          </p>
        </Card>
      </div>
    </div>
  );
}
