<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import UsagePanel from "../lib/UsagePanel.svelte";
  import { formatLastUpdated, isStale } from "../lib/countdown";
  import type { UsageSnapshot } from "../lib/types";

  let snapshot: UsageSnapshot | null = null;
  let isRefreshing = false;
  let authError = false;
  let lastUpdated: string | null = null;

  onMount(async () => {
    // Load initial cached snapshot
    try {
      snapshot = await invoke<UsageSnapshot>("get_snapshot");
      if (snapshot) {
        lastUpdated = snapshot.fetched_at;
      }
    } catch (e) {
      console.error("Failed to get snapshot:", e);
    }

    // Listen for usage updates
    const unlistenUpdate = await listen<UsageSnapshot>("usage_updated", (event) => {
      snapshot = event.payload;
      lastUpdated = snapshot.fetched_at;
      isRefreshing = false;
      authError = false;
    });

    // Listen for auth errors
    const unlistenError = await listen("auth_error", () => {
      authError = true;
      isRefreshing = false;
    });

    return () => {
      unlistenUpdate();
      unlistenError();
    };
  });

  async function handleRefresh() {
    if (isRefreshing) return;
    isRefreshing = true;

    try {
      const result = await invoke<UsageSnapshot>("refresh_now");
      snapshot = result;
      lastUpdated = result.fetched_at;
      authError = false;
    } catch (e) {
      console.error("Refresh failed:", e);
      authError = true;
    } finally {
      isRefreshing = false;
    }
  }

  $: stale = isStale(lastUpdated);
</script>

<main>
  <div class="container">
    <header>
      <h1>Claude Usage</h1>
      <button on:click={handleRefresh} disabled={isRefreshing} class="refresh-btn">
        {#if isRefreshing}
          <span class="spinner" />
          Refreshing...
        {:else}
          🔄 Refresh
        {/if}
      </button>
    </header>

    {#if authError}
      <div class="error-box">
        <p>
          <strong>Authentication Required</strong>
        </p>
        <p>Run the Claude CLI to log in:</p>
        <code>claude login</code>
      </div>
    {:else if snapshot}
      <div class="panels">
        <UsagePanel label="5-Hour Window" data={snapshot.five_hour} />
        <UsagePanel label="7-Day Window" data={snapshot.seven_day} />

        {#if snapshot.extra_usage?.is_enabled}
          <div class="extra-panel">
            <h3>Extra Usage</h3>
            <div class="extra-content">
              <div class="extra-row">
                <span>Credits Used:</span>
                <span class="value">{snapshot.extra_usage.currency ?? "USD"} {snapshot.extra_usage.used_credits.toFixed(2)}</span>
              </div>
              <div class="extra-row">
                <span>Monthly Limit:</span>
                <span class="value">{snapshot.extra_usage.currency ?? "USD"} {snapshot.extra_usage.monthly_limit.toFixed(2)}</span>
              </div>
              <div class="extra-row">
                <span>Utilization:</span>
                <span class="value">{Math.round(snapshot.extra_usage.utilization)}%</span>
              </div>
            </div>
          </div>
        {/if}
      </div>

      <footer>
        <div class="status">
          <span class="label">Updated:</span>
          <span class="value">{formatLastUpdated(lastUpdated)}</span>
          {#if stale}
            <span class="stale-badge">stale</span>
          {/if}
        </div>
      </footer>
    {:else}
      <div class="loading">
        <span class="spinner" />
        <p>Loading usage data...</p>
      </div>
    {/if}
  </div>
</main>

<style>
  :global(body) {
    margin: 0;
    padding: 0;
    background: #0f0f0f;
    color: #e0e0e0;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue",
      Arial, sans-serif;
  }

  main {
    width: 100%;
    height: 100vh;
    display: flex;
    align-items: center;
    justify-content: center;
    background: #0f0f0f;
  }

  .container {
    width: 100%;
    max-width: 420px;
    padding: 24px;
    background: linear-gradient(135deg, #1a1a1a 0%, #0f0f0f 100%);
    border-radius: 12px;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 24px;
    gap: 12px;
  }

  h1 {
    margin: 0;
    font-size: 28px;
    font-weight: 700;
    background: linear-gradient(135deg, #60a5fa, #8b5cf6);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
  }

  .refresh-btn {
    padding: 8px 12px;
    background: #333;
    color: #e0e0e0;
    border: 1px solid #444;
    border-radius: 4px;
    cursor: pointer;
    font-size: 12px;
    font-weight: 600;
    display: flex;
    align-items: center;
    gap: 6px;
    transition: all 0.2s;
  }

  .refresh-btn:hover:not(:disabled) {
    background: #444;
    border-color: #555;
  }

  .refresh-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .spinner {
    display: inline-block;
    width: 14px;
    height: 14px;
    border: 2px solid #555;
    border-top-color: #60a5fa;
    border-radius: 50%;
    animation: spin 0.6s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .error-box {
    background: #7f1d1d;
    border: 1px solid #991b1b;
    border-radius: 8px;
    padding: 16px;
    margin-bottom: 16px;
  }

  .error-box p {
    margin: 8px 0;
    font-size: 14px;
  }

  .error-box code {
    background: #0f0f0f;
    padding: 4px 8px;
    border-radius: 4px;
    font-family: "Monaco", "Courier New", monospace;
    font-size: 12px;
    color: #60a5fa;
  }

  .panels {
    display: flex;
    flex-direction: column;
    gap: 0;
  }

  .extra-panel {
    border: 1px solid #333;
    border-radius: 8px;
    padding: 16px;
    margin-top: 12px;
    background: #1a1a1a;
  }

  .extra-panel h3 {
    margin: 0 0 12px 0;
    font-size: 12px;
    font-weight: 600;
    color: #888;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .extra-content {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .extra-row {
    display: flex;
    justify-content: space-between;
    font-size: 13px;
  }

  .extra-row span:first-child {
    color: #888;
  }

  .value {
    color: #e0e0e0;
    font-weight: 600;
  }

  footer {
    margin-top: 16px;
    padding-top: 12px;
    border-top: 1px solid #222;
    font-size: 12px;
    text-align: right;
  }

  .status {
    display: flex;
    justify-content: flex-end;
    align-items: center;
    gap: 8px;
    color: #888;
  }

  .label {
    color: #666;
  }

  .stale-badge {
    background: #7f1d1d;
    color: #fca5a5;
    padding: 2px 6px;
    border-radius: 3px;
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
  }

  .loading {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    padding: 40px 0;
  }

  .loading p {
    margin: 0;
    color: #888;
  }
</style>
