export interface CheckProgressPayload {
  files_checked: number;
  total_files: number;
  bytes_processed: number;
  total_bytes: number;
  current_file: string;
  percent: number;
}

export interface DownloadProgressPayload {
  bytes_downloaded: number;
  total_bytes: number;
  percent: number;
  current_file: string;
}

export interface StatusChangePayload {
  message: string;
}

export interface ErrorPayload {
  message: string;
}

export type SyncPhase =
  | 'idle'
  | 'manifest'
  | 'checking'
  | 'downloading'
  | 'complete'
  | 'launching'
  | 'error';

export interface ModInfo {
  name: string;
  enabled: boolean;
}
