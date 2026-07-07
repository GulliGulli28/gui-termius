# gui-termius

![Licence](https://img.shields.io/badge/licence-PolyForm%20Noncommercial%201.0.0-blue)
![Plateforme](https://img.shields.io/badge/plateforme-Windows-informational)

Un client SSH / SFTP de bureau, personnel et sans fioritures inutiles — pensé pour
quelqu'un qui jongle avec plusieurs serveurs, bastions et terminaux locaux au
quotidien, et qui en avait assez de recoller les mêmes bouts de workflow dans des
outils génériques. Construit avec Tauri, Rust et React.

Ce n'est pas un produit commercial ni un concurrent de Termius : c'est un outil
pensé pour un usage personnel, qui grandit au fil des besoins réels plutôt que
d'une roadmap figée.

## Fonctionnalités

**Terminaux SSH**
- Connexions directes ou enchaînées via un ou plusieurs bastions/rebonds (comme
  `ssh -J`), agent forwarding, keepalive configurable.
- Plusieurs onglets, vue « split » à deux terminaux côte à côte.
- Diffusion d'une commande vers un sous-ensemble choisi de terminaux ouverts — en
  mode « une commande à la fois » ou en mode « direct » (la frappe dans un
  terminal est répercutée en temps réel vers les autres).
- Reconnexion automatique en cas de coupure, avec délai croissant configurable.
- Recherche dans le scrollback (Ctrl+F), avec options casse/regex, et export du
  scrollback d'un terminal vers un fichier.

**Terminal local**
- Shell système intégré, avec détection automatique des shells disponibles
  (PowerShell, cmd, PowerShell 7, Git Bash, WSL…) et possibilité d'en choisir un
  par défaut ou ponctuellement à l'ouverture.

**SFTP**
- Navigateur à deux panneaux (local ↔ distant ou distant ↔ distant), copie par
  glisser-déposer entre panneaux ou depuis l'Explorateur de fichiers.
- Création de dossiers et de fichiers, renommage, permissions, édition rapide des
  petits fichiers texte sans quitter l'app.

**Snippets & scripts**
- Commandes ou scripts réutilisables avec variables `{{comme_ceci}}`.
- Sélecteur rapide au clavier : on peut taper le nom du snippet suivi de ses
  arguments en une seule fois (`sys start apache2`), ou les remplir un par un si
  besoin.
- Exécution sur le terminal actif ou sur un ensemble choisi de terminaux ouverts.

**Organisation**
- Dossiers hiérarchiques avec icône et couleur (affichée en un coup d'œil sur les
  onglets), recherche, import d'hôtes depuis `~/.ssh/config`, export/import d'un
  hôte ou de l'espace de travail complet.

**Sécurité**
- Vérification des clés d'hôte à la première connexion (« trust on first use »,
  dans l'esprit de `known_hosts`), avec alerte en cas de changement de clé.
- Mots de passe et passphrases stockés dans le trousseau du système
  (Credential Manager sous Windows), jamais en clair sur disque — avec un repli
  en mémoire (le temps de la session) si le trousseau n'est pas disponible.

**Confort**
- Palette de commandes (Ctrl+K), raccourcis clavier personnalisables — avec
  détection des collisions avec les raccourcis shell courants (Ctrl+W, Ctrl+K,
  Ctrl+\, …) pour éviter les surprises.
- Thèmes de terminal (Dracula, Nord, Gruvbox, Solarized…), polices, couleurs
  d'accent, mode clair/sombre.
- Mise à jour automatique de l'application (vérification silencieuse au
  lancement + bouton dans Paramètres).

## Stack technique

- **Backend** : [Tauri 2](https://tauri.app/), Rust. SSH en pur Rust via
  [`russh`](https://github.com/Eugeny/russh) (pas de dépendance au binaire
  système `ssh`), terminaux locaux via `portable-pty`, secrets via `keyring`.
- **Frontend** : React 19, TypeScript, Tailwind CSS, [xterm.js](https://xtermjs.org/)
  pour le rendu des terminaux.
- Le code Rust est séparé en deux : `core/` (logique métier pure, indépendante de
  Tauri — SSH, SFTP, vault, known_hosts, parsing de `~/.ssh/config`…) et
  `src-tauri/` (commandes Tauri qui exposent cette logique au frontend, état
  applicatif, packaging).

## Installation

Actuellement, seule une CI Windows est en place : télécharger le dernier
installeur depuis les [Releases](https://github.com/GulliGulli28/gui-termius/releases)
du dépôt. L'application se met
ensuite à jour toute seule (silencieusement au lancement, ou via Paramètres →
Général → « Vérifier les mises à jour »).

## Développement

Prérequis : [Node.js](https://nodejs.org/) 20+, [Rust](https://rustup.rs/)
stable (toolchain MSVC recommandée sous Windows), WebView2 (déjà présent sur
Windows 10/11 à jour).

```bash
npm install        # dépendances frontend
npm run tauri dev  # lance l'app en mode développement (hot reload)
```

Autres commandes utiles :

```bash
npm run build         # build frontend seul (tsc + vite)
npm run tauri build    # build de production packagé (installeur inclus)
npm run bump-version -- 1.5.0   # bump package.json + Cargo.toml
```

Voir [`RELEASING.md`](RELEASING.md) pour le détail du processus de publication
d'une nouvelle version (tag Git → build CI → publication manuelle du brouillon
de release).

## Structure du projet

```
core/           logique métier Rust, indépendante de Tauri
  ssh.rs          connexions SSH directes et enchaînées (bastions)
  sftp.rs         opérations de fichiers distants
  local_fs.rs     opérations de fichiers locaux (côté SFTP local)
  transfer.rs     copie/upload entre panneaux SFTP
  vault.rs        secrets (trousseau OS + repli mémoire)
  known_hosts.rs  vérification des clés d'hôte (trust on first use)
  ssh_config.rs   parsing/import de ~/.ssh/config
  port_forward.rs tunnels locaux/distants
  store.rs        persistance de l'espace de travail (workspace.json)
  export.rs       export/import d'hôtes ou de l'espace de travail

src-tauri/      pont Tauri
  src/commands/   une commande par domaine (hosts, terminal, sftp, forward, …)
  src/state.rs    état applicatif partagé (sessions actives, workspace en mémoire)

src/            frontend React/TypeScript
  components/     composants d'interface (un fichier par panneau/widget)
  lib/            pont API vers Tauri, préférences, types partagés, raccourcis
```

## Données et configuration

- L'espace de travail (hôtes, dossiers, snippets, tunnels) est stocké en clair
  dans un `workspace.json`, dans le dossier de configuration utilisateur standard
  (géré par la crate `directories` — typiquement
  `%APPDATA%\gui-termius\gui-termius\config\` sous Windows).
- Les secrets (mots de passe, passphrases) ne sont **jamais** dans ce fichier :
  ils vivent dans le trousseau du système d'exploitation.
- Les préférences d'interface (thème, raccourcis, taille de police…) sont
  stockées côté navigateur (`localStorage` de la webview), pas dans
  `workspace.json`.

## Licence

[PolyForm Noncommercial 1.0.0](https://polyformproject.org/licenses/noncommercial/1.0.0)
— usage personnel et non commercial uniquement. Voir [`LICENSE`](LICENSE).
