<script>
  import { flip } from 'svelte/animate';
  import { fade } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import Icon from './Icon.svelte';
  import { commitList, uid, toggleItem, removeItem, moveItemById, trackCompletion } from './state.svelte.js';
  import { dragReorder } from './reorder.js';

  let { list, compact = false } = $props();
  let editing = $state(null);
  let editText = $state('');
  let draft = $state('');
  let draggingId = $state(null);
  let box = $state();
  /** Éléments en train de glisser vers leur nouveau groupe (légèrement soulevés). */
  let moving = $state({});

  /** Quand on coche, l'élément reste à sa place le temps de l'animation de la case, puis glisse. */
  const SETTLE_MS = 420;
  let frozen = $state({});

  const isDone = (it) => (it.id in frozen ? frozen[it.id] : it.done);
  const todo = $derived(list.items.filter((it) => !isDone(it)));
  // Terminés : dans l'ordre où ils ont été cochés, le dernier tout en bas.
  const done = $derived(list.items.filter((it) => isDone(it)).sort((a, b) => (a.doneAt ?? 0) - (b.doneAt ?? 0)));

  // Une seule liste animée (à faire, ajout, séparateur, terminés) : un élément coché
  // glisse à travers les autres jusqu'à sa nouvelle place, sans disparaître.
  const entries = $derived([
    ...todo.map((it, idx) => ({ key: it.id, kind: 'item', it, idx, group: 'todo' })),
    ...(list.items.length === 0 && !compact ? [{ key: '__empty', kind: 'empty' }] : []),
    ...(todo.length === 0 && done.length > 0 ? [{ key: '__alldone', kind: 'alldone' }] : []),
    { key: '__add', kind: 'add' },
    ...(done.length > 0 ? [{ key: '__sep', kind: 'sep' }] : []),
    ...done.map((it, idx) => ({ key: it.id, kind: 'item', it, idx, group: 'done' }))
  ]);

  function toggle(it) {
    if (!(it.id in frozen)) frozen[it.id] = it.done;
    toggleItem(list, it);
    clearTimeout(timers[it.id]);
    timers[it.id] = setTimeout(() => {
      delete frozen[it.id];
      delete timers[it.id];
      moving[it.id] = true;
      setTimeout(() => delete moving[it.id], 560);
    }, SETTLE_MS);
  }
  const timers = {};

  function startEdit(it) {
    editing = it.id;
    editText = it.text;
  }
  function saveEdit(it) {
    const t = editText.trim();
    if (t && t !== it.text) {
      it.text = t;
      commitList(list);
    }
    editing = null;
  }
  function onEditKey(e, it) {
    if (e.key === 'Enter') saveEdit(it);
    if (e.key === 'Escape') {
      e.stopPropagation();
      editing = null;
    }
  }
  function add(e) {
    e.preventDefault();
    const t = draft.trim();
    if (!t) return;
    list.items.push({ id: uid(), text: t, done: false });
    if (list.completedThisPeriod === undefined) list.completedThisPeriod = false;
    trackCompletion(list);
    draft = '';
    commitList(list);
  }
  // Réordonner : uniquement à l'intérieur de son groupe (à faire / terminés).
  function onRowKey(e, group, idx) {
    if (group !== todo) return;
    if (!e.altKey || (e.key !== 'ArrowUp' && e.key !== 'ArrowDown')) return;
    e.preventDefault();
    const to = e.key === 'ArrowUp' ? idx - 1 : idx + 1;
    if (to < 0 || to >= group.length) return;
    moveItemById(list, group[idx].id, group[to].id);
    commitList(list);
  }
  function onPointerDown(e, it, group, idx) {
    if (compact || editing || group !== 'todo') return;
    dragReorder(e, {
      container: box,
      from: idx,
      onstart: () => (draggingId = it.id),
      move: (a, b) => {
        if (todo[a] && todo[b]) moveItemById(list, todo[a].id, todo[b].id);
      },
      end: (moved) => {
        draggingId = null;
        if (moved) commitList(list);
      }
    });
  }
  function focus(node) {
    node.focus();
    node.select();
  }
</script>

{#snippet row(it, idx, groupName, group)}
  {#if editing === it.id}
    <label class="sr" for="edit-{it.id}">Modifier l’élément</label>
    <input id="edit-{it.id}" class="row-edit" bind:value={editText} use:focus onkeydown={(e) => onEditKey(e, it)} onblur={() => saveEdit(it)} />
  {:else}
    {#if !compact && groupName === 'todo'}
      <span class="grip" aria-hidden="true"><Icon name="grip" size={14} stroke={2.4} /></span>
    {/if}
    <button
      class="row-main"
      aria-pressed={it.done}
      aria-keyshortcuts={compact || groupName === 'done' ? undefined : 'Alt+ArrowUp Alt+ArrowDown'}
      title={compact ? undefined : groupName === 'done' ? 'Clic : décocher · Double-clic : modifier' : 'Clic : cocher · Double-clic : modifier · Glisser ou Alt+↑/↓ : déplacer'}
      onclick={() => toggle(it)}
      ondblclick={() => !compact && startEdit(it)}
      onpointerdown={(e) => onPointerDown(e, it, groupName, idx)}
      onkeydown={(e) => onRowKey(e, group, idx)}
    >
      <span class="cb" class:on={it.done}><Icon name="check" size={compact ? 11 : 13} stroke={3.2} /></span>
      <span class="txt" class:on={it.done}>{it.text}</span>
    </button>
    {#if !compact}
      <button class="icon-btn row-del" aria-label="Supprimer « {it.text} »" onclick={() => removeItem(list, it)}><Icon name="x" size={15} stroke={2.4} /></button>
    {/if}
  {/if}
{/snippet}

<div class="items" bind:this={box}>
  {#each entries as e (e.key)}
    <div
      class={e.kind === 'item' ? 'row' : e.kind === 'add' ? (compact ? 'w-add-wrap' : 'add-wrap') : e.kind === 'sep' ? 'done-sep' : 'empty' + (e.kind === 'alldone' ? ' all-done' : '')}
      class:done={e.kind === 'item' && e.it.done}
      class:dragging={e.kind === 'item' && draggingId === e.key}
      class:moving={moving[e.key]}
      data-reorder={e.kind === 'item' && e.group === 'todo' ? '' : undefined}
      animate:flip={{ duration: 420, easing: cubicOut }}
      in:fade={{ duration: 160 }}
    >
      {#if e.kind === 'item'}
        {@render row(e.it, e.idx, e.group, e.group === 'todo' ? todo : done)}
      {:else if e.kind === 'add'}
        <form class={compact ? 'w-add' : 'addrow'} onsubmit={add}>
          <Icon name="plus" size={compact ? 13 : 16} stroke={2} />
          <label class="sr" for="add-{list.id}">Nouvel élément</label>
          <input id="add-{list.id}" placeholder={compact ? 'Ajouter…' : 'Ajouter un élément puis Entrée  (N)'} bind:value={draft} autocomplete="off" />
        </form>
      {:else if e.kind === 'sep'}
        Terminés · {done.length}
      {:else if e.kind === 'alldone'}
        <Icon name="circleCheck" size={15} />Tout est fait
      {:else}
        Aucun élément pour l’instant. Ajoutez le premier ci-dessous.
      {/if}
    </div>
  {/each}
</div>
