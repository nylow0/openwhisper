<script lang="ts">
  // PROTOTYPE: whole-app redesign directions, switchable via ?variant=A..F.
  // E and F are A + D mixes on a near-black, blue/cyan palette (requested after round one).
  // Left: the main window at its real default size (880×720). Right: the recording
  // overlay window (420×160, transparent) in all four phases, over a fake app backdrop.
  import { onDestroy } from 'svelte';
  import type { ComponentType } from 'svelte';
  import Switcher from './Switcher.svelte';
  import { OVERLAY_PHASES, OVERLAY_SAMPLE, setHistorySample, simulateError } from './mock';
  import VariantA from './VariantA.svelte';
  import OverlayA from './OverlayA.svelte';
  import VariantB from './VariantB.svelte';
  import OverlayB from './OverlayB.svelte';
  import VariantC from './VariantC.svelte';
  import OverlayC from './OverlayC.svelte';
  import VariantD from './VariantD.svelte';
  import OverlayD from './OverlayD.svelte';
  import VariantE from './VariantE.svelte';
  import OverlayE from './OverlayE.svelte';
  import VariantF from './VariantF.svelte';
  import OverlayF from './OverlayF.svelte';

  const VARIANTS: Array<{ key: string; name: string; pitch: string; window: ComponentType; overlay: ComponentType }> = [
    {
      key: 'A',
      name: 'Ledger',
      pitch: 'No chrome, no cards. The title is the navigation, lists are plain lines on black, settings read like a document.',
      window: VariantA,
      overlay: OverlayA,
    },
    {
      key: 'B',
      name: 'Console',
      pitch: 'Keyboard-first and dense. Monospace table, command bar, config-file settings, a status line that is always visible.',
      window: VariantB,
      overlay: OverlayB,
    },
    {
      key: 'C',
      name: 'Stage',
      pitch: 'Your words are the interface. Editorial serif feed, one big prompt, settings tucked into a side sheet.',
      window: VariantC,
      overlay: OverlayC,
    },
    {
      key: 'D',
      name: 'Split',
      pitch: 'Two-pane like a mail client. List on the left, full transcript on the right, settings inline with no modals.',
      window: VariantD,
      overlay: OverlayD,
    },
    {
      key: 'E',
      name: 'Ledger Split (A + D)',
      pitch: "D's two panes in A's voice: title-as-nav and status sentence on top, dotted list left, detail right. Near-black, blue/cyan.",
      window: VariantE,
      overlay: OverlayE,
    },
    {
      key: 'F',
      name: 'Ledger Plus (A + D)',
      pitch: "A's single column with D folded in: transcripts open into a detail block, settings get section tabs. Near-black, blue/cyan.",
      window: VariantF,
      overlay: OverlayF,
    },
  ];

  let variant = new URLSearchParams(location.search).get('variant')?.toUpperCase() ?? 'A';
  $: active = VARIANTS.find((item) => item.key === variant) ?? VARIANTS[0];

  let empty = false;
  let lightBackdrop = true;

  // Recording timer loops so the live phase keeps moving.
  let elapsedMs = 0;
  const timer = setInterval(() => (elapsedMs = (elapsedMs + 200) % 15_000), 200);
  onDestroy(() => clearInterval(timer));

  function changeVariant(key: string) {
    variant = key;
    const url = new URL(location.href);
    url.searchParams.set('variant', key);
    history.replaceState(null, '', url);
  }
</script>

<div class="scroll-area h-screen overflow-auto bg-black px-10 pb-28 pt-8 text-white">
  <div class="mx-auto flex w-fit max-w-full flex-wrap items-start gap-10">
    <section>
      <p class="font-sans text-[12px] text-neutral-500">
        Variant {active.key} · <span class="text-white">{active.name}</span>
      </p>
      <p class="mb-4 mt-1 max-w-[880px] font-sans text-[13px] text-neutral-400">{active.pitch}</p>
      <div class="relative h-[720px] w-[880px] overflow-hidden rounded-lg ring-1 ring-neutral-800">
        {#key active.key}
          <svelte:component this={active.window} />
        {/key}
      </div>
    </section>

    <section class="w-[420px]">
      <p class="font-sans text-[12px] text-neutral-500">Recording overlay</p>
      <p class="mb-4 mt-1 font-sans text-[13px] text-neutral-400">Floats over whatever app you are typing into.</p>
      <div class="flex flex-col gap-3">
        {#each OVERLAY_PHASES as phase}
          <div>
            <p class="mb-1 font-sans text-[11px] uppercase tracking-wider text-neutral-600">{phase}</p>
            <div
              class="relative h-[160px] w-[420px] overflow-hidden rounded-lg ring-1 ring-neutral-800 {lightBackdrop
                ? 'bg-[#f3f3f3]'
                : 'bg-[#1f1f1f]'}"
            >
              <!-- Fake document lines behind the transparent overlay window. -->
              <div class="absolute inset-0 flex flex-col gap-2.5 p-5">
                {#each [92, 78, 85, 60, 88, 70] as width}
                  <div
                    class="h-2 rounded-full {lightBackdrop ? 'bg-black/10' : 'bg-white/10'}"
                    style="width: {width}%"
                  ></div>
                {/each}
              </div>
              <div class="absolute inset-0">
                <svelte:component
                  this={active.overlay}
                  {phase}
                  {elapsedMs}
                  partial={OVERLAY_SAMPLE.partial}
                  finalText={OVERLAY_SAMPLE.finalText}
                  errorText={OVERLAY_SAMPLE.errorText}
                />
              </div>
            </div>
          </div>
        {/each}
      </div>
    </section>
  </div>
</div>

<Switcher
  variants={VARIANTS}
  current={active.key}
  {empty}
  {lightBackdrop}
  on:change={(event) => changeVariant(event.detail)}
  on:toggleEmpty={() => {
    empty = !empty;
    setHistorySample(empty);
  }}
  on:toggleBackdrop={() => (lightBackdrop = !lightBackdrop)}
  on:error={simulateError}
/>
