# check!

Checklists de bureau qui se décochent toutes seules. Tauri 2 + Svelte 5, ~5 Mo, design « dopamine ».

## Fonctionnalités

- **Checklists** : nom, couleur (8), style de fond (Uni, Pois, Quadrillage, Cahier, Bonbon, Néon).
- **Remise à zéro automatique** : chaque jour / semaine (jour + heure) / mois (jour + heure) / toutes les N heures. Rattrapée au lancement si l’ordinateur était éteint.
- **Post-it** : « Poser sur le bureau » ouvre la checklist dans une petite fenêtre sans bordure, déplaçable, repliable, cochable. Options : premier plan, verrouiller, opacité, largeur.
- **Arrière-plan** : icône dans la zone de notification, lancement au démarrage de Windows (option « démarrer en arrière-plan » = sans fenêtre), fermer = réduire.
- **Mises à jour** : Paramètres › Rechercher une mise à jour lit `latest.json` de la dernière release GitHub, vérifie la signature, installe et relance l’app.

## Développer

Prérequis : Node 22, Rust stable, [dépendances Tauri](https://v2.tauri.app/start/prerequisites/).

```bash
npm install
npm run tauri dev
```

Build local de l’installateur :

```bash
npm run tauri build
```

## Releases

`.github/workflows/release.yml` construit l’installateur Windows (`Check_<version>_x64-setup.exe`, NSIS) sur `windows-latest` à chaque push :

- sur `main` → release `v0.1.<n>` (celle utilisée par la mise à jour intégrée) ;
- sur une autre branche (`claude/**`) → pré-release `v0.1.<n>-dev`.

La version est `0.1.<numéro de run>`, appliquée par `scripts/set-version.mjs`.

## Mises à jour signées

La mise à jour intégrée utilise `tauri-plugin-updater` : la CI signe l’installateur et publie `latest.json` dans la release.
Secrets à créer dans GitHub (Settings › Secrets and variables › Actions) :

- `TAURI_SIGNING_PRIVATE_KEY` : contenu du fichier de clé privée (`npx tauri signer generate`)
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` : son mot de passe

La clé publique correspondante est dans `src-tauri/tauri.conf.json` (`plugins.updater.pubkey`). Sans ces secrets, la CI construit quand même l’installateur, mais sans mise à jour automatique.

## Données

`%APPDATA%\com.maxime272003.check\data.json` — checklists, réglages et position des post-it.
