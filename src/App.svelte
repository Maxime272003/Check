<script>
  import { onMount } from 'svelte';
  import { store, init } from './lib/state.svelte.js';
  import MainWindow from './lib/MainWindow.svelte';
  import Widget from './lib/Widget.svelte';

  const widgetId = new URLSearchParams(location.search).get('widget');
  if (widgetId) document.documentElement.classList.add('is-widget');

  onMount(init);
</script>

{#if store.data}
  {#if widgetId}
    <Widget id={widgetId} />
  {:else}
    <MainWindow />
  {/if}
{/if}
