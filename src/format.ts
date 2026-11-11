export function bytes(b: number | null | undefined): string {
  const n = b ?? 0;
  if (n >= 1e12) return `${(n / 1e12).toFixed(2)} TB`;
  if (n >= 1e9) return `${(n / 1e9).toFixed(1)} GB`;
  if (n >= 1e6) return `${(n / 1e6).toFixed(0)} MB`;
  if (n >= 1e3) return `${(n / 1e3).toFixed(0)} KB`;
  return `${n} B`;
}

export function eur(x: number | null | undefined): string {
  return `€${(x ?? 0).toFixed(2)}`;
}

export function pct(used: number, total: number): number {
  if (!total) return 0;
  return Math.min(100, (used / total) * 100);
}

export function ago(iso?: string | null): string {
  if (!iso) return "never";
  const s = (Date.now() - new Date(iso).getTime()) / 1000;
  if (s < 60) return "just now";
  if (s < 3600) return `${Math.floor(s / 60)} min ago`;
  if (s < 86400) return `${Math.floor(s / 3600)} h ago`;
  return `${Math.floor(s / 86400)} d ago`;
}

export const STATUS_LABEL: Record<string, string> = {
  ok: "Healthy",
  near: "Nearly full",
  critical: "Almost full",
  over: "Over plan",
};
