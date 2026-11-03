<script>
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import Icon from './Icon.svelte';
  import Logo from './Logo.svelte';
  import { store, commitSettings, toast, flush } from './state.svelte.js';

  const REPO_URL = 'https://github.com/Maxime272003/Check';
  // Estimation : chaque post-it est une vue WebView2 (processus de rendu partagé avec Check).
  const MB_PER_WIDGET = 35;

  let autostart = $state(false);
  let version = $state('');
  let widgets = $state(0);
  // idle | checking | available | uptodate | none | downloading | error
  let up = $state('idle');
  let info = $state(null);
  let error = $state('');
  let progress = $state({ downloaded: 0, total: 0 });

  const s = $derived(store.data.settings);
  const pct = $derived(progress.total ? Math.round((progress.downloaded / progress.total) * 100) : 0);
  const mo = (b) => (b / 1048576).toFixed(1).replace('.', ',');
  const lastCheck = $derived(
    s.lastUpdateCheck ? new Date(s.lastUpdateCheck).toLocaleString('fr-FR', { dateStyle: 'short', timeStyle: 'short' }) : 'jamais'
  );
  const pinned = $derived(store.data.lists.filter((l) => l.pinned).length);

  const KEYS = [
    ['Ctrl+Alt+C', 'Afficher / masquer Check (partout dans Windows)'],
    ['Ctrl+Alt+B', 'Post-it : afficher, ramener devant ou masquer (partout)'],
    ['Ctrl+N', 'Nouvelle checklist'],
    ['N ou /', 'Ajouter un élément'],
    ['Ctrl+1 … 9', 'Aller à la checklist n'],
    ['Ctrl+Tab', 'Checklist suivante (Maj : précédente)'],
    ['Ctrl+E', 'Modifier la checklist'],
    ['Ctrl+D', 'Poser sur le bureau / retirer'],
    ['Ctrl+Maj+U', 'Tout décocher'],
    ['Ctrl+Z', 'Annuler la dernière action'],
    ['Alt+↑ / ↓', 'Déplacer l’élément ou la checklist'],
    ['Ctrl+,', 'Paramètres'],
    ['Échap', 'Fermer la fenêtre de création']
  ];

  onMount(() => {
    invoke('get_autostart').then((v) => (autostart = v));
    invoke('app_version').then((v) => (version = v));
    invoke('widget_count').then((v) => (widgets = v));
    const un = listen('update-progress', (e) => (progress = e.payload));
    return () => un.then((f) => f());
  });

  async function toggleAutostart() {
    try {
      autostart = await invoke('set_autostart', { enabled: !autostart });
    } catch (e) {
      toast(`Impossible de modifier le démarrage : ${e}`);
    }
  }
  function toggle(key) {
    s[key] = !s[key];
    commitSettings();
  }

  const rows = $derived([
    { key: 'startHidden', t: 'Démarrer en arrière-plan', d: 'Au démarrage de Windows, Check reste dans la zone de notification sans ouvrir la fenêtre.', disabled: !autostart },
    { key: 'closeToTray', t: 'Fermer = réduire dans la zone de notification', d: 'La croix masque la fenêtre ; les remises à zéro et les post-it continuent.' },
    { key: 'notifyReset', t: 'Notifier à chaque remise à zéro', d: 'Une notification Windows signale chaque checklist décochée.' },
    { key: 'restoreWidgets', t: 'Réafficher les post-it au démarrage', d: 'Les checklists posées reviennent à leur place sur le bureau.' },
    { key: 'paused', t: 'Suspendre les remises à zéro', d: 'Aucune checklist ne sera décochée automatiquement tant que c’est activé.' }
  ]);

  async function check() {
    up = 'checking';
    error = '';
    try {
      const [res] = await Promise.all([invoke('check_update'), new Promise((r) => setTimeout(r, 700))]);
      info = res;
      up = res.latest === null ? 'none' : res.available ? 'available' : 'uptodate';
      s.lastUpdateCheck = Date.now();
      commitSettings();
    } catch (e) {
      error = String(e);
      up = 'error';
    }
  }
  async function install() {
    up = 'downloading';
    progress = { downloaded: 0, total: 0 };
    try {
      await invoke('install_update');
    } catch (e) {
      error = String(e);
      up = 'error';
    }
  }
  function setTheme(t) {
    s.theme = t;
    commitSettings();
  }
  async function exportData() {
    try {
      const path = await invoke('export_data');
      if (path) toast(`Exporté dans ${path}`);
    } catch (e) {
      toast(`Export impossible : ${e}`);
    }
  }
  async function importData() {
    try {
      const n = await invoke('import_data');
      if (n) toast(`${n} checklist${n > 1 ? 's' : ''} importée${n > 1 ? 's' : ''}`);
    } catch (e) {
      toast(String(e));
    }
  }
</script>

<div class="settings fadeup">
  <div class="settings-col">
    <h1>Paramètres</h1>

    <section class="section">
      <h2 class="label">Démarrage &amp; arrière-plan</h2>
      <div class="card">
        <button role="switch" aria-checked={autostart} class="setrow" onclick={toggleAutostart}>
          <span style="display: flex; flex-direction: column; gap: 3px">
            <span class="t">Lancer Check au démarrage de Windows</span>
            <span class="d">L’application démarre à l’ouverture de session.</span>
          </span>
          <span class="toggle" class:on={autostart}></span>
        </button>
        {#each rows as r (r.key)}
          <button role="switch" aria-checked={!!s[r.key] && !r.disabled} class="setrow" disabled={r.disabled} onclick={() => toggle(r.key)}>
            <span style="display: flex; flex-direction: column; gap: 3px">
              <span class="t">{r.t}</span>
              <span class="d">{r.d}</span>
            </span>
            <span class="toggle" class:on={!!s[r.key] && !r.disabled}></span>
          </button>
        {/each}
        <div class="mem">
          {widgets} post-it ouvert{widgets > 1 ? 's' : ''} sur {pinned} posé{pinned > 1 ? 's' : ''} ·
          environ {widgets * MB_PER_WIDGET} Mo de mémoire en plus (estimation). Les post-it inactifs demandent à Windows de réduire leur mémoire.
        </div>
      </div>
    </section>

    <section class="section">
      <h2 class="label">Apparence</h2>
      <div class="card" style="padding: 14px 18px; display: flex; align-items: center; justify-content: space-between; gap: 16px; flex-wrap: wrap">
        <span class="t" style="font-size: 14px; font-weight: 500">Thème</span>
        <div class="segs" role="group" aria-label="Thème">
          {#each [['light', 'Clair'], ['dark', 'Sombre'], ['system', 'Système']] as [id, name] (id)}
            <button class="seg" class:on={s.theme === id} aria-pressed={s.theme === id} onclick={() => setTheme(id)}>{name}</button>
          {/each}
        </div>
      </div>
    </section>

    <section class="section">
      <h2 class="label">Raccourcis clavier</h2>
      <div class="card keys">
        {#each KEYS as [k, d] (k)}
          <kbd>{k}</kbd><span>{d}</span>
        {/each}
      </div>
    </section>

    <section class="section">
      <h2 class="label">Données</h2>
      <div class="card">
        <div class="data-actions">
          <button class="btn ghost" onclick={exportData}><Icon name="download" size={16} stroke={2.4} />Exporter</button>
          <button class="btn ghost" onclick={importData}><Icon name="upload" size={16} stroke={2.4} />Importer</button>
          <button class="btn ghost" onclick={() => invoke('open_backups')}><Icon name="folder" size={16} />Dossier des sauvegardes</button>
        </div>
        <div class="mem">
          Une sauvegarde automatique est faite à chaque lancement puis toutes les 6 h (les 10 dernières sont gardées).
          L’import ajoute les checklists d’un export ou d’une sauvegarde sans rien effacer.
        </div>
      </div>
    </section>

    <section class="section">
      <h2 class="label">Mises à jour</h2>
      <div class="card" style="padding: 18px; display: flex; flex-direction: column; gap: 14px">
        <div style="display: flex; align-items: center; gap: 14px; flex-wrap: wrap">
          <Logo size={40} />
          <div style="flex: 1; display: flex; flex-direction: column; gap: 2px; min-width: 180px">
            <span style="font-size: 15px; font-weight: 600">Check <span class="mono" style="font-size: 13px">v{version}</span></span>
            <span style="font-size: 13px; color: var(--muted); font-weight: 500">GitHub Releases · dernière vérification : {lastCheck}</span>
          </div>
          {#if up !== 'downloading'}
            <button class="btn primary" onclick={check} disabled={up === 'checking'}>
              {#if up === 'checking'}<span class="spinner"></span>Recherche…{:else}<Icon name="reset" size={16} stroke={2.4} />Rechercher une mise à jour{/if}
            </button>
          {/if}
        </div>

        <div aria-live="polite">
          {#if up === 'available'}
            <div class="update-box hot">
              <div style="display: flex; align-items: center; gap: 10px; flex-wrap: wrap">
                <span class="badge"></span>
                <strong style="font-size: 14px">Version {info.latest} disponible</strong>
                <span class="mono" style="font-size: 12px">signée · vérifiée avant installation</span>
              </div>
              {#if info.notes}<pre class="notes">{info.notes}</pre>{/if}
              <div style="display: flex; gap: 8px; justify-content: flex-end; flex-wrap: wrap">
                {#if info.htmlUrl}<button class="btn ghost" onclick={() => invoke('open_url', { url: info.htmlUrl })}><Icon name="external" size={15} />Voir la release</button>{/if}
                <button class="btn primary" onclick={install}><Icon name="download" size={16} stroke={2.4} />Installer et redémarrer</button>
              </div>
            </div>
          {:else if up === 'downloading'}
            <div class="update-box hot">
              <div style="display: flex; justify-content: space-between; font-size: 13px; font-weight: 700">
                <span>Téléchargement de la version {info.latest}…</span>
                <span class="mono">{mo(progress.downloaded)}{progress.total ? ` / ${mo(progress.total)}` : ''} Mo</span>
              </div>
              <div class="dl" role="progressbar" aria-label="Téléchargement" aria-valuenow={pct} aria-valuemin="0" aria-valuemax="100"><div style="width: {pct}%"></div></div>
              <span style="font-size: 12px; font-weight: 600">Check se fermera, s’installera puis redémarrera tout seul.</span>
            </div>
          {:else if up === 'uptodate'}
            <div class="update-box ok" style="flex-direction: row; align-items: center; gap: 8px; font-weight: 700; font-size: 14px">
              <Icon name="circleCheck" size={18} />Vous avez la dernière version.
            </div>
          {:else if up === 'none'}
            <div class="update-box ok" style="font-weight: 700; font-size: 14px">Aucune release publiée pour l’instant sur GitHub.</div>
          {:else if up === 'error'}
            <div class="update-box err" style="font-weight: 700; font-size: 14px">{error}</div>
          {/if}
        </div>
      </div>
    </section>

    <section class="section">
      <h2 class="label">À propos</h2>
      <div class="card" style="padding: 16px 18px; display: flex; align-items: center; justify-content: space-between; gap: 12px; flex-wrap: wrap; font-size: 14px; font-weight: 600">
        <span>Tauri 2 · Svelte 5 · Rust</span>
        <div style="display: flex; gap: 16px; align-items: center">
          <button class="link" onclick={() => invoke('open_url', { url: REPO_URL })}>Dépôt GitHub</button>
          <button class="btn ghost" onclick={() => { flush(); setTimeout(() => invoke('quit_app'), 200); }}>Quitter Check</button>
        </div>
      </div>
    </section>
  </div>
</div>
