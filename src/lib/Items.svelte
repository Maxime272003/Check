<script>
  import { flip } from 'svelte/animate';
  import Icon from './Icon.svelte';
  import { commitList, uid, toggleItem, removeItem, moveItem, trackCompletion } from './state.svelte.js';
  import { dragReorder } from './reorder.js';

  let { list, compact = false } = $props();
  let editing = $state(null);
  let editText = $state('');
  let draft = $state('');
  let draggingId = $state(null);
  let container = $state();

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
  function onRowKey(e, idx) {
    if (!e.altKey || (e.key !== 'ArrowUp' && e.key !== 'ArrowDown')) return;
    e.preventDefault();
    const to = e.key === 'ArrowUp' ? idx - 1 : idx + 1;
    if (to < 0 || to >= list.items.length) return;
    moveItem(list, idx, to);
    commitList(list);
  }
  function onPointerDown(e, it, idx) {
    if (compact || editing) return;
    dragReorder(e, {
      container,
      from: idx,
      onstart: () => (draggingId = it.id),
      move: (a, b) => moveItem(list, a, b),
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

<div class="items" bind:this={container}>
  {#each list.items as it, idx (it.id)}
    <div class="row" class:done={it.done} class:dragging={draggingId === it.id} data-reorder animate:flip={{ duration: 180 }}>
      {#if editing === it.id}
        <label class="sr" for="edit-{it.id}">Modifier l’élément</label>
        <input id="edit-{it.id}" class="row-edit" bind:value={editText} use:focus onkeydown={(e) => onEditKey(e, it)} onblur={() => saveEdit(it)} />
      {:else}
        {#if !compact}
          <span class="grip" aria-hidden="true"><Icon name="grip" size={14} stroke={2.4} /></span>
        {/if}
        <button
          class="row-main"
          aria-pressed={it.done}
          aria-keyshortcuts={compact ? undefined : 'Alt+ArrowUp Alt+ArrowDown'}
          title={compact ? undefined : 'Clic : cocher · Double-clic : modifier · Glisser ou Alt+↑/↓ : déplacer'}
          onclick={() => toggleItem(list, it)}
          ondblclick={() => !compact && startEdit(it)}
          onpointerdown={(e) => onPointerDown(e, it, idx)}
          onkeydown={(e) => onRowKey(e, idx)}
        >
          <span class="cb" class:on={it.done}><Icon name="check" size={compact ? 11 : 13} stroke={3.2} /></span>
          <span class="txt" class:on={it.done}>{it.text}</span>
        </button>
        {#if !compact}
          <button class="icon-btn row-del" aria-label="Supprimer « {it.text} »" onclick={() => removeItem(list, it)}><Icon name="x" size={15} stroke={2.4} /></button>
        {/if}
      {/if}
    </div>
  {/each}

  {#if list.items.length === 0 && !compact}
    <p class="empty">Aucun élément pour l’instant. Ajoutez le premier ci-dessous.</p>
  {/if}

  <form class={compact ? 'w-add' : 'addrow'} onsubmit={add}>
    <Icon name="plus" size={compact ? 13 : 16} stroke={2} />
    <label class="sr" for="add-{list.id}">Nouvel élément</label>
    <input id="add-{list.id}" placeholder={compact ? 'Ajouter…' : 'Ajouter un élément puis Entrée  (N)'} bind:value={draft} autocomplete="off" />
  </form>
</div>
