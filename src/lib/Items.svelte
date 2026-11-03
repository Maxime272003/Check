<script>
  import Icon from './Icon.svelte';
  import { commit, uid } from './state.svelte.js';

  let { list, compact = false } = $props();
  let editing = $state(null);
  let editText = $state('');
  let draft = $state('');

  function toggle(it) {
    it.done = !it.done;
    commit();
  }
  function remove(it) {
    list.items = list.items.filter((x) => x.id !== it.id);
    commit();
  }
  function startEdit(it) {
    editing = it.id;
    editText = it.text;
  }
  function saveEdit(it) {
    const t = editText.trim();
    if (t && t !== it.text) {
      it.text = t;
      commit();
    }
    editing = null;
  }
  function onEditKey(e, it) {
    if (e.key === 'Enter') saveEdit(it);
    if (e.key === 'Escape') editing = null;
  }
  function add(e) {
    e.preventDefault();
    const t = draft.trim();
    if (!t) return;
    list.items.push({ id: uid(), text: t, done: false });
    draft = '';
    commit();
  }
  function focus(node) {
    node.focus();
    node.select();
  }
</script>

<div class="items">
  {#each list.items as it (it.id)}
    <div class="row" class:done={it.done}>
      {#if editing === it.id}
        <label class="sr" for="edit-{it.id}">Modifier l’élément</label>
        <input
          id="edit-{it.id}"
          class="row-edit"
          bind:value={editText}
          use:focus
          onkeydown={(e) => onEditKey(e, it)}
          onblur={() => saveEdit(it)}
        />
      {:else}
        <button class="row-main" aria-pressed={it.done} onclick={() => toggle(it)} ondblclick={() => startEdit(it)} title="Double-clic pour modifier">
          <span class="cb" class:on={it.done}><Icon name="check" size={compact ? 12 : 14} stroke={3.6} /></span>
          <span class="txt" class:on={it.done}>{it.text}</span>
        </button>
        {#if !compact}
          <button class="icon-btn row-del" aria-label="Supprimer « {it.text} »" onclick={() => remove(it)}><Icon name="x" size={15} stroke={2.4} /></button>
        {/if}
      {/if}
    </div>
  {/each}

  {#if list.items.length === 0 && !compact}
    <p class="empty">Aucun élément pour l’instant. Ajoutez le premier ci-dessous.</p>
  {/if}

  <form class={compact ? 'w-add' : 'addrow'} onsubmit={add}>
    <Icon name="plus" size={compact ? 14 : 18} stroke={2.4} />
    <label class="sr" for="add-{list.id}">Nouvel élément</label>
    <input id="add-{list.id}" placeholder={compact ? 'Ajouter…' : 'Ajouter un élément puis Entrée'} bind:value={draft} autocomplete="off" />
  </form>
</div>
