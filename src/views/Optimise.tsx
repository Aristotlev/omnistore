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
        {d.findings.length} recommendations · up to <b>&nbsp;{bytes(freeable)}&nbsp;</b> to review · <b>&nbsp;{eur(d.totals.potentialSavingsEur)}/mo&nbsp;</b> in plan savings
      </div>
      <div className="grid">
        {GROUPS.map((g) => {
          const items = d.findings.filter((f) => f.severity === g.key);
          if (!items.length) return null;
          return (
            <Card key={g.key} title={g.title} sub={`${items.length} items`}>
              {items.map((f) => <FindingRow key={f.id} f={f} onAction={(x) => x.action && c.act(x.action)} />)}
            </Card>
          );
        })}
        {!d.findings.length && <div className="empty">Nothing to optimise yet. Connect your clouds and scan this device.</div>}
      </div>
    </>
  );
}
