import { Cpu, FolderOpen, HardDrive, RefreshCw, ScanSearch, Cloud, Sparkles } from "lucide-react";
import { Card, Spinner, UsageBar } from "../components";
import { ago, bytes } from "../format";
import type { Ctx } from "../ctx";
import { api } from "../api";

export default function ThisDevice({ c }: { c: Ctx }) {
  const { d, busy } = c;
  const l = d.local;
  const clients = (l?.syncClients ?? []).filter((x) => x.installed || x.syncRoot);
  const open = (path?: string | null) => path && c.act({ label: "", kind: "open_path", target: path });

  return (
    <>
      <div className="row" style={{ marginBottom: 16 }}>
        <button className="btn" disabled={!!busy.local} onClick={() => c.task("local", api.scanLocal)}>
          {busy.local ? <Spinner /> : <RefreshCw size={14} />}Rescan drives
        </button>
        <button className="btn" disabled={!!busy.deep} onClick={() => c.task("deep", api.deepScan)}>
          {busy.deep ? <Spinner /> : <ScanSearch size={14} />}Find large files
        </button>
        <span className="muted small">Last scan {ago(l?.scannedAt)}</span>
      </div>

      <div className="grid g2">
        <Card title="Drives" icon={<HardDrive size={16} />} sub={`${l?.volumes.length ?? 0} volumes`}>
          {l?.volumes.length ? (
            <div className="grid" style={{ gap: 14 }}>
              {l.volumes.map((v) => (
                <div key={v.mount}>
                  <div className="row" style={{ justifyContent: "space-between" }}>
                    <span><b>{v.mount}</b> <span className="muted">{v.name !== v.mount ? v.name : ""} · {v.fileSystem} · {v.kind.toUpperCase()}{v.removable ? " · external" : ""}</span></span>
                    <span className="muted small num">{bytes(v.freeBytes)} free of {bytes(v.totalBytes)}</span>
                  </div>
                  <div style={{ marginTop: 6 }}><UsageBar used={v.totalBytes - v.freeBytes} total={v.totalBytes} /></div>
                </div>
              ))}
            </div>
          ) : <div className="empty">{busy.local ? "Reading drives…" : "No scan yet."}</div>}
        </Card>

        <Card title="Hardware" icon={<Cpu size={16} />} sub="Physical disks">
          {l?.physicalDisks.length ? (
            <table>
              <thead><tr><th>Disk</th><th>Type</th><th>Bus</th><th>Health</th><th className="r">Size</th></tr></thead>
              <tbody>
                {l.physicalDisks.map((p, i) => (
                  <tr key={i}>
                    <td className="strong">{p.name}</td>
                    <td>{p.mediaType}</td>
                    <td>{p.busType}</td>
                    <td><span className={`pill ${p.health === "Healthy" ? "ok" : p.health === "Unknown" ? "" : "critical"}`}>{p.health}</span></td>
                    <td className="r num">{bytes(p.sizeBytes)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          ) : <div className="empty">No disk details available.</div>}
        </Card>
      </div>

      <Card title="Cloud apps on this PC" icon={<Cloud size={16} />} sub="What is downloaded vs online-only" className="mt">
        {clients.length ? (
          <table>
            <thead><tr><th>App</th><th>Folder</th><th className="r">On this PC</th><th className="r">Online-only</th><th className="r">Cache</th><th /></tr></thead>
            <tbody>
              {clients.map((s) => (
                <tr key={s.id}>
                  <td className="strong">{s.name}</td>
                  <td className="path ellipsis" style={{ maxWidth: 320 }}>{s.syncRoot ?? "Streaming drive"}</td>
                  <td className="r num">{bytes(s.localBytes)}</td>
                  <td className="r num muted">{bytes(s.cloudOnlyBytes)}</td>
                  <td className="r num muted">{s.cacheBytes ? bytes(s.cacheBytes) : "–"}</td>
                  <td className="r">{s.syncRoot && <button className="btn ghost" onClick={() => open(s.syncRoot)}><FolderOpen size={14} /></button>}</td>
                </tr>
              ))}
            </tbody>
          </table>
        ) : <div className="empty">No iCloud, Google Drive, OneDrive or Dropbox apps found.</div>}
      </Card>

      <div className="grid g2 mt">
        <Card title="Reclaimable" icon={<Sparkles size={16} />}>
          {l?.reclaimable.length ? (
            <table>
              <tbody>
                {l.reclaimable.map((r) => (
                  <tr key={r.id}>
                    <td>{r.label}</td>
                    <td className="r num">{bytes(r.bytes)}</td>
                    <td className="r">
                      {r.id === "recycle_bin"
                        ? <button className="btn" disabled={!!busy["act:empty_recycle_bin"] || r.bytes === 0} onClick={() => c.act({ label: "", kind: "empty_recycle_bin" }, "act:empty_recycle_bin")}>
                            {busy["act:empty_recycle_bin"] && <Spinner />}{r.bytes === 0 ? "Empty" : "Empty now"}
                          </button>
                        : r.path && <button className="btn ghost" onClick={() => open(r.path)}><FolderOpen size={14} /></button>}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          ) : <div className="empty">Nothing found.</div>}
        </Card>
        <Card title="Largest files" icon={<ScanSearch size={16} />} sub={`Over ${c.settings?.largeFileMinMb ?? 500} MB in your user folder`}>
          {d.largeFiles.length ? (
            <table>
              <tbody>
                {d.largeFiles.slice(0, 40).map((f) => (
                  <tr key={f.path}>
                    <td className="path ellipsis" style={{ maxWidth: 380 }} title={f.path}>{f.path}</td>
                    <td className="muted small">{f.modified}</td>
                    <td className="r num">{bytes(f.bytes)}{f.cloudOnly && <span className="pill info" style={{ marginLeft: 6 }}>cloud</span>}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          ) : <div className="empty">Run "Find large files" to list them.</div>}
        </Card>
      </div>
    </>
  );
}
