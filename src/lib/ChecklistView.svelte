<script>
  import Icon from './Icon.svelte';
  import Items from './Items.svelte';
  import { store, commit, toast, BGS } from './state.svelte.js';
  import { nextReset, scheduleLabel, fmtIn } from './schedule.js';

  let { list, onedit } = $props();

  const done = $derived(list.items.filter((i) => i.done).length);
  const total = $derived(list.items.length);
  const pct = $derived(total ? Math.round((done / total) * 100) : 0);
  const resetIn = $derived(
    store.data.settings.paused ? 'Suspendu' : fmtIn(nextReset(list.schedule, list.lastReset), store.now)
  );

  function uncheckAll() {
    list.items.forEach((i) => (i.done = false));
    commit();
    toast(`« ${list.name} » a été décochée`);
  }
  function togglePin() {
    list.pinned = !list.pinned;
    commit();
    toast(list.pinned ? `« ${list.name} » est posée sur le bureau` : `« ${list.name} » retirée du bureau`);
  }
  function setBg(id) {
    list.bg = id;
    commit();
  }
</script>

<div class="list-pane bg bg-{list.bg}" style="--c: {list.color}">
  <div class="list-col">
    {#key list.id}
      <div class="fadeup">
        <p class="label">{done} sur {total} terminés</p>
        <h1 class="list-title">{list.name}</h1>
        <div class="chips">
          <span class="chip"><Icon name="reset" size={14} />{scheduleLabel(list.schedule)}</span>
          <span class="chip mono" style="font-size: 12px"><Icon name="clock" size={14} />{resetIn}</span>
          <span style="flex: 1"></span>
          <button class="btn ghost" onclick={onedit}><Icon name="edit" size={15} />Modifier</button>
          <button class="btn ghost" class:pinned={list.pinned} aria-pressed={list.pinned} onclick={togglePin}>
            <Icon name="pin" size={16} />{list.pinned ? 'Sur le bureau' : 'Poser sur le bureau'}
          </button>
        </div>
      </div>
    {/key}

    <div class="track" role="progressbar" aria-label="Progression" aria-valuenow={pct} aria-valuemin="0" aria-valuemax="100">
      <div class="fill" style="width: {pct}%"></div>
    </div>

    {#if total > 0 && done === total}
      <div class="banner" role="status">
        <Icon name="circleCheck" size={28} />
        <div style="display: flex; flex-direction: column; gap: 2px">
          <strong class="disp" style="font-size: 17px">Tout est coché, bravo !</strong>
          <span style="font-size: 13px; font-weight: 600">
            {list.schedule.kind === 'none' ? 'Décochez tout quand vous voulez recommencer.' : `Remise à zéro automatique ${resetIn}.`}
          </span>
        </div>
      </div>
    {/if}

    <Items {list} />

    <div class="list-foot">
      <div class="swatches" role="group" aria-label="Style du fond">
        <span class="label" style="margin-right: 6px">Fond</span>
        {#each BGS as b (b.id)}
          <button class="swbtn" class:on={list.bg === b.id} aria-label="Fond {b.name}" aria-pressed={list.bg === b.id} title={b.name} onclick={() => setBg(b.id)}>
            <span class="sw mini bg bg-{b.id}" style="--c: {list.color}"></span>
          </button>
        {/each}
      </div>
      <button class="btn ghost" onclick={uncheckAll} disabled={done === 0}>
        <Icon name="reset" size={16} stroke={2.4} />Tout décocher
      </button>
    </div>
  </div>
</div>
