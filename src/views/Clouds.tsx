import { Cloud, LogOut, RefreshCw, ExternalLink, ShieldCheck, KeyRound, FolderOpen } from "lucide-react";
import { useEffect, useState } from "react";
import { Card, Legend, Spinner, StackedBar, StatusPill } from "../components";
import { ago, bytes, eur, pct } from "../format";
import type { CloudAccount } from "../types";
import type { Ctx } from "../ctx";
import { api } from "../api";

function AccountDetail(p: { c: Ctx; a: CloudAccount; name: string; color: string; busyKey: string; manageUrl: string; refresh: () => Promise<unknown>; signOut: () => Promise<unknown> }) {
  const { a, c } = p;
  return (
    <Card title={p.name} icon={<Cloud size={16} color={p.color} />} sub={`${a.accountLabel} · updated ${ago(a.fetchedAt)}`}
      end={<>
        <StatusPill status={a.status} />
        <button className="btn" onClick={() => c.task(p.busyKey, p.refresh)} disabled={!!c.busy[p.busyKey]}>
          {c.busy[p.busyKey] ? <Spinner /> : <RefreshCw size={14} />}Refresh
        </button>
      </>}>
      <div className="row" style={{ justifyContent: "space-between", alignItems: "baseline" }}>
        <div style={{ fontSize: 24, fontWeight: 650 }} className="num">{bytes(a.used)} <span className="muted" style={{ fontSize: 14, fontWeight: 500 }}>of {bytes(a.limit)}</span></div>
        <div className="muted num">{pct(a.used, a.limit).toFixed(1)}% used</div>
      </div>
      <div className="mt"><StackedBar slices={a.breakdown} total={Math.max(a.limit, a.used)} /></div>
      <Legend slices={a.breakdown} />
      {a.note && <p className="muted small" style={{ marginTop: 12 }}>{a.note}</p>}

      <table className="mt2">
        <tbody>
          <tr><td className="muted">Plan</td><td className="r strong">{a.plan.name}</td></tr>
          <tr><td className="muted">Price</td><td className="r">{eur(a.plan.monthlyEur)}/mo {!a.plan.priceVerified && <span className="pill warn">estimate</span>}</td></tr>
          <tr><td className="muted">Free space</td><td className="r num">{bytes(Math.max(0, a.limit - a.used))}</td></tr>
          <tr><td className="muted">Over plan by</td><td className="r num" style={{ color: a.overBy ? "var(--bad)" : undefined }}>{a.overBy ? bytes(a.overBy) : "Not over"}</td></tr>
          {a.familyShared && <tr><td className="muted">Family Sharing</td><td className="r">Shared plan</td></tr>}
        </tbody>
      </table>
      <div className="row mt">
        <button className="btn" onClick={() => c.act({ label: "", kind: "open_url", target: p.manageUrl })}><ExternalLink size={14} />Manage plan</button>
        <button className="btn ghost danger" onClick={() => c.task(`${p.busyKey}-out`, p.signOut)}><LogOut size={14} />Sign out</button>
      </div>
    </Card>
  );
}

export function GoogleButton({ c }: { c: Ctx }) {
  const [available, setAvailable] = useState(true);
  useEffect(() => { api.googleAvailable().then(setAvailable); }, []);
  return (
    <>
      <button className="btn google" disabled={!available || !!c.busy.google} onClick={() => c.task("google", api.googleConnect)}>
        {c.busy.google ? <Spinner /> : <span className="g-mark">G</span>}Continue with Google
      </button>
      {c.busy.google && <span className="muted small">Finish signing in in your browser, then come back here.</span>}
      {!available && <span className="muted small">Google sign-in comes with the published OmniStore build.</span>}
    </>
  );
}

export function AppleButton({ c }: { c: Ctx }) {
  return (
    <button className="btn apple" disabled={!!c.busy.icloud} onClick={() => c.task("icloud", api.icloudConnect)}>
      {c.busy.icloud ? <Spinner /> : <KeyRound size={14} />}Sign in to iCloud
    </button>
  );
}

export default function Clouds({ c }: { c: Ctx }) {
  const { d } = c;
  const apps = (d.local?.syncClients ?? []).filter((s) => s.installed || s.syncRoot);

  return (
    <>
      <div className="grid g2">
        {d.icloud ? (
          <AccountDetail c={c} a={d.icloud} name="iCloud" color="var(--icloud)" busyKey="icloud" manageUrl="https://www.icloud.com/settings/"
            refresh={api.icloudRefresh} signOut={api.icloudDisconnect} />
        ) : (
          <Card title="iCloud" icon={<Cloud size={16} color="var(--icloud)" />} sub="Apple Account">
            <div className="connect">
              <p>Sign in once on Apple's own page. Use a passkey (scan the QR code with your iPhone) or approve the sign-in on your iPhone.</p>
              <AppleButton c={c} />
              <span className="muted small row"><ShieldCheck size={14} />Your password goes to Apple only. You stay signed in on this PC until you sign out.</span>
            </div>
          </Card>
        )}

        {d.google ? (
          <AccountDetail c={c} a={d.google} name="Google" color="var(--google)" busyKey="google" manageUrl="https://one.google.com/storage"
            refresh={api.googleRefresh} signOut={api.googleDisconnect} />
        ) : (
          <Card title="Google" icon={<Cloud size={16} color="var(--google)" />} sub="Google One, Drive, Gmail, Photos">
            <div className="connect">
              <p>Pick your Google account in your browser and click Continue. OmniStore only reads your storage numbers. It cannot open your files, mail or photos.</p>
              <GoogleButton c={c} />
            </div>
          </Card>
        )}
      </div>

      <Card title="Cloud apps on this PC" icon={<Cloud size={16} />} sub="What each app keeps downloaded" className="mt">
        {apps.length ? (
          <table>
            <thead><tr><th>App</th><th className="r">On this PC</th><th className="r">Online-only</th><th /></tr></thead>
            <tbody>
              {apps.map((s) => (
                <tr key={s.id}>
                  <td><div className="strong">{s.name}</div><div className="path ellipsis" style={{ maxWidth: 420 }}>{s.syncRoot ?? "Streaming drive"}</div></td>
                  <td className="r num">{bytes(s.localBytes + s.cacheBytes)}</td>
                  <td className="r num muted">{bytes(s.cloudOnlyBytes)}</td>
                  <td className="r">{s.syncRoot && (
                    <button className="btn ghost" title="Open folder" onClick={() => c.act({ label: "", kind: "open_path", target: s.syncRoot })}><FolderOpen size={14} /></button>
                  )}</td>
                </tr>
              ))}
            </tbody>
          </table>
        ) : <div className="empty">No iCloud, Google Drive, OneDrive or Dropbox apps found.</div>}
      </Card>
    </>
  );
}
