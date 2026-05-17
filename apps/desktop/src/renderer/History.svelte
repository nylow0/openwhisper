<script lang="ts">
  import { getContext, onDestroy, onMount } from 'svelte';
  import { fly } from 'svelte/transition';
  import { flip } from 'svelte/animate';
  import type { HistoryItem } from '../shared/types';

  const api = window.api;
  const pushToast = getContext<(message: string) => void>('pushToast');

  let items: HistoryItem[] = [];
  let query = '';
  let copiedId: string | null = null;
  let copiedTimer: ReturnType<typeof setTimeout> | undefined;
  let unsubHistory: (() => void) | undefined;

  $: needle = query.trim().toLowerCase();
  $: filtered = needle ? items.filter((item) => item.text.toLowerCase().includes(needle)) : items;

  onMount(async () => {
    if (!api) return;
    unsubHistory = api.onHistoryChanged((next: HistoryItem[]) => {
      items = next;
    });
    items = await api.getHistory();
  });

  onDestroy(() => {
    unsubHistory?.();
    if (copiedTimer) clearTimeout(copiedTimer);
  });

  async function copyItem(item: HistoryItem) {
    try {
      await navigator.clipboard.writeText(item.text);
      copiedId = item.id;
      if (copiedTimer) clearTimeout(copiedTimer);
      copiedTimer = setTimeout(() => {
        copiedId = null;
      }, 1500);
    } catch (e) {
      pushToast('Failed to copy: ' + (e as Error).message);
    }
  }

  async function deleteItem(item: HistoryItem) {
    await api?.deleteHistoryItem(item.id);
  }

  async function clearAll() {
    await api?.clearHistory();
  }

  function formatWhen(ms: number) {
    const date = new Date(ms);
    const time = date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
    if (date.toDateString() === new Date().toDateString()) return time;
    return `${date.toLocaleDateString([], { month: 'short', day: 'numeric' })} · ${time}`;
  }
</script>

<div class="flex h-full flex-col">
  <!-- Header + search -->
  <div class="shrink-0 px-7 pt-6">
    <h1 class="text-lg font-semibold text-zinc-100">History</h1>
    <p class="mt-0.5 text-[12px] text-zinc-500">
      Everything you've dictated. Hold Ctrl + Win anywhere to add more.
    </p>
    <div class="mt-4 flex items-center gap-2.5">
      <div class="relative flex-1">
        <svg
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
          class="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-zinc-600"
          aria-hidden="true"
        >
          <circle cx="11" cy="11" r="7" />
          <path d="m21 21-4.3-4.3" />
        </svg>
        <input
          type="text"
          placeholder="Search transcripts…"
          bind:value={query}
          class="w-full rounded-lg border border-zinc-800 bg-zinc-900 py-2 pl-9 pr-3 text-[13px] text-zinc-100 placeholder:text-zinc-600 focus:border-indigo-500/50 focus:outline-none"
        />
      </div>
      <button
        type="button"
        class="shrink-0 rounded-lg border border-zinc-800 px-3 py-2 text-[12px] font-medium text-zinc-400 transition-colors hover:bg-zinc-800 hover:text-zinc-200 disabled:pointer-events-none disabled:opacity-40"
        on:click={clearAll}
        disabled={items.length === 0}
      >
        Clear all
      </button>
    </div>
  </div>

  <!-- Transcript list -->
  <div class="scroll-area mt-4 min-h-0 flex-1 space-y-2 overflow-y-auto px-7 pb-6">
    {#if items.length === 0}
      <div class="flex min-h-[260px] flex-col items-center justify-center gap-3 text-center">
        <div class="flex h-14 w-14 items-center justify-center rounded-2xl border border-zinc-800 bg-zinc-900 text-zinc-700">
          <svg viewBox="0 0 24 24" fill="none" class="h-6 w-6" aria-hidden="true">
            <circle cx="12" cy="12" r="2.4" fill="currentColor" />
            <path d="M7.5 7.5a6.4 6.4 0 0 0 0 9" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" />
            <path d="M16.5 7.5a6.4 6.4 0 0 1 0 9" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" />
            <path d="M4.4 4.4a10.6 10.6 0 0 0 0 15.2" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" opacity="0.45" />
            <path d="M19.6 4.4a10.6 10.6 0 0 1 0 15.2" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" opacity="0.45" />
          </svg>
        </div>
        <div>
          <p class="text-[13px] font-medium text-zinc-400">No transcripts yet</p>
          <p class="mt-0.5 text-[12px] text-zinc-600">
            Hold Ctrl + Win, speak, and release — your words appear here.
          </p>
        </div>
      </div>
    {:else if filtered.length === 0}
      <div class="flex min-h-[200px] items-center justify-center text-center">
        <p class="text-[13px] text-zinc-500">No transcripts match “{query}”.</p>
      </div>
    {:else}
      {#each filtered as item (item.id)}
        <article
          in:fly={{ y: 8, duration: 200 }}
          animate:flip={{ duration: 200 }}
          class="group rounded-xl border border-zinc-800 bg-zinc-900 p-3.5 transition-colors hover:border-zinc-700"
        >
          <div class="mb-1.5 flex items-center justify-between gap-2">
            <div class="flex items-center gap-2 text-[11px] text-zinc-500">
              <span class="tabular-nums">{formatWhen(item.createdAt)}</span>
              {#if item.language}
                <span class="rounded bg-zinc-800 px-1.5 py-0.5 font-medium uppercase text-zinc-400">
                  {item.language}
                </span>
              {/if}
              {#if item.latencyMs !== null}
                <span class="tabular-nums">{item.latencyMs} ms</span>
              {/if}
            </div>
            <div class="flex items-center gap-0.5 opacity-70 transition-opacity group-hover:opacity-100">
              <button
                type="button"
                class="inline-flex items-center gap-1 rounded-md px-1.5 py-1 text-[11px] font-medium transition-colors hover:bg-zinc-800 {copiedId ===
                item.id
                  ? 'text-emerald-400'
                  : 'text-zinc-400 hover:text-zinc-100'}"
                on:click={() => copyItem(item)}
              >
                {#if copiedId === item.id}
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" class="h-3.5 w-3.5" aria-hidden="true">
                    <path d="M20 6 9 17l-5-5" />
                  </svg>
                  Copied
                {:else}
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="h-3.5 w-3.5" aria-hidden="true">
                    <rect x="9" y="9" width="11" height="11" rx="2.5" />
                    <path d="M5 15V5.5A2.5 2.5 0 0 1 7.5 3H16" />
                  </svg>
                  Copy
                {/if}
              </button>
              <button
                type="button"
                class="rounded-md p-1 text-zinc-500 transition-colors hover:bg-zinc-800 hover:text-rose-400"
                on:click={() => deleteItem(item)}
                aria-label="Delete transcript"
              >
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="h-3.5 w-3.5" aria-hidden="true">
                  <path d="M3 6h18M8 6V4a1 1 0 0 1 1-1h6a1 1 0 0 1 1 1v2m2 0v14a1 1 0 0 1-1 1H7a1 1 0 0 1-1-1V6" />
                </svg>
              </button>
            </div>
          </div>
          <p class="whitespace-pre-wrap break-words text-[14px] leading-relaxed text-zinc-100">
            {item.text}
          </p>
        </article>
      {/each}
    {/if}
  </div>
</div>
