<script>
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { getCurrentWindow, LogicalSize } from '@tauri-apps/api/window';
  import Icon from './Icon.svelte';
  import Items from './Items.svelte';
  import { store, commitList, findList } from './state.svelte.js';
  import { nextReset, scheduleLabel, fmtIn } from './schedule.js';

  let { id } = $props();
  const win = getCurrentWindow();
  const list = $derived(findList(id));
  let menu = $state(false);
  let root = $state();

  const done = $derived(list ? list.items.filter((i) => i.done).length : 0);
  const pct = $derived(list?.items.length ? Math.round((done / list.items.length) * 100) : 0);
  const resetText = $derived(
    !list
      ? ''
      : list.schedule.kind === 'none'
        ? 'Remise à zéro manuelle'
        : store.data.settings.paused
          ? 'Remises à zéro suspendues'
          : `${scheduleLabel(list.schedule)} · ${fmtIn(nextReset(list.schedule, list.lastReset), store.now)}`
  );

  // La fenêtre épouse la taille du post-it (+ marge pour l'ombre décalée).
  onMount(() => {
    const ro = new ResizeObserver(() => {
      if (!root) return;
      const r = root.getBoundingClientRect();
      win.setSize(new LogicalSize(Math.ceil(r.width) + 14, Math.ceil(r.height) + 14)).catch(() => {});
    });
    if (root) ro.observe(root);
    return () => ro.disconnect();
  });

  function set(key, value) {
    list.widget[key] = value;
    commitList(list);
  }
  function startDrag(e) {
    if (e.button !== 0 || list.widget.locked || e.target.closest('button')) return;
    win.startDragging().catch(() => {});
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
    style="--c: {list.color}; width: {list.widget.width}px; opacity: {list.widget.opacity}"
  >
    <div class="w-head" class:locked={list.widget.locked} role="presentation" onmousedown={startDrag} ondblclick={() => set('collapsed', !list.widget.collapsed)}>
      <span class="dot" style="background: {list.color}"></span>
      <strong>{list.name}</strong>
      <span class="count">{done}/{list.items.length}</span>
      <div class="w-tools">
        <button class="tbtn" aria-label={list.widget.collapsed ? 'Déplier' : 'Replier'} onclick={() => set('collapsed', !list.widget.collapsed)}>
          <Icon name={list.widget.collapsed ? 'down' : 'up'} size={14} stroke={2.4} />
        </button>
        <button class="tbtn" aria-label="Options du post-it" aria-expanded={menu} onclick={() => (menu = !menu)}>
          <Icon name="dots" size={16} fill="currentColor" stroke={0} />
        </button>
      </div>
    </div>

    {#if menu}
      <div class="w-menu" role="menu" aria-label="Options du post-it">
        <button role="menuitemcheckbox" aria-checked={list.widget.onTop} class="mi" onclick={() => set('onTop', !list.widget.onTop)}>
          Toujours au premier plan<span class="toggle sm" class:on={list.widget.onTop}></span>
        </button>
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
        <div class="mi static">
          <span>Largeur</span>
          <div class="segs" role="group" aria-label="Largeur">
            {#each [[260, 'S'], [300, 'M'], [360, 'L']] as [w, n] (w)}
              <button class="seg" class:on={list.widget.width === w} aria-pressed={list.widget.width === w} onclick={() => set('width', w)}>{n}</button>
            {/each}
          </div>
        </div>
        <div class="sep"></div>
        <button role="menuitem" class="mi" onclick={() => { menu = false; invoke('open_main', { listId: list.id }); }}>
          Ouvrir dans Check<Icon name="external" size={14} />
        </button>
        <button role="menuitem" class="mi red" onclick={unpin}>Retirer du bureau<Icon name="x" size={14} stroke={2.4} /></button>
      </div>
    {/if}

    {#if !list.widget.collapsed}
      <div class="w-body">
        <div class="track"><div class="fill" style="width: {pct}%"></div></div>
        <Items {list} compact />
        <div class="w-foot"><Icon name="reset" size={12} stroke={2.4} />{resetText}</div>
      </div>
    {/if}
  </div>
{/if}
