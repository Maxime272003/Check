<script>
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import Icon from './Icon.svelte';
  import Logo from './Logo.svelte';
  import ChecklistView from './ChecklistView.svelte';
  import Settings from './Settings.svelte';
  import ListModal from './ListModal.svelte';
  import { store } from './state.svelte.js';

  const win = getCurrentWindow();
  let screen = $state('list');
  let activeId = $state(store.data.lists[0]?.id ?? null);
  // null | { mode: 'create' } | { mode: 'edit' }
  let modal = $state(null);

  const active = $derived(store.data.lists.find((l) => l.id === activeId) ?? store.data.lists[0] ?? null);

  // Thème
  let systemDark = $state(window.matchMedia('(prefers-color-scheme: dark)').matches);
  $effect(() => {
    const t = store.data.settings.theme;
    const dark = t === 'dark' || (t === 'system' && systemDark);
    document.documentElement.dataset.theme = dark ? 'dark' : 'light';
  });

  onMount(() => {
    const mq = window.matchMedia('(prefers-color-scheme: dark)');
    const onMq = (e) => (systemDark = e.matches);
    mq.addEventListener('change', onMq);
    const uns = [
      listen('ui-action', (e) => {
        if (e.payload === 'new') modal = { mode: 'create' };
        if (e.payload === 'settings') screen = 'settings';
      }),
      listen('focus-list', (e) => {
        activeId = e.payload;
        screen = 'list';
      })
    ];
    return () => {
      mq.removeEventListener('change', onMq);
      uns.forEach((u) => u.then((f) => f()));
    };
  });

  function open(id) {
    activeId = id;
    screen = 'list';
  }
</script>

<div class="window">
  <div class="titlebar" data-tauri-drag-region>
    <div class="brand" data-tauri-drag-region>
      <Logo size={22} />
      <span data-tauri-drag-region>check!</span>
    </div>
    <div style="display: flex">
      <button class="wc" aria-label="Réduire" onclick={() => win.minimize()}><Icon name="minus" size={14} stroke={1.8} /></button>
      <button class="wc" aria-label="Agrandir" onclick={() => win.toggleMaximize()}><Icon name="square" size={13} stroke={1.8} /></button>
      <button class="wc close" aria-label="Fermer" onclick={() => win.close()}><Icon name="x" size={14} stroke={1.8} /></button>
    </div>
  </div>

  <div class="body">
    <nav class="sidebar" aria-label="Checklists">
      <button class="btn primary" style="width: 100%" onclick={() => (modal = { mode: 'create' })}>
        <Icon name="plus" size={16} stroke={2.6} />Nouvelle checklist
      </button>
      <p class="label" style="padding: 18px 10px 6px">Mes checklists</p>
      <div class="lists">
        {#each store.data.lists as l (l.id)}
          <button class="nav" class:on={screen === 'list' && active?.id === l.id} onclick={() => open(l.id)}>
            <span class="dot" style="background: {l.color}"></span>
            <span class="name">{l.name}</span>
            {#if l.pinned}<Icon name="pin" size={13} stroke={2.2} label="Posée sur le bureau" />{/if}
            <span class="count">{l.items.filter((i) => i.done).length}/{l.items.length}</span>
          </button>
        {/each}
      </div>
      {#if store.data.settings.paused}
        <div class="paused-banner"><Icon name="pause" size={14} stroke={2.4} />Remises à zéro suspendues</div>
      {/if}
      <button class="nav" class:on={screen === 'settings'} onclick={() => (screen = 'settings')}>
        <Icon name="sliders" size={16} />
        <span class="name">Paramètres</span>
      </button>
    </nav>

    <main class="pane">
      {#if screen === 'settings'}
        <Settings />
      {:else if active}
        <ChecklistView list={active} onedit={() => (modal = { mode: 'edit' })} />
      {:else}
        <div class="list-pane bg bg-aurora" style="--c: #FF3D8B; display: grid; place-items: center; text-align: center">
          <div style="display: flex; flex-direction: column; gap: 14px; align-items: center">
            <Logo size={72} />
            <h1 class="list-title" style="margin: 0">Aucune checklist</h1>
            <button class="btn primary" onclick={() => (modal = { mode: 'create' })}><Icon name="plus" size={16} stroke={2.6} />Créer ma première checklist</button>
          </div>
        </div>
      {/if}

      {#if store.toast}
        <div class="toast-wrap">
          <div class="toast" role="status"><Icon name="check" size={16} stroke={3} />{store.toast}</div>
        </div>
      {/if}
    </main>
  </div>

  {#if modal}
    <ListModal
      list={modal.mode === 'edit' ? active : null}
      onclose={(deleted) => {
        modal = null;
        if (deleted) activeId = store.data.lists[0]?.id ?? null;
      }}
      oncreated={(id) => {
        modal = null;
        open(id);
      }}
    />
  {/if}
</div>
