<script>
  import { untrack, onMount } from 'svelte';
  import Icon from './Icon.svelte';
  import { store, commitList, toast, newList, deleteList, COLORS, BGS } from './state.svelte.js';
  import { KINDS, DAYS, DAY_SHORT, nextReset, scheduleLabel, fmtIn, lowerFirst } from './schedule.js';

  /** list = null : création ; sinon modification de cette checklist. */
  let { list = null, onclose, oncreated, ondeleted, onrestored } = $props();

  const init = untrack(() =>
    list
      ? { name: list.name, color: list.color, bg: list.bg, ...$state.snapshot(list.schedule), pin: list.pinned }
      : { name: '', color: '#3B82F6', bg: 'aurora', kind: 'weekly', day: 0, time: '00:00', dom: 1, hours: 24, pin: true }
  );
  let f = $state(init);
  let dialog = $state();

  const sched = $derived({ kind: f.kind, day: f.day, time: f.time || '00:00', dom: Number(f.dom) || 1, hours: Number(f.hours) || 24 });
  const summary = $derived(
    f.kind === 'none'
      ? 'Les éléments restent cochés jusqu’à ce que vous les décochiez.'
      : `Tout sera décoché ${lowerFirst(scheduleLabel(sched))} — prochaine fois ${fmtIn(nextReset(sched, Date.now()), store.now)}.`
  );

  // Focus piégé dans la fenêtre, rendu à l'élément d'origine à la fermeture.
  onMount(() => {
    const previous = document.activeElement;
    dialog?.querySelector('#cl-name')?.focus();
    return () => {
      if (previous instanceof HTMLElement && document.contains(previous)) previous.focus();
    };
  });

  function onkey(e) {
    if (e.key === 'Escape') {
      e.stopPropagation();
      onclose();
      return;
    }
    if (e.key !== 'Tab' || !dialog) return;
    const els = [...dialog.querySelectorAll('button:not([disabled]), input:not([disabled]), [tabindex]:not([tabindex="-1"])')].filter(
      (el) => el.offsetParent !== null
    );
    if (!els.length) return;
    const first = els[0];
    const last = els[els.length - 1];
    if (e.shiftKey && document.activeElement === first) {
      e.preventDefault();
      last.focus();
    } else if (!e.shiftKey && document.activeElement === last) {
      e.preventDefault();
      first.focus();
    }
  }

  function submit(e) {
    e.preventDefault();
    if (list) {
      const schedChanged = JSON.stringify(sched) !== JSON.stringify($state.snapshot(list.schedule));
      list.name = f.name.trim() || list.name;
      list.color = f.color;
      list.bg = f.bg;
      list.schedule = sched;
      list.pinned = f.pin;
      if (schedChanged) {
        list.lastReset = Date.now();
        list.completedThisPeriod = false;
      }
      commitList(list);
      toast('Checklist mise à jour');
      onclose();
    } else {
      const nl = newList({ ...f, ...sched });
      store.data.lists.push(nl);
      commitList(store.data.lists[store.data.lists.length - 1]);
      toast(`Checklist « ${nl.name} » créée${f.pin ? ' et posée sur le bureau' : ''}`);
      oncreated(nl.id);
    }
  }

  function remove() {
    deleteList(list, onrestored);
    ondeleted();
  }
</script>

<div class="scrim" role="presentation" onkeydown={onkey}>
  <div class="sheet" bind:this={dialog} role="dialog" aria-modal="true" aria-labelledby="dlg-title">
  <form style="display: contents" onsubmit={submit}>
    <div class="sheet-main">
      <div class="sheet-scroll">
        <div class="sheet-head">
          <h2 id="dlg-title">{list ? 'Modifier la checklist' : 'Nouvelle checklist'}</h2>
          <button type="button" class="icon-btn" aria-label="Fermer (Échap)" onclick={() => onclose()}><Icon name="x" size={18} /></button>
        </div>

        <div class="field-group">
          <label for="cl-name" class="label">Nom</label>
          <input id="cl-name" class="field" placeholder="Ex. Routine du soir" bind:value={f.name} autocomplete="off" />
        </div>

        <div class="field-group" style="gap: 4px">
          <span class="label" id="lbl-color">Couleur</span>
          <div role="group" aria-labelledby="lbl-color" style="display: flex; flex-wrap: wrap; margin-left: -6px">
            {#each COLORS as c (c.c)}
              <button type="button" class="swbtn" style="width: 44px" class:on={f.color === c.c} aria-label={c.n} aria-pressed={f.color === c.c} onclick={() => (f.color = c.c)}>
                <span class="sw" style="background: {c.c}"></span>
              </button>
            {/each}
          </div>
        </div>

        <div class="field-group">
          <span class="label" id="lbl-bg">Style de fond</span>
          <div class="tiles" role="group" aria-labelledby="lbl-bg">
            {#each BGS as b (b.id)}
              <button type="button" class="tile" class:on={f.bg === b.id} aria-pressed={f.bg === b.id} onclick={() => (f.bg = b.id)}>
                <span class="mini bg bg-{b.id}" style="--c: {f.color}"></span>
                <span class="n">{b.name}</span>
              </button>
            {/each}
          </div>
        </div>

        <div class="field-group" style="gap: 10px">
          <span class="label" id="lbl-reset">Tout décocher automatiquement</span>
          <div class="segs" role="group" aria-labelledby="lbl-reset">
            {#each KINDS as k (k.id)}
              <button type="button" class="seg" class:on={f.kind === k.id} aria-pressed={f.kind === k.id} onclick={() => (f.kind = k.id)}>{k.name}</button>
            {/each}
          </div>
          {#if f.kind !== 'none'}
            <div class="when fadeup">
              {#if f.kind === 'weekly'}
                <div role="group" aria-label="Jour de la semaine" style="display: flex; gap: 4px; flex-wrap: wrap">
                  {#each DAY_SHORT as d, i}
                    <button type="button" class="day" class:on={f.day === i} aria-label={DAYS[i]} aria-pressed={f.day === i} onclick={() => (f.day = i)}>{d}</button>
                  {/each}
                </div>
              {/if}
              {#if f.kind === 'monthly'}
                <label for="cl-dom">Le</label>
                <input id="cl-dom" type="number" min="1" max="28" class="field mono" style="width: 84px" bind:value={f.dom} />
                <span>du mois</span>
              {/if}
              {#if f.kind === 'interval'}
                <label for="cl-hours">Toutes les</label>
                <input id="cl-hours" type="number" min="1" max="720" class="field mono" style="width: 96px" bind:value={f.hours} />
                <span>heures</span>
              {/if}
              {#if f.kind !== 'interval'}
                <label for="cl-time">à</label>
                <input id="cl-time" type="time" class="field mono" style="width: 124px" bind:value={f.time} />
              {/if}
            </div>
          {/if}
          <p class="summary" aria-live="polite"><Icon name="clock" size={16} />{summary}</p>
        </div>

        <button type="button" role="switch" aria-checked={f.pin} class="setrow" style="padding: 8px 0; border: 0" onclick={() => (f.pin = !f.pin)}>
          <span style="display: flex; flex-direction: column; gap: 2px">
            <span class="t">Poser sur le bureau</span>
            <span class="d">S’affiche comme un post-it, cochable sans ouvrir Check.</span>
          </span>
          <span class="toggle" class:on={f.pin}></span>
        </button>
      </div>

      <div class="sheet-foot">
        <div>
          {#if list}
            <button type="button" class="btn danger" onclick={remove}><Icon name="trash" size={16} />Supprimer</button>
          {/if}
        </div>
        <div style="display: flex; gap: 8px">
          <button type="button" class="btn ghost" onclick={() => onclose()}>Annuler</button>
          <button type="submit" class="btn primary">{list ? 'Enregistrer' : 'Créer la checklist'}</button>
        </div>
      </div>
    </div>

    <div class="preview bg bg-{f.bg}" style="--c: {f.color}" aria-hidden="true">
      <p class="label">Aperçu</p>
      <strong style="font-size: 20px; font-weight: 650; line-height: 1.2; overflow-wrap: anywhere">{f.name.trim() || 'Sans titre'}</strong>
      <div class="track"><div class="fill" style="width: 33%"></div></div>
      <div class="items">
        <div class="row"><span class="row-main"><span class="cb on"><Icon name="check" size={12} stroke={3.2} /></span><span class="txt on">Élément terminé</span></span></div>
        <div class="row"><span class="row-main"><span class="cb"></span><span class="txt">Élément à faire</span></span></div>
        <div class="row" style="border-bottom: 0"><span class="row-main"><span class="cb"></span><span class="txt">Élément à faire</span></span></div>
      </div>
      <span class="chip" style="align-self: flex-start">{scheduleLabel(sched)}</span>
    </div>
  </form>
  </div>
</div>
