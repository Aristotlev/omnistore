import type { ReactNode } from "react";
import {
  Cloud, HardDrive, Laptop, Monitor, Smartphone, Tablet, Watch, Archive, TriangleAlert, CircleAlert, Info, LoaderCircle,
} from "lucide-react";
import type { Finding, Severity, UsageSlice } from "./types";
import { bytes, eur, pct, STATUS_LABEL } from "./format";

export function Card(p: { title?: string; sub?: string; icon?: ReactNode; end?: ReactNode; children: ReactNode; className?: string }) {
  return (
    <section className={`card ${p.className ?? ""}`}>
      {(p.title || p.end) && (
        <div className="card-h">
          {p.icon && <div className="icon-chip">{p.icon}</div>}
          <div>
            {p.title && <h3>{p.title}</h3>}
            {p.sub && <div className="sub">{p.sub}</div>}
          </div>
          {p.end && <div className="end">{p.end}</div>}
        </div>
      )}
      {p.children}
    </section>
  );
}

export function Kpi(p: { label: string; value: ReactNode; hint?: ReactNode; icon?: ReactNode }) {
  return (
    <section className="card kpi">
      <div className="label">{p.icon}{p.label}</div>
      <div className="value num">{p.value}</div>
      {p.hint && <div className="hint">{p.hint}</div>}
    </section>
  );
}

export function Ring(p: { used: number; total: number; color: string; size?: number; children?: ReactNode }) {
  const size = p.size ?? 92;
  const stroke = 9;
  const r = (size - stroke) / 2;
  const c = 2 * Math.PI * r;
  const v = pct(p.used, p.total) / 100;
  return (
    <div style={{ position: "relative", width: size, height: size }}>
      <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`} role="img" aria-label={`${Math.round(v * 100)}% used`}>
        <circle cx={size / 2} cy={size / 2} r={r} fill="none" stroke="var(--surface-2)" strokeWidth={stroke} />
        <circle
          cx={size / 2} cy={size / 2} r={r} fill="none" stroke={p.color} strokeWidth={stroke} strokeLinecap="round"
          strokeDasharray={`${c * v} ${c}`} transform={`rotate(-90 ${size / 2} ${size / 2})`}
        />
      </svg>
      <div style={{ position: "absolute", inset: 0, display: "grid", placeItems: "center", textAlign: "center", fontWeight: 650 }}>
        {p.children ?? `${Math.round(v * 100)}%`}
      </div>
    </div>
  );
}

export function statusColor(status: string) {
  return status === "over" || status === "critical" ? "var(--bad)" : status === "near" ? "var(--warn)" : "var(--good)";
}

export function UsageBar(p: { used: number; total: number; thick?: boolean; color?: string }) {
  const v = pct(p.used, p.total);
  const color = p.color ?? (v >= 95 ? "var(--bad)" : v >= 85 ? "var(--warn)" : "var(--primary)");
  return (
    <div className={`bar ${p.thick ? "thick" : ""}`}>
      <span style={{ width: `${v}%`, background: color }} />
    </div>
  );
}

const FALLBACK = ["#6c8cff", "#2ed3b7", "#ffb547", "#ff5c7a", "#b28cff", "#64d2ff"];

export function StackedBar(p: { slices: UsageSlice[]; total: number }) {
  return (
    <div className="bar thick">
      {p.slices.map((s, i) => (
        <span key={s.key} title={`${s.label}: ${bytes(s.bytes)}`} style={{ width: `${pct(s.bytes, p.total)}%`, background: s.color ?? FALLBACK[i % 6] }} />
      ))}
    </div>
  );
}

export function Legend(p: { slices: UsageSlice[] }) {
  return (
    <div className="legend">
      {p.slices.map((s, i) => (
        <div className="legend-row" key={s.key}>
          <span className="dot" style={{ background: s.color ?? FALLBACK[i % 6] }} />
          <span>{s.label}</span>
          <span className="num muted">{bytes(s.bytes)}</span>
        </div>
      ))}
    </div>
  );
}

export function StatusPill({ status }: { status: string }) {
  return <span className={`pill ${status}`}>{STATUS_LABEL[status] ?? status}</span>;
}

export function Spinner({ size = 15 }: { size?: number }) {
  return <LoaderCircle size={size} className="spin" />;
}

export function SevIcon({ severity }: { severity: Severity }) {
  const Icon = severity === "critical" ? CircleAlert : severity === "warn" ? TriangleAlert : Info;
  return <div className={`sev ${severity}`}><Icon size={16} /></div>;
}

export function FindingRow(p: { f: Finding; onAction: (f: Finding) => void; busy?: boolean }) {
  const { f } = p;
  return (
    <div className="finding">
      <SevIcon severity={f.severity} />
      <div style={{ minWidth: 0 }}>
        <div className="t">{f.title}</div>
        <div className="d">{f.detail}</div>
      </div>
      <div className="row" style={{ justifyContent: "flex-end" }}>
        {f.monthlyEur > 0 && <span className="save">−{eur(f.monthlyEur)}/mo</span>}
        {f.bytes > 0 && f.monthlyEur === 0 && <span className="muted small num">{bytes(f.bytes)}</span>}
        {f.action && (
          <button className="btn" disabled={p.busy} onClick={() => p.onAction(f)}>
            {p.busy && <Spinner />}{f.action.label}
          </button>
        )}
      </div>
    </div>
  );
}

export function DeviceIcon({ kind, size = 18 }: { kind: string; size?: number }) {
  switch (kind) {
    case "ios": return <Smartphone size={size} />;
    case "ipados": return <Tablet size={size} />;
    case "macos": return <Laptop size={size} />;
    case "windows": case "linux": return <Monitor size={size} />;
    case "backup": return <Archive size={size} />;
    case "apple": return <Watch size={size} />;
    case "cloud": return <Cloud size={size} />;
    default: return <HardDrive size={size} />;
  }
}

export function TrendChart(p: { series: { color: string; label: string; points: number[] }[]; min?: number; max?: number }) {
  const w = 600, h = 120, pad = 6;
  const all = p.series.flatMap((s) => s.points);
  if (all.length < 2) return <div className="empty small">The trend appears after a few days of refreshes.</div>;
  const min = p.min ?? Math.min(...all), max = p.max ?? Math.max(...all);
  const span = max - min || 1;
  return (
    <>
    <svg className="trend" viewBox={`0 0 ${w} ${h}`} preserveAspectRatio="none" role="img" aria-label="Storage trend">
      {p.series.map((s, i) => {
        if (s.points.length < 2) return null;
        const step = (w - pad * 2) / (s.points.length - 1);
        const d = s.points.map((v, j) => `${j ? "L" : "M"}${pad + j * step},${h - pad - ((v - min) / span) * (h - pad * 2)}`).join(" ");
        return <path key={i} d={d} fill="none" stroke={s.color} strokeWidth={2.2} vectorEffect="non-scaling-stroke" strokeLinejoin="round" />;
      })}
    </svg>
    <div className="row small muted" style={{ marginTop: 8, gap: 16 }}>
      {p.series.filter((s) => s.points.length > 1).map((s) => (
        <span key={s.label} className="row" style={{ gap: 6 }}>
          <span className="dot" style={{ background: s.color }} />{s.label} {s.points[s.points.length - 1].toFixed(0)}%
        </span>
      ))}
    </div>
    </>
  );
}
