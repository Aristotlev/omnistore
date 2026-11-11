export type Provider = "icloud" | "google";
export type Status = "ok" | "near" | "critical" | "over";
export type Severity = "critical" | "warn" | "info";

export interface UsageSlice { key: string; label: string; bytes: number; color?: string | null }
export interface PlanMatch { id: string; name: string; bytes: number; monthlyEur: number; priceVerified: boolean; exact: boolean }

export interface CloudAccount {
  provider: Provider;
  accountLabel: string;
  used: number;
  limit: number;
  plan: PlanMatch;
  status: Status;
  overBy: number;
  breakdown: UsageSlice[];
  trashBytes: number;
  familyShared: boolean;
  fetchedAt: string;
  note?: string | null;
}

export interface Volume { name: string; mount: string; fileSystem: string; totalBytes: number; freeBytes: number; kind: string; removable: boolean }

export interface DeviceSnapshot {
  id: string;
  name: string;
  kind: string;
  model?: string | null;
  os?: string | null;
  totalBytes: number;
  freeBytes: number;
  volumes: Volume[];
  source: string;
  updatedAt: string;
  breakdown?: UsageSlice[];
  topApps?: AppUsage[];
  media?: MediaStats | null;
  cloud?: CloudLink[];
  connection?: string | null;
}

export interface AppUsage { name: string; bundleId: string; appBytes: number; dataBytes: number }
export interface MediaStats { photos: number; photosBytes: number; videos: number; videosBytes: number; screenshots: number; screenshotsBytes: number; estimated: boolean }
export interface CloudLink { name: string; detail: string; localBytes: number; cloudBytes: number }

export interface PhysicalDisk { name: string; mediaType: string; busType: string; health: string; sizeBytes: number }
export interface SyncClient { id: string; name: string; installed: boolean; syncRoot?: string | null; localBytes: number; cloudOnlyBytes: number; cacheBytes: number; fileCount: number }
export interface Reclaimable { id: string; label: string; path?: string | null; bytes: number }
export interface IosBackup { deviceName: string; productType?: string | null; iosVersion?: string | null; lastBackup?: string | null; path: string; bytes: number }

export interface LocalReport {
  volumes: Volume[];
  physicalDisks: PhysicalDisk[];
  syncClients: SyncClient[];
  reclaimable: Reclaimable[];
  iosBackups: IosBackup[];
  appleUsbDevices: string[];
  scannedAt: string;
  iosStatus?: string;
  iosErrors?: string[];
}

export interface FileEntry { path: string; bytes: number; modified?: string | null; cloudOnly: boolean }
export interface Action { label: string; kind: string; target?: string | null }
export interface Finding { id: string; severity: Severity; title: string; detail: string; bytes: number; monthlyEur: number; action?: Action | null }

export interface Totals {
  cloudUsed: number;
  cloudCapacity: number;
  localUsed: number;
  localCapacity: number;
  monthlySpendEur: number;
  reclaimableBytes: number;
  potentialSavingsEur: number;
}

export interface HistoryPoint { date: string; icloudUsed?: number | null; googleUsed?: number | null; localUsed?: number | null }

export interface Dashboard {
  thisDevice?: DeviceSnapshot | null;
  local?: LocalReport | null;
  icloud?: CloudAccount | null;
  google?: CloudAccount | null;
  devices: DeviceSnapshot[];
  largeFiles: FileEntry[];
  findings: Finding[];
  totals: Totals;
  history: HistoryPoint[];
  updatedAt?: string | null;
  syncFolder?: string | null;
}

export interface Settings {
  deviceId: string;
  deviceName: string;
  icloudConnected: boolean;
  googleEmail?: string | null;
  largeFileMinMb: number;
}
