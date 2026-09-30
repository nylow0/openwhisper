<script lang="ts">
  // A language's flag: a custom SVG when the region has no ISO code, else flag-icons.
  import 'flag-icons/css/flag-icons.min.css';
  import type { LanguageOption } from './languages';

  export let language: LanguageOption;
</script>

{#if language.flagImage}
  <img src={language.flagImage} alt="" class="h-3 w-4 shrink-0 rounded-[2px] object-cover ring-1 ring-inset ring-white/10" />
{:else if language.flag}
  <span class="fi fi-{language.flag} shrink-0 rounded-[2px] text-[12px] ring-1 ring-inset ring-white/10" aria-hidden="true"></span>
{:else}
  <span class="h-3 w-4 shrink-0 rounded-[2px] bg-ink-700" aria-hidden="true"></span>
{/if}
