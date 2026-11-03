<script>
  import { onMount, tick } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { getCurrentWindow, LogicalSize } from '@tauri-apps/api/window';
  import Icon from './Icon.svelte';
  import Items from './Items.svelte';
  import { scrollFade } from './scrollFade.js';
  import { store, commitList, findList } from './state.svelte.js';
  import { nextReset, scheduleLabel, fmtIn } from './schedule.js';

  let { id } = $props();
  const win = getCurrentWindow();
  /** Marge transparente autour du post-it dans sa fenêtre (pour l'ombre). */
  const MARGIN = 10;
  const MIN_W = 220;
  const MAX_W = 560;

  const list = $derived(findList(id));
  let menu = $state(false);
  let root = $state();
  let resizing = false;
  let resizeTimer;

  const done = $derived(list ? list.items.filter((i) => i.done).length : 0);
  const pct = $derived(list?.items.length ? Math.round((done / list.items.length) * 100) : 0);
  const autoHeight = $derived(list?.widget.height == null || list?.widget.collapsed);
  const resetText = $derived(
    !list
      ? ''
      : list.schedule.kind === 'none'
        ? 'Remise à zéro manuelle'
        : store.data.settings.paused
          ? 'Remises à zéro suspendues'
          : `${scheduleLabel(list.schedule)} · ${fmtIn(nextReset(list.schedule, list.lastReset), store.now)}`
  );

  // Hauteur automatique : la fenêtre épouse le contenu. La fenêtre est créée cachée
  // et ne s'affiche qu'une fois à la bonne taille (pas de carré blanc au chargement).
  onMount(() => {
    let shown = false;
    const fit = async () => {
      if (!root || resizing) return;
      if (autoHeight) {
        const r = root.getBoundingClientRect();
        await win.setSize(new LogicalSize(Math.ceil(r.width) + 2 * MARGIN, Math.ceil(r.height) + 2 * MARGIN)).catch(() => {});
      }
      if (!shown) {
        shown = true;
        await tick();
        win.show().catch(() => {});
      }
    };
    const ro = new ResizeObserver(fit);
    if (root) ro.observe(root);
    fit();

    // Redimensionnement par le coin : la taille suit la fenêtre, enregistrée une fois lâchée.
    const unResize = win.onResized(async (e) => {
      if (!resizing || !list) return;
      const s = e.payload.toLogical(await win.scaleFactor());
      list.widget.width = Math.round(Math.min(MAX_W, Math.max(MIN_W, s.width - 2 * MARGIN)));
      list.widget.height = Math.round(Math.max(90, s.height - 2 * MARGIN));
      clearTimeout(resizeTimer);
      resizeTimer = setTimeout(() => {
        resizing = false;
        commitList(list);
      }, 500);
    });
    return () => {
      ro.disconnect();
      unResize.then((f) => f());
    };
  });

  function set(key, value) {
    list.widget[key] = value;
    commitList(list);
  }
  function startResize(e) {
    if (e.button !== 0) return;
    e.preventDefault();
    resizing = true;
    win.startResizeDragging('SouthEast').catch(() => (resizing = false));
  }
  function autoFit() {
    list.widget.height = null;
    commitList(list);
  }
  function unpin() {
    list.pinned = false;
    commitList(list);
  }
</script>

{#if list}
  <div
    bind:this={root}
    class="widget bg bg-{list.bg}"
    class:menu
    style="--c: {list.color}; width: {list.widget.width}px; {autoHeight ? '' : `height: ${list.widget.height}px;`} opacity: {list.widget.opacity}"
  >
    <div
      class="w-head"
      class:locked={list.widget.locked}
      data-tauri-drag-region={list.widget.locked ? undefined : true}
      role="presentation"
      ondblclick={() => set('collapsed', !list.widget.collapsed)}
    >
      <span class="dot" style="background: {list.color}"></span>
      <strong>{list.name}</strong>
      <span class="count">{done}/{list.items.length}</span>
      <div class="w-tools">
        <button
          class="tbtn"
          class:on={list.widget.onTop}
          class:w-pin-on={list.widget.onTop}
          aria-pressed={list.widget.onTop}
          aria-label={list.widget.onTop ? 'Ne plus garder au premier plan' : 'Garder au premier plan'}
          title={list.widget.onTop ? 'Au premier plan' : 'Garder au premier plan'}
          onclick={() => set('onTop', !list.widget.onTop)}
        >
          <Icon name="pin" size={14} stroke={2.2} />
        </button>
        <button class="tbtn" aria-label={list.widget.collapsed ? 'Déplier' : 'Replier'} onclick={() => set('collapsed', !list.widget.collapsed)}>
          <Icon name={list.widget.collapsed ? 'down' : 'up'} size={14} stroke={2.2} />
        </button>
        <button class="tbtn" aria-label="Options du post-it" aria-expanded={menu} onclick={() => (menu = !menu)}>
          <Icon name="dots" size={15} fill="currentColor" stroke={0} />
        </button>
      </div>
    </div>

    {#if menu}
      <div class="w-menu" role="menu" aria-label="Options du post-it">
        <button role="menuitemcheckbox" aria-checked={list.widget.locked} class="mi" onclick={() => set('locked', !list.widget.locked)}>
          Verrouiller la position<span class="toggle sm" class:on={list.widget.locked}></span>
        </button>
        <div class="mi static">
          <span>Opacité</span>
          <div class="segs" role="group" aria-label="Opacité">
            {#each [1, 0.85, 0.7] as o (o)}
              <button class="seg" class:on={list.widget.opacity === o} aria-pressed={list.widget.opacity === o} onclick={() => set('opacity', o)}>{Math.round(o * 100)} %</button>
            {/each}
          </div>
        </div>
        {#if !autoHeight}
          <button role="menuitem" class="mi" onclick={autoFit}>Hauteur automatique</button>
        {/if}
        <div class="sep"></div>
        <button role="menuitem" class="mi" onclick={() => { menu = false; invoke('open_main', { listId: list.id }); }}>
          Ouvrir dans Check<Icon name="external" size={13} />
        </button>
        <button role="menuitem" class="mi red" onclick={unpin}>Retirer du bureau<Icon name="x" size={13} stroke={2.2} /></button>
      </div>
    {/if}

    {#if !list.widget.collapsed}
      <div class="w-body" use:scrollFade>
        <div class="track"><div class="fill" style="width: {pct}%"></div></div>
        <Items {list} compact />
        <div class="w-foot"><Icon name="reset" size={12} />{resetText}</div>
      </div>
    {/if}

    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="w-resize" title="Redimensionner" onpointerdown={startResize}></div>
  </div>
{/if}
