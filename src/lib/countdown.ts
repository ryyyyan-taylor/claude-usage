/**
 * Format a countdown timer from ISO 8601 timestamp
 */
export function formatCountdown(resetsAt: string): string {
  try {
    const resetTime = new Date(resetsAt).getTime();
    const nowTime = Date.now();
    const diffMs = resetTime - nowTime;

    // Already expired
    if (diffMs <= 0) {
      return "Resetting...";
    }

    // Convert to time units
    const totalSeconds = Math.floor(diffMs / 1000);
    const totalMinutes = Math.floor(totalSeconds / 60);
    const totalHours = Math.floor(totalMinutes / 60);
    const totalDays = Math.floor(totalHours / 24);

    // Format based on magnitude
    if (totalDays > 0) {
      const hours = totalHours % 24;
      return `Resets in ${totalDays}d ${hours}h`;
    } else if (totalHours > 0) {
      const minutes = totalMinutes % 60;
      return `Resets in ${totalHours}h ${minutes}m`;
    } else {
      return `Resets in ${totalMinutes}m`;
    }
  } catch (e) {
    return "Time unknown";
  }
}

/**
 * Format a "last updated" timestamp
 */
export function formatLastUpdated(timestamp: string | null): string {
  if (!timestamp) {
    return "Never";
  }

  try {
    const updateTime = new Date(timestamp).getTime();
    const nowTime = Date.now();
    const diffMs = nowTime - updateTime;

    const diffSeconds = Math.floor(diffMs / 1000);
    const diffMinutes = Math.floor(diffSeconds / 60);
    const diffHours = Math.floor(diffMinutes / 60);

    if (diffHours > 0) {
      return `${diffHours}h ago`;
    } else if (diffMinutes > 0) {
      return `${diffMinutes}m ago`;
    } else if (diffSeconds > 0) {
      return `${diffSeconds}s ago`;
    } else {
      return "Just now";
    }
  } catch (e) {
    return "Unknown";
  }
}

/**
 * Check if a snapshot is stale (> 10 minutes old)
 */
export function isStale(lastRefreshed: string | null): boolean {
  if (!lastRefreshed) {
    return true;
  }

  try {
    const lastTime = new Date(lastRefreshed).getTime();
    const nowTime = Date.now();
    const diffMs = nowTime - lastTime;
    return diffMs > 10 * 60 * 1000; // 10 minutes
  } catch (e) {
    return true;
  }
}
