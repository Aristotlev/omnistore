import { useEffect, useState } from "react";
import { Archive, Cable, Cloud, Image, Info, LayoutGrid, Smartphone } from "lucide-react";
import { Card, DeviceIcon, Legend, StackedBar } from "../components";
import { ago, bytes } from "../format";
import type { DeviceSnapshot, UsageSlice } from "../types";
import type { Ctx } from "../ctx";

const KIND: Record<string, string> = {
  windows: "Windows PC", macos: "Mac", ios: "iPhone", ipados: "iPad", android: "Android", apple: "Apple device", linux: "Linux",
};

function withFree(x: DeviceSnapshot): UsageSlice[] {
  const slices = [...(x.breakdown ?? [])];
  if (!slices.length && x.totalBytes) slices.push({ key: "used", label: "Used", bytes: x.totalBytes - x.freeBytes, color: "#6c8cff" });
  if (x.freeBytes) slices.push({ key: "free", label: "Free", bytes: x.freeBytes, color: "var(--surface-2)" });
  return slices;
}

function DeviceTile({ x, active, onClick }: { x: DeviceSnapshot; active: boolean; onClick: () => void }) {
  const has = x.totalBytes > 0;
  return (
    <button className={`card tile ${active ? "active" : ""}`} onClick={onClick}>
      <div className="device">
        <div className="top">
          <div className="icon-chip"><DeviceIcon kind={x.kind} /></div>
          <div className="grow">
            <div className="name ellipsis" title={x.name}>{x.name}</div>
            <div className="muted small ellipsis">{[x.model, x.os].filter(Boolean).join(" · ") || KIND[x.kind]}</div>
          </div>
        </div>
        {has ? (
          <>
            <StackedBar slices={withFree(x)} total={x.totalBytes} />
            <div className="row small" style={{ justifyContent: "space-between" }}>
              <span className="num">{bytes(x.totalBytes - x.freeBytes)} used</span>
              <span className="muted num">{bytes(x.freeBytes)} free of {bytes(x.totalBytes)}</span>
            </div>
          </>
        ) : (
          <div className="muted small">{x.kind === "macos" ? "Install OmniStore on this Mac to read its storage." : x.kind === "ios" || x.kind === "ipados" ? "Plug it into this PC once to read its storage." : "Listed on your Apple Account."}</div>
        )}
        <div className="row small" style={{ justifyContent: "space-between" }}>
          <span className="muted">{x.connection ? `${x.connection} · ` : ""}{ago(x.updatedAt)}</span>
          <span className="source">{x.source}</span>
        </div>
      </div>
    </button>
  );
}

function Detail({ x, c }: { x: DeviceSnapshot; c: Ctx }) {
  const { d } = c;
  const isApple = ["ios", "ipados", "macos"].includes(x.kind);
  const icloudBackups = d.icloud?.breakdown.find((s) => s.key === "backup")?.bytes ?? 0;
  const icloudPhotos = d.icloud?.breakdown.find((s) => s.key === "photos")?.bytes ?? 0;
  const m = x.media;
  const apps = x.topApps ?? [];
  const cloud = x.cloud ?? [];

  return (
    <div className="grid g2 mt">
      <Card title={`${x.name} storage`} icon={<LayoutGrid size={16} />} sub={[x.model, x.os].filter(Boolean).join(" · ")}>
        {x.totalBytes ? (
          <>
            <div className="row" style={{ justifyContent: "space-between", alignItems: "baseline" }}>
              <div style={{ fontSize: 24, fontWeight: 650 }} className="num">{bytes(x.totalBytes - x.freeBytes)} <span className="muted" style={{ fontSize: 14, fontWeight: 500 }}>used of {bytes(x.totalBytes)}</span></div>
              <div className="muted num">{bytes(x.freeBytes)} free</div>
            </div>
            <div className="mt"><StackedBar slices={withFree(x)} total={x.totalBytes} /></div>
            <Legend slices={(x.breakdown ?? []).concat(x.freeBytes ? [{ key: "free", label: "Free", bytes: x.freeBytes, color: "var(--surface-2)" }] : [])} />
          </>
        ) : <div className="empty">No storage reading for this device yet.</div>}

        {m && (
          <div className="grid g3 mt2" style={{ gap: 10 }}>
            <div className="stat"><div className="muted small">Photos</div><div className="strong num">{m.photos.toLocaleString()}</div><div className="muted small num">{bytes(m.photosBytes)}</div></div>
            <div className="stat"><div className="muted small">Videos</div><div className="strong num">{m.videos.toLocaleString()}</div><div className="muted small num">{bytes(m.videosBytes)}</div></div>
            <div className="stat"><div className="muted small">Screenshots</div><div className="strong num">{m.screenshots.toLocaleString()}</div><div className="muted small num">{bytes(m.screenshotsBytes)}</div></div>
          </div>
        )}
        {m?.estimated && <p className="muted small">Very large library: sizes extrapolated from the files measured.</p>}
      </Card>

      <div className="grid" style={{ alignContent: "start" }}>
        <Card title="Cloud" icon={<Cloud size={16} />} sub="What this device keeps in the cloud">
          {cloud.length ? (
            <table>
              <thead><tr><th>Service</th><th className="r">On device</th><th className="r">Cloud only</th></tr></thead>
              <tbody>
                {cloud.map((l) => (
                  <tr key={l.name}>
                    <td><div className="strong">{l.name}</div><div className="muted small ellipsis" style={{ maxWidth: 280 }} title={l.detail}>{l.detail}</div></td>
                    <td className="r num">{l.localBytes ? bytes(l.localBytes) : "–"}</td>
                    <td className="r num muted">{l.cloudBytes ? bytes(l.cloudBytes) : "–"}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          ) : <div className="muted small">No cloud apps found on this device.</div>}
          {isApple && d.icloud && (
            <p className="muted small" style={{ marginBottom: 0 }}>
              On your iCloud: backups of all your devices use <b>{bytes(icloudBackups)}</b>, the shared Photos library <b>{bytes(icloudPhotos)}</b>.
            </p>
          )}
        </Card>

        {apps.length > 0 && (
          <Card title="Apps" icon={<Smartphone size={16} />} sub={`${apps.length} apps · biggest first`}>
            <table>
              <thead><tr><th>App</th><th className="r">App</th><th className="r">Data</th><th className="r">Total</th></tr></thead>
              <tbody>
                {apps.slice(0, 15).map((a) => (
                  <tr key={a.bundleId}>
                    <td className="ellipsis" style={{ maxWidth: 220 }} title={a.bundleId}>{a.name}</td>
                    <td className="r num muted">{bytes(a.appBytes)}</td>
                    <td className="r num muted">{bytes(a.dataBytes)}</td>
                    <td className="r num strong">{bytes(a.appBytes + a.dataBytes)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </Card>
        )}
      </div>
    </div>
  );
}

export default function Devices({ c }: { c: Ctx }) {
  const { d } = c;
  const withStorage = [d.thisDevice, ...d.devices.filter((x) => x.totalBytes > 0)].filter(Boolean) as DeviceSnapshot[];
  const names = new Set(withStorage.map((x) => x.name.toLowerCase()));
  const accountOnly = d.devices.filter((x) => x.totalBytes === 0 && !names.has(x.name.toLowerCase()));
  const all = [...withStorage, ...accountOnly];
  const [sel, setSel] = useState<string | null>(null);
  useEffect(() => { if (!sel && all[0]) setSel(all[0].id); }, [all, sel]);
  const selected = all.find((x) => x.id === sel) ?? all[0];
  const backups = d.local?.iosBackups ?? [];
  const status = d.local?.iosStatus;
  const errs = d.local?.iosErrors ?? [];
  const phones = d.devices.filter((x) => x.source === "Read over USB / Wi-Fi");

  return (
    <>
      {status === "no_service" && (
        <div className="banner">
          <Info size={16} />
          <span>To read iPhone and iPad storage with nothing installed on them, add Apple's free <b>Apple Devices</b> app to this PC, plug your iPhone in once and tap Trust.</span>
          <button className="btn primary" style={{ marginLeft: "auto" }} onClick={() => c.act({ label: "", kind: "open_url", target: "ms-windows-store://search/?query=Apple%20Devices" })}>Get Apple Devices</button>
        </div>
      )}
      {status === "none" && phones.length === 0 && (
        <div className="banner"><Cable size={16} /><span>Plug in your iPhone or iPad and tap Trust. To read it without the cable next time, turn on <b>Show this iPhone when on Wi-Fi</b> in Apple Devices.</span></div>
      )}
      {errs.map((e) => <div key={e} className="banner err">{e}</div>)}

      <div className="grid g3">
        {all.map((x) => <DeviceTile key={x.id} x={x} active={selected?.id === x.id} onClick={() => setSel(x.id)} />)}
      </div>

      {selected && <Detail x={selected} c={c} />}

      <Card title="iPhone and iPad backups on this PC" icon={<Archive size={16} />} sub={`${backups.length} found`} className="mt">
        {backups.length ? (
          <table>
            <thead><tr><th>Device</th><th>iOS</th><th>Last backup</th><th className="r">Size</th></tr></thead>
            <tbody>
              {backups.map((b) => (
                <tr key={b.path}>
                  <td><div className="strong">{b.deviceName}</div><div className="path ellipsis" style={{ maxWidth: 420 }}>{b.path}</div></td>
                  <td>{b.iosVersion ?? "?"}</td>
                  <td>{b.lastBackup ?? "?"}</td>
                  <td className="r num">{bytes(b.bytes)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        ) : <div className="empty"><Image size={14} /> No local iPhone or iPad backups.</div>}
      </Card>
    </>
  );
}
