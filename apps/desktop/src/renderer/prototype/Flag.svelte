<script lang="ts">
  // PROTOTYPE: renders a language flag via flag-icons, or a custom SVG for regions without an ISO code.
  import type { LanguageOption } from './mock';

  export let language: LanguageOption;
  export let size = 12;
  export let square = false;
</script>

{#if language.flagImage}
  <img
    src={language.flagImage}
    alt=""
    class="shrink-0 object-cover"
    style="width: {square ? size : Math.round(size * 1.33)}px; height: {size}px; border-radius: {square ? 999 : 2}px;"
  />
{:else if language.flag}
  <span
    class="fi fi-{language.flag} shrink-0 {square ? 'fis' : ''}"
    style="font-size: {size}px; width: {square ? `${size}px` : ''}; line-height: {size}px; border-radius: {square ? 999 : 2}px;"
    aria-hidden="true"
  ></span>
{/if}
