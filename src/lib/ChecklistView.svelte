<script>
  import Icon from './Icon.svelte';
  import Items from './Items.svelte';
  import { store, commitList, uncheckAll, togglePin, streakInfo, BGS } from './state.svelte.js';
  import { nextReset, scheduleLabel, fmtIn } from './schedule.js';

  let { list, onedit } = $props();

  const done = $derived(list.items.filter((i) => i.done).length);
  const total = $derived(list.items.length);
  const pct = $derived(total ? Math.round((done / total) * 100) : 0);
  const resetIn = $derived(
    store.data.settings.paused ? 'Suspendu' : fmtIn(nextReset(list.schedule, list.lastReset), store.now)
  );
  const st = $derived(streakInfo(list));
  const periodWord = $derived(
    { daily: 'jour', weekly: 'semaine', monthly: 'mois', interval: 'période', none: 'fois' }[list.schedule.kind] ?? 'période'
  );

  function setBg(id) {
    list.bg = id;
    commitList(list);
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
          {#if st.current > 1}
            <span class="chip streak" title="Périodes terminées d’affilée"><Icon name="flame" size={15} stroke={2.2} />Série : {st.current}</span>
          {/if}
          <span class="head-actions">
          <button class="btn ghost" onclick={onedit} aria-keyshortcuts="Control+E"><Icon name="edit" size={15} />Modifier</button>
          <button class="btn ghost" class:pinned={list.pinned} aria-pressed={list.pinned} aria-keyshortcuts="Control+D" onclick={() => togglePin(list)}>
            <Icon name="pin" size={16} />{list.pinned ? 'Sur le bureau' : 'Poser sur le bureau'}
          </button>
          </span>
        </div>
      </div>
    {/key}

    <div
      class="track"
      role="progressbar"
      aria-label="Progression de {list.name}"
      aria-valuenow={pct}
      aria-valuemin="0"
      aria-valuemax="100"
      aria-valuetext="{done} éléments cochés sur {total}"
    >
      <div class="fill" style="width: {pct}%"></div>
    </div>

    {#if total > 0 && done === total}
      <div class="banner" role="status">
        <Icon name="circleCheck" size={28} />
        <div style="display: flex; flex-direction: column; gap: 2px">
          <strong class="disp" style="font-size: 17px">Tout est coché, bravo !{st.current > 1 ? ` ${st.current} d’affilée.` : ''}</strong>
          <span style="font-size: 13px; font-weight: 600">
            {list.schedule.kind === 'none' ? 'Décochez tout quand vous voulez recommencer.' : `Remise à zéro automatique ${resetIn}.`}
          </span>
        </div>
      </div>
    {/if}

    <Items {list} />

    {#if st.completions > 0 || st.periods > 0}
      <section class="stats" aria-label="Statistiques">
        <div class="stat"><b>{st.current}</b><span>Série</span></div>
        <div class="stat"><b>{st.best}</b><span>Record</span></div>
        <div class="stat"><b>{st.completions}</b><span>Terminée</span></div>
        {#if st.rate !== null}
          <div class="stat"><b>{st.rate} %</b><span>Réussite</span></div>
        {/if}
        {#if st.recent.length}
          <div class="history" role="img" aria-label="{st.recent.filter((e) => e.completed).length} {periodWord}(s) terminé(es) sur les {st.recent.length} dernières">
            {#each st.recent as e, i (i)}
              <i
                class:ok={e.completed}
                class:part={!e.completed && e.done > 0}
                style="--p: {e.total ? Math.round((e.done / e.total) * 100) : 0}%"
                title="{new Date(e.at).toLocaleDateString('fr-FR')} : {e.done}/{e.total}"
              ></i>
            {/each}
          </div>
        {/if}
      </section>
    {/if}

    <div class="list-foot">
      <div class="swatches" role="group" aria-label="Style du fond">
        <span class="label" style="margin-right: 6px">Fond</span>
        {#each BGS as b (b.id)}
          <button class="swbtn" class:on={list.bg === b.id} aria-label="Fond {b.name}" aria-pressed={list.bg === b.id} title={b.name} onclick={() => setBg(b.id)}>
            <span class="sw mini bg bg-{b.id}" style="--c: {list.color}"></span>
          </button>
        {/each}
      </div>
      <button class="btn ghost" onclick={() => uncheckAll(list)} disabled={done === 0} aria-keyshortcuts="Control+Shift+U">
        <Icon name="reset" size={16} stroke={2.4} />Tout décocher
      </button>
    </div>
  </div>
</div>
