/**
 * TypeScript types matching Rust backend structures
 */

export interface WindowData {
  utilization: number; // 0.0-1.0
  resets_at: string; // ISO 8601
}

export interface ExtraUsageData {
  enabled: boolean;
  used_credits: number;
  monthly_limit: number;
  utilization: number;
}

export interface UsageSnapshot {
  five_hour: WindowData;
  seven_day: WindowData;
  extra_usage: ExtraUsageData | null;
  fetched_at: string; // ISO 8601
}

export interface AppState {
  snapshot: UsageSnapshot | null;
  is_refreshing: boolean;
  last_refreshed: string | null; // ISO 8601
  auth_error: boolean;
}
