<script>
  import { onMount } from 'svelte';
  import { store, init } from './lib/state.svelte.js';
  import MainWindow from './lib/MainWindow.svelte';
  import Widget from './lib/Widget.svelte';

  const widgetId = window.__CHECK_WIDGET__ ?? new URLSearchParams(location.search).get('widget');
  if (widgetId) document.documentElement.classList.add('is-widget');

  // Thème appliqué à toutes les fenêtres (fenêtre principale et post-it).
  let systemDark = $state(window.matchMedia('(prefers-color-scheme: dark)').matches);
  $effect(() => {
    const t = store.data?.settings.theme ?? 'system';
    const dark = t === 'dark' || (t === 'system' && systemDark);
    document.documentElement.dataset.theme = dark ? 'dark' : 'light';
  });

  onMount(() => {
    init();
    const mq = window.matchMedia('(prefers-color-scheme: dark)');
    const onMq = (e) => (systemDark = e.matches);
    mq.addEventListener('change', onMq);
    return () => mq.removeEventListener('change', onMq);
  });
</script>

{#if store.data}
  {#if widgetId}
    <Widget id={widgetId} />
  {:else}
    <MainWindow />
  {/if}
{/if}
