/**
 * TypeScript types matching Rust backend structures
 */

export interface WindowData {
  utilization: number; // percentage 0–100 (already scaled by API)
  resets_at: string; // ISO 8601
}

export interface ExtraUsageData {
  is_enabled: boolean;
  used_credits: number; // e.g. 2.61 USD
  monthly_limit: number; // e.g. 20.00 USD
  utilization: number; // percentage 0–100
  currency: string;
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
