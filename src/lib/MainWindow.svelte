<script>
  import { onMount } from 'svelte';
  import { flip } from 'svelte/animate';
  import { listen } from '@tauri-apps/api/event';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import Icon from './Icon.svelte';
  import Logo from './Logo.svelte';
  import ChecklistView from './ChecklistView.svelte';
  import Settings from './Settings.svelte';
  import ListModal from './ListModal.svelte';
  import { store, commitAll, flush, runUndo, uncheckAll, togglePin } from './state.svelte.js';
  import { dragReorder } from './reorder.js';

  const win = getCurrentWindow();
  let screen = $state('list');
  let activeId = $state(store.data.lists[0]?.id ?? null);
  // null | { mode: 'create' } | { mode: 'edit' }
  let modal = $state(null);
  let navBox = $state();
  let draggingId = $state(null);

  const active = $derived(store.data.lists.find((l) => l.id === activeId) ?? store.data.lists[0] ?? null);

  onMount(() => {
    const onHide = () => document.visibilityState === 'hidden' && flush();
    document.addEventListener('visibilitychange', onHide);
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
      document.removeEventListener('visibilitychange', onHide);
      uns.forEach((u) => u.then((f) => f()));
    };
  });

  function open(id) {
    activeId = id;
    screen = 'list';
  }
  function step(delta) {
    const lists = store.data.lists;
    if (!lists.length) return;
    const i = Math.max(0, lists.findIndex((l) => l.id === active?.id));
    open(lists[(i + delta + lists.length) % lists.length].id);
  }
  function moveList(from, to) {
    const [l] = store.data.lists.splice(from, 1);
    store.data.lists.splice(to, 0, l);
  }
  function onNavDown(e, l, idx) {
    dragReorder(e, {
      container: navBox,
      from: idx,
      onstart: () => (draggingId = l.id),
      move: moveList,
      end: (moved) => {
        draggingId = null;
        if (moved) commitAll();
      }
    });
  }
  function onNavKey(e, idx) {
    if (!e.altKey || (e.key !== 'ArrowUp' && e.key !== 'ArrowDown')) return;
    e.preventDefault();
    const to = e.key === 'ArrowUp' ? idx - 1 : idx + 1;
    if (to < 0 || to >= store.data.lists.length) return;
    moveList(idx, to);
    commitAll();
  }

  // Raccourcis clavier de la fenêtre (le raccourci global Ctrl+Alt+C est géré côté Rust).
  function onKey(e) {
    if (modal) return; // la fenêtre de création gère ses propres touches
    const typing = e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement;
    const ctrl = e.ctrlKey || e.metaKey;
    const k = e.key.toLowerCase();

    if (ctrl && !e.shiftKey && k === 'z' && !typing) {
      if (runUndo()) e.preventDefault();
      return;
    }
    if (ctrl && !e.shiftKey && k === 'n') {
      e.preventDefault();
      modal = { mode: 'create' };
    } else if (ctrl && (e.key === ',' || k === ';')) {
      e.preventDefault();
      screen = 'settings';
    } else if (ctrl && !e.shiftKey && k === 'e' && active) {
      e.preventDefault();
      screen = 'list';
      modal = { mode: 'edit' };
    } else if (ctrl && !e.shiftKey && k === 'd' && active) {
      e.preventDefault();
      togglePin(active);
    } else if (ctrl && e.shiftKey && k === 'u' && active) {
      e.preventDefault();
      uncheckAll(active);
    } else if (ctrl && /^[1-9]$/.test(e.key)) {
      const l = store.data.lists[Number(e.key) - 1];
      if (l) {
        e.preventDefault();
        open(l.id);
      }
    } else if (ctrl && (e.key === 'PageDown' || e.key === 'Tab')) {
      e.preventDefault();
      step(e.shiftKey ? -1 : 1);
    } else if (ctrl && e.key === 'PageUp') {
      e.preventDefault();
      step(-1);
    } else if (!typing && !ctrl && !e.altKey && (k === 'n' || e.key === '/') && active) {
      e.preventDefault();
      screen = 'list';
      setTimeout(() => document.getElementById(`add-${active.id}`)?.focus(), 0);
    } else if (ctrl && (k === 'r' || e.key === 'F5')) {
      e.preventDefault(); // pas de rechargement de la page dans l'app
    }
  }
</script>

<svelte:window onkeydown={onKey} />

<div class="window">
  <div class="titlebar" data-tauri-drag-region>
    <div class="brand" data-tauri-drag-region>
      <Logo size={18} />
      <span data-tauri-drag-region>Check</span>
    </div>
    <div style="display: flex">
      <button class="wc" aria-label="Réduire" onclick={() => win.minimize()}><Icon name="minus" size={14} stroke={1.8} /></button>
      <button class="wc" aria-label="Agrandir" onclick={() => win.toggleMaximize()}><Icon name="square" size={13} stroke={1.8} /></button>
      <button class="wc close" aria-label="Fermer" onclick={() => { flush(); win.close(); }}><Icon name="x" size={14} stroke={1.8} /></button>
    </div>
  </div>

  <div class="body">
    <nav class="sidebar" aria-label="Checklists">
      <button class="btn primary" style="width: 100%" onclick={() => (modal = { mode: 'create' })} aria-keyshortcuts="Control+N">
        <Icon name="plus" size={15} stroke={2.2} />Nouvelle checklist
      </button>
      <p class="label" style="padding: 16px 10px 6px">Mes checklists</p>
      <div class="lists" bind:this={navBox}>
        {#each store.data.lists as l, idx (l.id)}
          <button
            class="nav"
            class:on={screen === 'list' && active?.id === l.id}
            class:dragging={draggingId === l.id}
            data-reorder
            animate:flip={{ duration: 180 }}
            title="Ctrl+{idx + 1 <= 9 ? idx + 1 : '…'} · glisser ou Alt+↑/↓ pour réordonner"
            onclick={() => open(l.id)}
            onpointerdown={(e) => onNavDown(e, l, idx)}
            onkeydown={(e) => onNavKey(e, idx)}
          >
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
      <button class="nav" class:on={screen === 'settings'} onclick={() => (screen = 'settings')} aria-keyshortcuts="Control+,">
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
        {#key store.toast.key}
          <div class="toast-wrap">
            <div class="toast" role="status">
              <Icon name="check" size={16} stroke={3} />{store.toast.msg}
              {#if store.toast.undo}
                <button class="undo" onclick={runUndo}>Annuler<kbd>Ctrl+Z</kbd></button>
              {/if}
            </div>
          </div>
        {/key}
      {/if}
    </main>
  </div>

  {#if modal}
    <ListModal
      list={modal.mode === 'edit' ? active : null}
      onclose={() => (modal = null)}
      oncreated={(id) => {
        modal = null;
        open(id);
      }}
      ondeleted={() => {
        modal = null;
        activeId = store.data.lists[0]?.id ?? null;
      }}
      onrestored={(id) => open(id)}
    />
  {/if}
</div>
