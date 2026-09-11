<script lang="ts">
  import { onMount } from "svelte";
  import { hexRgb } from "./palettes";
  import { startSilk, type SilkColors } from "./silk";

  let canvas: HTMLCanvasElement;
  let stop: (() => void) | null = null;

  function readCss(name: string, fallback: string) {
    const v = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
    return v || fallback;
  }

  function colors(): SilkColors {
    const dark = document.documentElement.dataset.theme !== "light";
    const field = hexRgb(readCss("--shell", "#06070a"));
    const wire = hexRgb(readCss("--wire", dark ? "#141720" : "#202430"));
    return { field, wire };
  }

  onMount(() => {
    stop = startSilk(canvas, colors);
    return () => stop?.();
  });
</script>

<canvas bind:this={canvas} class="silk" aria-hidden="true"></canvas>

<style>
  .silk {
    position: fixed;
    inset: 0;
    width: 100%;
    height: 100%;
    z-index: 0;
    pointer-events: none;
    display: block;
    opacity: 1;
  }
</style>
