<script lang="ts">
  import { onMount } from "svelte";
  import { formatCountdown } from "./countdown";
  import type { WindowData } from "./types";

  export let label: string;
  export let data: WindowData;

  let countdown: string = "";

  function updateCountdown() {
    countdown = formatCountdown(data.resets_at);
  }

  onMount(() => {
    updateCountdown();
    const interval = setInterval(updateCountdown, 30000); // Update every 30s
    return () => clearInterval(interval);
  });

  $: data && updateCountdown(); // Re-run when data changes

  // utilization is already 0–100 from the API
  $: percentage = Math.min(100, Math.max(0, Math.round(data.utilization)));

  // Color based on percentage (reactive)
  $: bgColor = percentage >= 90 ? "#ef4444" : percentage >= 70 ? "#eab308" : "#22c55e";
</script>

<div class="panel">
  <div class="header">
    <h3>{label}</h3>
  </div>

  <div class="content">
    <div class="progress-container">
      <div class="progress-bar" style="--fill: {percentage}%; --color: {bgColor};">
        <div class="progress-text">{percentage}%</div>
      </div>
    </div>

    <div class="countdown">{countdown}</div>
  </div>
</div>

<style>
  .panel {
    border: 1px solid #333;
    border-radius: 8px;
    padding: 16px;
    margin-bottom: 12px;
    background: #1a1a1a;
  }

  .header {
    margin-bottom: 12px;
  }

  h3 {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
    color: #888;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .content {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .progress-container {
    position: relative;
    height: 32px;
    background: #0a0a0a;
    border-radius: 4px;
    overflow: hidden;
    border: 1px solid #222;
  }

  .progress-bar {
    position: relative;
    height: 100%;
    width: var(--fill);
    background: var(--color);
    transition: width 0.3s ease;
    display: flex;
    align-items: center;
    justify-content: flex-end;
    padding-right: 8px;
    min-width: 40px;
  }

  .progress-text {
    color: white;
    font-weight: 600;
    font-size: 14px;
    text-shadow: 0 1px 2px rgba(0, 0, 0, 0.5);
  }

  .countdown {
    font-size: 12px;
    color: #888;
    text-align: right;
  }
</style>
