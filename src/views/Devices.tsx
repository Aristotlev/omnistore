import { Archive, Usb } from "lucide-react";
import { Card, DeviceIcon, UsageBar } from "../components";
import { ago, bytes } from "../format";
import type { DeviceSnapshot } from "../types";
import type { Ctx } from "../ctx";

const KIND: Record<string, string> = {
  windows: "Windows PC", macos: "Mac", ios: "iPhone", ipados: "iPad", android: "Android", apple: "Apple device", linux: "Linux",
};

function DeviceCard({ x }: { x: DeviceSnapshot }) {
  const hasStorage = x.totalBytes > 0;
  return (
    <Card>
      <div className="device">
        <div className="top">
          <div className="icon-chip"><DeviceIcon kind={x.kind} /></div>
          <div style={{ minWidth: 0 }}>
            <div className="name ellipsis">{x.name}</div>
            <div className="muted small ellipsis">{[x.model, x.os].filter(Boolean).join(" · ") || KIND[x.kind]}</div>
          </div>
          <span className="pill info" style={{ marginLeft: "auto" }}>{x.source}</span>
        </div>
        {hasStorage ? (
          <>
            <UsageBar used={x.totalBytes - x.freeBytes} total={x.totalBytes} thick />
            <div className="row small" style={{ justifyContent: "space-between" }}>
              <span className="num">{bytes(x.totalBytes - x.freeBytes)} used</span>
              <span className="muted num">{bytes(x.freeBytes)} free of {bytes(x.totalBytes)}</span>
            </div>
          </>
        ) : (
          <div className="muted small">Seen on your Apple Account. Install OmniStore on it to read its storage.</div>
        )}
        <div className="muted small">Updated {ago(x.updatedAt)}</div>
      </div>
    </Card>
  );
}

export default function Devices({ c }: { c: Ctx }) {
  const { d } = c;
  const withStorage = [d.thisDevice, ...d.devices.filter((x) => x.totalBytes > 0)].filter(Boolean) as DeviceSnapshot[];
  // Apple devices on the account that already report storage via OmniStore are not listed twice.
  const names = new Set(withStorage.map((x) => x.name.toLowerCase()));
  const accountOnly = d.devices.filter((x) => x.totalBytes === 0 && !names.has(x.name.toLowerCase()));
  const backups = d.local?.iosBackups ?? [];
  const usb = d.local?.appleUsbDevices ?? [];

  return (
    <>
      <div className="grid g3">
        {withStorage.map((x) => <DeviceCard key={x.id} x={x} />)}
        {accountOnly.map((x) => <DeviceCard key={x.id} x={x} />)}
      </div>

      {!d.google && (
        <div className="banner mt">Install OmniStore on your Mac, iPhone and iPad and sign in with the same Google account. Each device then appears here with its storage.</div>
      )}

      <div className="grid g2 mt">
        <Card title="iPhone and iPad backups on this PC" icon={<Archive size={16} />} sub={`${backups.length} found`}>
          {backups.length ? (
            <table>
              <thead><tr><th>Device</th><th>iOS</th><th>Last backup</th><th className="r">Size</th></tr></thead>
              <tbody>
                {backups.map((b) => (
                  <tr key={b.path}>
                    <td><div className="strong">{b.deviceName}</div><div className="path ellipsis" style={{ maxWidth: 260 }}>{b.path}</div></td>
                    <td>{b.iosVersion ?? "?"}</td>
                    <td>{b.lastBackup ?? "?"}</td>
                    <td className="r num">{bytes(b.bytes)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          ) : <div className="empty">No local iPhone or iPad backups.</div>}
        </Card>
        <Card title="Connected by USB" icon={<Usb size={16} />} sub="Apple devices plugged into this PC">
          {usb.length ? usb.map((n) => <div key={n} className="row" style={{ padding: "6px 0" }}><DeviceIcon kind={n.includes("iPad") ? "ipados" : "ios"} />{n}</div>)
            : <div className="empty">Plug in an iPhone or iPad and tap Trust to see it here.</div>}
        </Card>
      </div>
    </>
  );
}
