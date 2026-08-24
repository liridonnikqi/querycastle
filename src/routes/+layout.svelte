<script lang="ts">
  import "../app.css";
  import { onMount } from 'svelte';
  import type { Snippet } from 'svelte';

  let { children }: { children: Snippet } = $props();

  onMount(() => {
    // Static #app-splash (header #1c1c1e + icon Loading) is visible from app.html inline HTML
    // Keep it until Svelte app signals ready to avoid white flash between splash and app
    let ready = false;
    let minVisibleDone = false;
    const minVisible = 650;
    const tryHide = () => {
      if (!ready || !minVisibleDone) return;
      if (typeof window !== 'undefined' && (window as any).__hideAppSplash) {
        try { (window as any).__hideAppSplash(); } catch {}
      }
    };
    const onAppReady = () => {
      ready = true;
      tryHide();
    };
    window.addEventListener('app:ready', onAppReady, { once: true });
    setTimeout(() => {
      minVisibleDone = true;
      tryHide();
    }, minVisible);
    // Also hide on load as fallback, but only if app already ready
    const onLoad = () => {
      minVisibleDone = true;
      tryHide();
    };
    if (document.readyState === 'complete') {
      // Defer to next tick to allow +page to mount
      setTimeout(onLoad, 0);
    } else {
      window.addEventListener('load', onLoad, { once: true });
    }
    const fallback = setTimeout(() => {
      ready = true;
      minVisibleDone = true;
      tryHide();
    }, 1800);
    return () => {
      window.removeEventListener('app:ready', onAppReady);
      window.removeEventListener('load', onLoad);
      clearTimeout(fallback);
    };
  });
</script>

{@render children()}
