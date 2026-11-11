import { Cloud, Euro, HardDrive, PiggyBank, Sparkles, Smartphone } from "lucide-react";
import { Card, DeviceIcon, FindingRow, Kpi, Ring, StatusPill, TrendChart, UsageBar, statusColor } from "../components";
import { bytes, eur } from "../format";
import type { ReactNode } from "react";
import type { CloudAccount } from "../types";
import type { Ctx } from "../ctx";
import { AppleButton, GoogleButton } from "./Clouds";

function share(values: (number | null | undefined)[], limit?: number): number[] {
  if (!limit) return [];
  return values.filter((v): v is number => typeof v === "number").map((v) => Math.min(110, (v / limit) * 100));
}

function AccountCard(p: { a?: CloudAccount | null; name: string; color: string; connect: ReactNode; hint: string }) {
  const { a } = p;
  if (!a) {
    return (
      <Card title={p.name} icon={<Cloud size={16} color={p.color} />} sub="Not signed in">
        <div className="connect">
          <p>{p.hint}</p>
          {p.connect}
        </div>
      </Card>
    );
  }
  return (
    <Card title={p.name} icon={<Cloud size={16} color={p.color} />} sub={a.accountLabel} end={<StatusPill status={a.status} />}>
      <div className="account">
        <Ring used={a.used} total={a.limit} color={statusColor(a.status)} />
        <div className="meta">
          <div className="title ellipsis">{a.plan.name}</div>
          <div className="muted num">{bytes(a.used)} of {bytes(a.limit)}</div>
          <div className="muted small">
            {a.plan.id.endsWith("-custom") ? "Custom plan" : `${eur(a.plan.monthlyEur)}/mo`}
            {a.overBy > 0 && <span style={{ color: "var(--bad)" }}> · over by {bytes(a.overBy)}</span>}
          </div>
        </div>
      </div>
    </Card>
  );
}

export default function Overview({ c }: { c: Ctx }) {
  const { d, busy } = c;
  const t = d.totals;
  const a = d.icloud;
  const devices = [d.thisDevice, ...d.devices.filter((x) => x.totalBytes > 0)].filter(Boolean);
  const hist = d.history.slice(-30);
  const top = d.findings.slice(0, 5);

  return (
    <>
      <div className="grid g4">
        <Kpi label="Cloud storage" icon={<Cloud size={14} />}
          value={t.cloudCapacity ? <>{bytes(t.cloudUsed)} <small>of {bytes(t.cloudCapacity)}</small></> : "Not signed in"}
          hint={t.cloudCapacity ? <UsageBar used={t.cloudUsed} total={t.cloudCapacity} /> : "Sign in to iCloud or Google"} />
        <Kpi label="Monthly cloud spend" icon={<Euro size={14} />} value={eur(t.monthlySpendEur)} hint={`${eur(t.monthlySpendEur * 12)} per year`} />
        <Kpi label="Reclaimable now" icon={<Sparkles size={14} />} value={bytes(t.reclaimableBytes)} hint="Recycle Bin, temp files, old backups" />
        <Kpi label="Potential savings" icon={<PiggyBank size={14} />} value={<>{eur(t.potentialSavingsEur)}<small>/mo</small></>}
          hint={t.potentialSavingsEur > 0 ? "From right-sizing your plan" : "Your plan fits your usage"} />
      </div>

      <div className="grid g3 mt">
        <AccountCard a={a} name="iCloud" color="var(--icloud)" connect={<AppleButton c={c} />} hint="See your iCloud plan, what fills it and how much you are over." />
        <AccountCard a={d.google} name="Google" color="var(--google)" connect={<GoogleButton c={c} />} hint="See your Google One plan across Drive, Gmail and Photos." />

        <Card title="This device" icon={<HardDrive size={16} />} sub={d.thisDevice?.name ?? "Scanning…"}>
          {d.thisDevice ? (
            <div className="account">
              <Ring used={d.thisDevice.totalBytes - d.thisDevice.freeBytes} total={d.thisDevice.totalBytes} color="var(--primary)" />
              <div className="meta">
                <div className="title">{bytes(d.thisDevice.freeBytes)} free</div>
                <div className="muted num">of {bytes(d.thisDevice.totalBytes)} internal</div>
                <div className="muted small ellipsis">{d.thisDevice.os}</div>
              </div>
            </div>
          ) : <div className="muted">Reading drives…</div>}
        </Card>

      </div>

      <div className="grid g2 mt">
        <Card title="Fix first" icon={<Sparkles size={16} />} sub={`${d.findings.length} recommendations`}
          end={<button className="btn ghost" onClick={() => c.go("optimise")}>See all</button>}>
          {top.length ? top.map((f) => (
            <FindingRow key={f.id} f={f} busy={!!busy[`act:${f.id}`]} onAction={(x) => x.action && c.act(x.action, `act:${x.id}`)} />
          ))
            : <div className="empty">Nothing to fix. Everything is within plan.</div>}
        </Card>
        <div className="grid" style={{ alignContent: "start" }}>
          <Card title="Devices" icon={<Smartphone size={16} />} sub={`${devices.length} with storage data`}
            end={<button className="btn ghost" onClick={() => c.go("devices")}>Details</button>}>
            <div className="grid" style={{ gap: 14 }}>
              {devices.map((x) => x && (
                <div key={x.id} className="device">
                  <div className="top">
                    <div className="icon-chip"><DeviceIcon kind={x.kind} size={16} /></div>
                    <div className="grow">
                      <div className="row" style={{ justifyContent: "space-between", flexWrap: "nowrap" }}>
                        <span className="name ellipsis">{x.name}</span>
                        <span className="muted small num" style={{ flex: "none" }}>{bytes(x.freeBytes)} free of {bytes(x.totalBytes)}</span>
                      </div>
                      <div style={{ marginTop: 6 }}><UsageBar used={x.totalBytes - x.freeBytes} total={x.totalBytes} /></div>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          </Card>
          {(a || d.google) && (
            <Card title="Cloud usage, last 30 days" sub="Percent of each plan used">
              <TrendChart min={0} max={100} series={[
                { color: "var(--icloud)", label: "iCloud", points: share(hist.map((h) => h.icloudUsed), a?.limit) },
                { color: "var(--google)", label: "Google", points: share(hist.map((h) => h.googleUsed), d.google?.limit) },
              ]} />
            </Card>
          )}
        </div>
      </div>
    </>
  );
}
