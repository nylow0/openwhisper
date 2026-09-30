<script lang="ts">
  // Transcript history: a dotted timeline grouped by day. Clicking a transcript opens
  // it in place with its stats and actions. The newest one starts open.
  import { getContext } from 'svelte';
  import { fade, slide } from 'svelte/transition';
  import { history } from './app-state';
  import { LANGUAGE_BY_ID } from './languages';
  import type { HistoryItem } from '../shared/types';

  const api = window.api;
  const pushToast = getContext<(message: string) => void>('pushToast');

  let query = '';
  let openId: string | null | undefined; // undefined = not chosen yet, newest opens
  let copiedId: string | null = null;
  let copiedTimer: ReturnType<typeof setTimeout> | undefined;

  $: latestId = $history[0]?.id;
  $: if (openId === undefined && latestId) openId = latestId;
  $: needle = query.trim().toLowerCase();
  $: groups = groupByDay(needle ? $history.filter((item) => item.text.toLowerCase().includes(needle)) : $history);

  async function copy(item: HistoryItem) {
    try {
      await navigator.clipboard.writeText(item.text);
      copiedId = item.id;
      if (copiedTimer) clearTimeout(copiedTimer);
      copiedTimer = setTimeout(() => (copiedId = null), 1500);
    } catch (e) {
      pushToast('Failed to copy: ' + (e as Error).message);
    }
  }

  function dayLabel(ms: number): string {
    const day = new Date(ms).setHours(0, 0, 0, 0);
    const today = new Date().setHours(0, 0, 0, 0);
    if (day === today) return 'Today';
    if (day === new Date(today - 86_400_000).setHours(0, 0, 0, 0)) return 'Yesterday';
    return new Date(ms).toLocaleDateString([], { weekday: 'long', month: 'short', day: 'numeric' });
  }

  /** Buckets newest-first items into consecutive days. */
  function groupByDay(items: HistoryItem[]): Array<{ label: string; items: HistoryItem[] }> {
    const result: Array<{ label: string; items: HistoryItem[] }> = [];
    for (const item of items) {
      const label = dayLabel(item.createdAt);
      const last = result[result.length - 1];
      if (last?.label === label) last.items.push(item);
      else result.push({ label, items: [item] });
    }
    return result;
  }

  function stats(item: HistoryItem): Array<{ label: string; value: string }> {
    const language = item.language ? LANGUAGE_BY_ID.get(item.language)?.name ?? item.language.toUpperCase() : '—';
    return [
      { label: 'Language', value: language },
      { label: 'Latency', value: item.latencyMs === null ? '—' : `${item.latencyMs} ms` },
      { label: 'Words', value: String(item.text.split(/\s+/).filter(Boolean).length) },
      { label: 'Characters', value: String(item.text.length) },
    ];
  }
</script>

<div class="flex shrink-0 items-center gap-4 px-8 pb-2 pt-5">
  <input
    type="text"
    placeholder="Search"
    bind:value={query}
    class="flex-1 border-b border-ink-700 bg-transparent py-2 text-[14px] text-white placeholder:text-ink-500 focus:border-blue-500 focus:outline-none"
  />
  {#if $history.length > 0}
    <button type="button" class="text-[13px] text-ink-400 hover:text-white" on:click={() => api?.clearHistory()}>
      Clear all
    </button>
  {/if}
</div>

<div class="scroll-area min-h-0 flex-1 overflow-y-auto px-8 pb-10">
  {#if $history.length === 0}
    <div class="flex items-center gap-6 pt-14" in:fade={{ duration: 150 }}>
      <div class="flex gap-2">
        {#each ['Ctrl', 'Win'] as key}
          <kbd
            class="rounded-xl border border-[#1f3b5c] border-b-[3px] border-b-[#1e4f7a] bg-[#0f1822] px-4 py-2.5 font-sans text-[16px] font-semibold text-sky-300 shadow-[0_0_24px_rgba(34,211,238,0.12)]"
          >
            {key}
          </kbd>
        {/each}
      </div>
      <div>
        <p class="text-[18px] font-semibold">Nothing here yet.</p>
        <p class="mt-1 max-w-[420px] text-[14px] leading-relaxed text-ink-300">
          Hold both keys in any app, speak, and let go. Your words are typed at your cursor and saved here.
        </p>
      </div>
    </div>
  {:else if groups.length === 0}
    <p class="pt-10 text-[14px] text-ink-400">Nothing matches “{query}”.</p>
  {:else}
    {#each groups as group}
      <p class="pb-1 pt-5 text-[12px] font-medium text-ink-500">{group.label}</p>
      {#each group.items as item (item.id)}
        {@const open = openId === item.id}
        <div class="rounded-xl {open ? 'my-2 bg-ink-900 ring-1 ring-ink-700' : ''}">
          <button
            type="button"
            class="flex w-full items-start gap-4 rounded-xl py-2.5 text-left {open ? 'px-4 pt-4' : 'hover:bg-ink-900/60'}"
            aria-expanded={open}
            on:click={() => (openId = open ? null : item.id)}
          >
            <span
              class="mt-[9px] h-[5px] w-[5px] shrink-0 rounded-full {item.id === latestId
                ? 'bg-cyan-400 shadow-[0_0_6px_#22d3ee]'
                : open
                  ? 'bg-blue-500'
                  : 'bg-ink-600'}"
            ></span>
            <span
              class="min-w-0 flex-1 text-[15px] leading-[1.6] {open || item.id === latestId ? 'text-white' : 'text-ink-150'} {open
                ? 'whitespace-pre-wrap break-words'
                : 'line-clamp-2'}"
            >
              {item.text}
            </span>
            <span class="w-20 shrink-0 pt-[3px] text-right text-[13px] tabular-nums text-ink-500">
              {new Date(item.createdAt).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}
            </span>
          </button>
          {#if open}
            <div class="px-4 pb-4 pl-[37px]" transition:slide={{ duration: 180 }}>
              <div class="grid grid-cols-4 gap-6 border-t border-ink-700 pt-3">
                {#each stats(item) as cell}
                  <div>
                    <p class="text-[11px] text-ink-400">{cell.label}</p>
                    <p class="mt-0.5 text-[14px] tabular-nums">{cell.value}</p>
                  </div>
                {/each}
              </div>
              <div class="mt-4 flex gap-2">
                <button
                  type="button"
                  class="rounded-lg px-3.5 py-1.5 text-[13px] font-semibold transition-colors {copiedId === item.id
                    ? 'bg-cyan-950 text-cyan-300'
                    : 'bg-blue-500 text-white hover:bg-blue-400'}"
                  on:click={() => copy(item)}
                >
                  {copiedId === item.id ? 'Copied' : 'Copy'}
                </button>
                <button
                  type="button"
                  class="rounded-lg px-3 py-1.5 text-[13px] text-ink-300 hover:bg-ink-800 hover:text-rose-400"
                  on:click={() => api?.deleteHistoryItem(item.id)}
                >
                  Delete
                </button>
              </div>
            </div>
          {/if}
        </div>
      {/each}
    {/each}
  {/if}
</div>
