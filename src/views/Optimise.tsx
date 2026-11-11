import { Card, FindingRow } from "../components";
import { bytes, eur } from "../format";
import type { Ctx } from "../ctx";

const GROUPS = [
  { key: "critical", title: "Fix now" },
  { key: "warn", title: "Should fix" },
  { key: "info", title: "Save space and money" },
] as const;

export default function Optimise({ c }: { c: Ctx }) {
  const { d } = c;
  const freeable = d.findings.reduce((s, f) => s + (f.monthlyEur ? 0 : f.bytes), 0);
  return (
    <>
      <div className="banner">
        <span>{d.findings.length} recommendations · up to <b>{bytes(freeable)}</b> to review · <b>{eur(d.totals.potentialSavingsEur)}/mo</b> in plan savings</span>
      </div>
      <div className="grid">
        {GROUPS.map((g) => {
          const items = d.findings.filter((f) => f.severity === g.key);
          if (!items.length) return null;
          return (
            <Card key={g.key} title={g.title} sub={`${items.length} items`}>
              {items.map((f) => <FindingRow key={f.id} f={f} busy={!!c.busy[`act:${f.id}`]} onAction={(x) => x.action && c.act(x.action, `act:${x.id}`)} />)}
            </Card>
          );
        })}
        {!d.findings.length && <div className="empty">Nothing to optimise yet. Sign in to iCloud and scan this device.</div>}
      </div>
    </>
  );
}
