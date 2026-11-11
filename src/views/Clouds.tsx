import { Cloud, LogOut, RefreshCw, ExternalLink, ShieldCheck } from "lucide-react";
import { Card, Legend, Spinner, StackedBar, StatusPill, UsageBar } from "../components";
import { ago, bytes, eur, pct } from "../format";
import type { CloudAccount } from "../types";
import type { Ctx } from "../ctx";
import { api } from "../api";

function AccountDetail(p: {
  a: CloudAccount; name: string; color: string; manageUrl: string; busy: boolean;
  onRefresh: () => void; onDisconnect: () => void; onManage: () => void;
}) {
  const { a } = p;
  return (
    <Card title={p.name} icon={<Cloud size={16} color={p.color} />} sub={`${a.accountLabel} · updated ${ago(a.fetchedAt)}`}
      end={<>
        <StatusPill status={a.status} />
        <button className="btn" onClick={p.onRefresh} disabled={p.busy}>{p.busy ? <Spinner /> : <RefreshCw size={14} />}Refresh</button>
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
          <tr><td className="muted">Plan detected from</td><td className="r">{a.plan.exact ? "Your account" : "Quota size"}</td></tr>
          <tr><td className="muted">Price</td><td className="r">{eur(a.plan.monthlyEur)}/mo {!a.plan.priceVerified && <span className="pill warn">unverified</span>}</td></tr>
          <tr><td className="muted">Free space</td><td className="r num">{bytes(Math.max(0, a.limit - a.used))}</td></tr>
          <tr><td className="muted">Over plan by</td><td className="r num" style={{ color: a.overBy ? "var(--bad)" : undefined }}>{a.overBy ? bytes(a.overBy) : "Not over"}</td></tr>
          {a.familyShared && <tr><td className="muted">Family Sharing</td><td className="r">Shared plan</td></tr>}
        </tbody>
      </table>
      <div className="row mt">
        <button className="btn" onClick={p.onManage}><ExternalLink size={14} />Manage plan</button>
        <button className="btn ghost danger" onClick={p.onDisconnect}><LogOut size={14} />Disconnect</button>
      </div>
    </Card>
  );
}

export default function Clouds({ c }: { c: Ctx }) {
  const { d, busy } = c;
  const googleReady = !!c.settings?.googleClientId;

  return (
    <div className="grid g2">
      {d.icloud ? (
        <AccountDetail a={d.icloud} name="iCloud" color="var(--icloud)" manageUrl="https://www.icloud.com/settings/" busy={!!busy.icloud}
          onRefresh={() => c.task("icloud", api.icloudRefresh)}
          onDisconnect={() => c.task("icloud", api.icloudDisconnect)}
          onManage={() => c.act({ label: "", kind: "open_url", target: "https://www.icloud.com/settings/" })} />
      ) : (
        <Card title="iCloud" icon={<Cloud size={16} color="var(--icloud)" />} sub="Apple Account">
          <div className="connect">
            <p>Sign in on Apple's own iCloud page inside OmniStore. Your password and 2FA code go to Apple only. OmniStore then reads your plan, usage by category, and the Apple devices on your account.</p>
            <button className="btn primary" disabled={!!busy.icloud} onClick={() => c.task("icloud", api.icloudConnect)}>
              {busy.icloud && <Spinner />}Sign in with Apple Account
            </button>
            <span className="muted small row"><ShieldCheck size={14} />Session stays on this PC. Disconnect signs you out.</span>
          </div>
        </Card>
      )}

      {d.google ? (
        <AccountDetail a={d.google} name="Google" color="var(--google)" manageUrl="https://one.google.com/storage" busy={!!busy.google}
          onRefresh={() => c.task("google", api.googleRefresh)}
          onDisconnect={() => c.task("google", api.googleDisconnect)}
          onManage={() => c.act({ label: "", kind: "open_url", target: "https://one.google.com/storage" })} />
      ) : (
        <Card title="Google" icon={<Cloud size={16} color="var(--google)" />} sub="Google One, Drive, Gmail, Photos">
          <div className="connect">
            <p>Google sign-in opens in your browser. Approve access and come back here. OmniStore reads your storage quota and keeps a small hidden app folder to sync your devices. It cannot open your files, mail or photos.</p>
            <button className="btn primary" disabled={!googleReady || !!busy.google} onClick={() => c.task("google", api.googleConnect)}>
              {busy.google && <Spinner />}Sign in with Google
            </button>
            {!googleReady && (
              <span className="muted small">Add your Google OAuth client ID in <a href="#" onClick={(e) => { e.preventDefault(); c.go("settings"); }}>Settings</a> first.</span>
            )}
          </div>
        </Card>
      )}

      {(d.icloud || d.google) && (
        <Card title="Combined" sub="All connected clouds">
          <div className="row" style={{ justifyContent: "space-between" }}>
            <span className="num strong">{bytes(d.totals.cloudUsed)} of {bytes(d.totals.cloudCapacity)}</span>
            <span className="muted num">{eur(d.totals.monthlySpendEur)}/mo</span>
          </div>
          <div className="mt"><UsageBar used={d.totals.cloudUsed} total={d.totals.cloudCapacity} thick /></div>
        </Card>
      )}
    </div>
  );
}
