import { useEffect, useState } from "react";
import { Card } from "../components";
import type { Ctx } from "../ctx";
import { api, DEMO } from "../api";

export default function SettingsView({ c }: { c: Ctx }) {
  const s = c.settings;
  const [deviceName, setDeviceName] = useState("");
  const [clientId, setClientId] = useState("");
  const [secret, setSecret] = useState("");
  const [minMb, setMinMb] = useState(500);
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    if (!s) return;
    setDeviceName(s.deviceName);
    setClientId(s.googleClientId);
    setSecret(s.googleClientSecret);
    setMinMb(s.largeFileMinMb);
  }, [s]);

  const save = () =>
    c.task("settings", async () => {
      const next = await api.saveSettings({ deviceName, googleClientId: clientId, googleClientSecret: secret, largeFileMinMb: minMb });
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
        <h3 style={{ fontSize: 14, margin: "22px 0 12px" }}>Google sign-in</h3>
        <div className="field">
          <label>OAuth client ID</label>
          <input placeholder="1234567890-abc.apps.googleusercontent.com" value={clientId} onChange={(e) => setClientId(e.target.value)} />
        </div>
        <div className="field">
          <label>OAuth client secret</label>
          <input type="password" placeholder="GOCSPX-…" value={secret} onChange={(e) => setSecret(e.target.value)} />
        </div>
        <div className="row">
          <button className="btn primary" onClick={save}>Save</button>
          {saved && <span className="muted small">Saved</span>}
          {DEMO && <span className="muted small">Demo mode: settings are not stored</span>}
        </div>
      </Card>

      <Card title="Set up Google sign-in (one time, free)">
        <ol className="steps">
          <li>Open <code>console.cloud.google.com</code> and create a project named <b>OmniStore</b>.</li>
          <li>APIs &amp; Services → Library → enable <b>Google Drive API</b>.</li>
          <li>Google Auth Platform → Branding: app name OmniStore, your email. Audience: <b>External</b>, add yourself as a <b>test user</b>.</li>
          <li>Data Access → Add scope <code>.../auth/drive.appdata</code> (non-sensitive, no Google review needed).</li>
          <li>Clients → Create client → type <b>Desktop app</b> → copy the client ID and secret here.</li>
          <li>Audience → <b>Publish app</b>. Testing mode signs you out after 7 days. No Google review is needed for this scope.</li>
          <li>Back in OmniStore: Clouds → <b>Sign in with Google</b>.</li>
        </ol>
        <p className="muted small" style={{ marginTop: 14 }}>
          This identifies the OmniStore app to Google. Your users only ever see Google's normal sign-in and consent screen.
          Before public release, set the build-time variables <code>OMNISTORE_GOOGLE_CLIENT_ID</code> and <code>OMNISTORE_GOOGLE_CLIENT_SECRET</code> so nobody has to paste anything.
        </p>
        <h3 style={{ fontSize: 14, margin: "22px 0 8px" }}>Privacy</h3>
        <p className="muted small" style={{ margin: 0 }}>
          Everything is scanned on this device. Apple and Google passwords are typed into Apple's and Google's own pages only.
          The Google token is kept in Windows Credential Manager. No OmniStore server is involved.
        </p>
      </Card>
    </div>
  );
}
