## graphify

This project has a knowledge graph at graphify-out/ with god nodes, community structure, and cross-file relationships.

Rules:
- For codebase questions, first run `graphify query "<question>"` when graphify-out/graph.json exists. Use `graphify path "<A>" "<B>"` for relationships and `graphify explain "<concept>"` for focused concepts. These return a scoped subgraph, usually much smaller than GRAPH_REPORT.md or raw grep output.
- If graphify-out/wiki/index.md exists, use it for broad navigation instead of raw source browsing.
- Read graphify-out/GRAPH_REPORT.md only for broad architecture review or when query/path/explain do not surface enough context.
- After modifying code, run `graphify update .` to keep the graph current (AST-only, no API cost).

## Le projet en bref

`gui-termius` est un client SSH/SFTP de bureau (Tauri 2 + Rust + React/TypeScript),
à usage personnel (licence PolyForm Noncommercial). Voir `README.md` pour la
présentation complète des fonctionnalités et de la structure du dépôt — ce
fichier-ci se concentre sur ce qui n'est pas évident en lisant juste le code.

Découpage du code Rust :
- `core/` — logique métier pure (SSH via `russh`, SFTP, vault, known_hosts,
  parsing `~/.ssh/config`, persistance du workspace). Ne dépend pas de Tauri.
- `src-tauri/src/commands/` — une commande Tauri par domaine, fine couche
  au-dessus de `core/`. C'est là qu'ajouter une nouvelle commande invocable
  depuis le frontend.
- `src/lib/api.ts` — unique point de passage frontend → Tauri (`invoke(...)`).
  Toute nouvelle commande Rust doit avoir son entrée ici, typée.

## Environnement de dev (important)

Le dépôt est monté depuis WSL (`\\wsl.localhost\Ubuntu-24.04\...`). Pour de la
vérification rapide (`cargo check`, `cargo test`, `tsc`, `npm run build`),
passer par WSL explicitement — c'est l'environnement Rust "par défaut" de ce
projet (`termius-core` y tourne ses tests d'intégration, qui ont besoin d'un
vrai `sshd` Unix) :

```bash
wsl.exe -e bash -lc "cd ~/gui-termius/src-tauri && cargo check"
```

**Rust existe aussi nativement sur Windows sur cette machine** (rustup, MSVC
Build Tools 2026, WebView2 Runtime — tous déjà installés), mais `cargo`/
`rustc`/`tauri-driver` ne sont **pas sur le PATH d'une session PowerShell
fraîche** : soit invoquer par chemin complet
(`$env:USERPROFILE\.cargo\bin\cargo.exe`), soit ajouter au PATH de la session
(`$env:PATH += ";$env:USERPROFILE\.cargo\bin"`) — ne pas conclure « cargo
n'est pas installé côté Windows » juste parce que `Get-Command cargo` échoue.
C'est nécessaire pour le pipeline E2E WebView2 (section suivante), qui doit
tourner nativement sous Windows (WebView2 n'existe pas sous Linux).

Le frontend (`npm`, `npx tsc`, `vite build`) peut tourner indifféremment côté
Windows natif ou via WSL — mais **jamais mélanger les deux pour le même
`node_modules`** : `npm install` choisit des binaires natifs par plateforme
(`esbuild`, `rollup`...) au moment de l'install, donc un `node_modules`
installé sous WSL ne fait pas tourner Vite nativement sous Windows (et
inversement). Ce dépôt n'a qu'un seul `node_modules`, installé côté WSL —
rester cohérent là-dessus pour un même enchaînement de commandes évite les
surprises de cache dupliqué ou d'échecs de résolution de binaire natif.

**Accès écran réel : oui, via WSLg.** Ce WSL a un vrai serveur X actif
(`DISPLAY=:0`, `xdpyinfo`/`xrandr` répondent, résolution réelle détectée) —
voir la section suivante, ce n'est plus une limitation de cet environnement
depuis le 2026-07-07 (une session précédente l'affirmait à tort ; corrigé
après avoir effectivement lancé l'app et pris une vraie capture).

## Vérification Rust : `clippy -D warnings` est un gate CI bloquant

Le workflow GitHub (job **`windows-workspace`**) lance
`cargo clippy --workspace --all-targets -- -D warnings` : **le moindre warning
clippy fait échouer le push**. `cargo check` / `cargo test` ne déclenchent PAS
les lints clippy — il faut donc lancer clippy explicitement avant de considérer
une tâche Rust terminée, sinon le CI casse (déjà arrivé : `ptr_arg` sur
`&PathBuf` au lieu de `&Path`, `collapsible_if`). En local, via WSL :

```bash
wsl.exe -e bash -lc "cd ~/gui-termius && cargo clippy --workspace --all-targets -- -D warnings"
```

Piège : clippy interrompt la compilation d'une crate dès sa première erreur,
donc tant que `termius-core` échoue ses lints, ceux de `gui-termius` restent
invisibles — corriger, relancer, itérer jusqu'à zéro.

## Tests E2E réels — OBLIGATOIRE avant de clore une tâche UI/terminal

`cargo check` / `tsc --noEmit` / `npm run build` prouvent que le code
compile, pas qu'une fonctionnalité marche. Pour toute tâche qui touche à un
composant React, un terminal (`TerminalTab`/`LocalTerminalTab`), une
interaction clavier/souris, ou tout chemin passant par `invoke(...)`, il ne
suffit **pas** de s'arrêter à la compilation : lancer
**`npm run test:e2e`** (voir `scripts/e2e-run.mjs`) fait partie intégrante de
la vérification, au même titre que `cargo check`/`tsc` — pas une étape
optionnelle réservée à « si j'ai le temps ». Si la commande échoue ou que le
setup manque, le dire explicitement plutôt que de conclure sur la seule
compilation.

Ce que fait `npm run test:e2e` : il démarre `tauri-driver` (et Vite si besoin,
voir plus bas), pilote le **vrai binaire compilé** via le protocole WebDriver,
vérifie que la fenêtre s'ouvre, que React a bien monté (`#root`), prend une
vraie capture d'écran (`scripts/.output/e2e-smoke.png`, gitignored) puis
nettoie tous les processus qu'il a lancés. Contrairement à Puppeteer/
Playwright pointé sur `http://localhost:1420` (qui ne voit jamais
`window.__TAURI__` — un vrai navigateur n'est pas une webview Tauri), ceci
exécute du vrai code `invoke(...)`. Le script (`scripts/e2e-run.mjs`) est
**cross-platform** et détecte l'environnement d'exécution :

|                    | Linux (WSLg)                    | Windows natif                          |
|--------------------|----------------------------------|-----------------------------------------|
| Rendu               | WebKitGTK                       | **WebView2** (ce que les utilisateurs lancent réellement) |
| Pilote natif        | `WebKitWebDriver`               | `msedgedriver.exe`                      |
| Build testé         | debug (`cargo build`)           | release (`cargo build --release --features tauri/custom-protocol`) |
| Sert le frontend via| Vite dev server (`devUrl`)      | `dist/` embarqué dans le binaire (`frontendDist`) |

Les deux ont été validés avec succès le 2026-07-07 — voir les pièges
ci-dessous pour la mise en place, non triviale sur les deux plateformes.

**Setup one-time Linux/WSL (déjà fait sur cette machine)** :
```bash
wsl.exe -e bash -lc "sudo apt-get update && sudo apt-get install -y webkit2gtk-driver scrot"
wsl.exe -e bash -lc "cd ~/gui-termius && cargo install tauri-driver"
wsl.exe -e bash -lc "cd ~/gui-termius/src-tauri && cargo build"
```
`sudo` n'a pas d'accès non-interactif dans ce WSL (voir piège plus bas) — si
ce setup doit être refait, demander à l'utilisateur de lancer la commande
`apt-get` lui-même via le préfixe `!`.

**Setup one-time Windows (déjà fait sur cette machine)** — piloté depuis
PowerShell, jamais `wsl.exe` pour cette partie :
```powershell
# Rust : déjà installé sur cette machine (rustup), juste absent du PATH de
# session — toujours invoquer via le chemin complet ou ajouter au PATH :
$env:PATH += ";$env:USERPROFILE\.cargo\bin;C:\Program Files\NASM"

winget install --id NASM.NASM -e --accept-package-agreements --accept-source-agreements
& "$env:USERPROFILE\.cargo\bin\cargo.exe" install tauri-driver

# msedgedriver DOIT correspondre exactement à la version du WebView2 Runtime installé :
$wv2 = (Get-ItemProperty "HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}").pv
curl.exe -sL "https://msedgedriver.microsoft.com/$wv2/edgedriver_win64.zip" -o "$env:TEMP\ed.zip"
Expand-Archive "$env:TEMP\ed.zip" "$env:USERPROFILE\edgedriver" -Force

# Build release, sur un chemin NTFS natif (pas le chemin UNC du dépôt WSL) :
$env:CARGO_TARGET_DIR = "$env:USERPROFILE\gui-termius-target-windows"
Set-Location "\\wsl.localhost\Ubuntu-24.04\home\glorin\gui-termius\src-tauri"
& "$env:USERPROFILE\.cargo\bin\cargo.exe" build --release --features tauri/custom-protocol
```
MSVC Build Tools et le WebView2 Runtime étaient déjà présents sur cette
machine (`vswhere.exe` pour vérifier, `winget` pour installer sinon).

**Avant de lancer `npm run test:e2e`**, s'assurer que le binaire correspondant
à la plateforme existe et est à jour (commandes ci-dessus). Depuis Windows,
définir `$env:CARGO_TARGET_DIR` avant de lancer le script (même valeur qu'au
build) pour qu'il retrouve le bon binaire, et invoquer `node scripts\e2e-run.mjs`
directement plutôt que `npm run test:e2e` (voir piège `npm`/UNC ci-dessous).

Pour étendre la couverture E2E : ajouter des scénarios dans la fonction
`runScenarios()` de `scripts/e2e-run.mjs` (ex. taper dans un terminal local et
vérifier qu'une suggestion apparaît) plutôt que d'écrire un nouveau script
séparé à chaque fois — le même scénario tourne sur les deux plateformes sans
modification.

**Pièges Windows rencontrés en mettant ça en place (dans l'ordre où ils
mordent)** :
- **`aws-lc-sys` a besoin de NASM** pour compiler ses routines assembleur sous
  MSVC — `cargo build` échoue avec `NASM command not found` sinon
  (`winget install NASM.NASM`, puis ajouter `C:\Program Files\NASM` au PATH
  de la session).
- **Lock file de compilation incrémentale impossible à créer sur un chemin
  UNC** (`\\wsl.localhost\...`) — `cargo build` échoue avec `could not create
  session directory lock file: Fonction incorrecte`. Fixé en pointant
  `CARGO_TARGET_DIR` vers un chemin NTFS natif (`$env:USERPROFILE\...`) ; le
  code source reste lu depuis le chemin UNC sans problème, seul le
  répertoire de build doit être natif.
- **`npm run ...` échoue sur un `cwd` UNC** : `npm` passe par `cmd.exe`, qui
  ne supporte pas les chemins UNC comme répertoire courant (retombe
  silencieusement sur `C:\Windows` et ne trouve plus rien). Contournement
  dans `scripts/e2e-run.mjs` : invoquer `node.exe` directement sur les
  fichiers `.js` (`node_modules/vite/bin/vite.js`, `scripts/e2e-run.mjs`
  lui-même) plutôt que passer par les shims `.cmd` (`npx`, `npm run`).
- **Un `node_modules` installé via WSL ne peut pas faire tourner Vite
  nativement sous Windows** : `esbuild` (et d'autres) livrent un binaire
  natif par plateforme, choisi à l'installation — seul le binaire Linux a été
  installé. Plutôt que dupliquer `node_modules` pour Windows, contourné en
  testant un **build release** côté Windows : `frontendDist` (le contenu de
  `dist/`, déjà construit via WSL) est purement statique donc portable, alors
  que la tooling de build (`node_modules`) ne l'est pas.
- **`cargo build --release` seul ne suffit PAS à embarquer `frontendDist`** —
  contre-intuitif : le binaire continue de charger `devUrl`
  (`http://localhost:1420`, échec silencieux si rien n'écoute dessus) même en
  release. Il manque le feature flag Cargo `custom-protocol` sur la
  dépendance `tauri`, que la CLI `tauri build` active automatiquement mais
  qu'un `cargo build` direct n'active jamais (`tauri = { features = [] }`
  dans `Cargo.toml`). Fix : `cargo build --release --features
  tauri/custom-protocol`. Diagnostiqué en interrogeant `getUrl()` via
  WebDriver pendant la session (montrait `http://localhost:1420/` malgré le
  build release) plutôt qu'en devinant.

### Techniques plus légères, sans lancer l'app entière

Pour itérer plus vite qu'un cycle `test:e2e` complet (qui recompile/relance
une vraie fenêtre), deux techniques plus légères, mises en place et
vérifiées avec succès le 2026-07-07 en développant les suggestions de
commandes (ghost-text) des terminaux locaux, restent utiles en complément
(pas en remplacement) du test E2E :

- **Tests unitaires (`npm run test`, vitest)** pour toute la logique pure
  découplée de React/xterm/Tauri — typiquement un état de type "buffer de
  ligne" ou toute fonction `(state, event) => state`. Voir
  `src/lib/lineBuffer.ts` + `src/lib/lineBuffer.test.ts` : la state machine
  qui ombre les frappes clavier pour reconstituer la ligne tapée est testée
  isolément (Ctrl+L, Ctrl+C, Ctrl+U/W, désynchronisation sur Tab/flèches,
  paquets `onData` qui mélangent plusieurs frappes). `vite.config.ts` expose
  déjà un bloc `test` (vitest lit la même config que Vite) — pas de setup
  séparé nécessaire pour de nouveaux fichiers `*.test.ts`.
  **Piège Node** : vitest ≥ 4 exige Node ≥ 20 ; le Node de ce WSL est en
  18.19 → utiliser `vitest@^2` (compatible Node 18), sinon échec immédiat au
  démarrage (`SyntaxError: … does not provide an export named 'styleText'`).

- **Rendu DOM réel dans un navigateur headless (Playwright), sans Tauri.**
  Pour tout ce qui dépend du DOM effectivement produit par xterm.js (mesures
  de cellules, positionnement d'un overlay, alignement visuel) — donc au-delà
  de ce qu'un test unitaire peut couvrir — on peut monter un vrai
  `@xterm/xterm` dans une page headless, lui écrire du texte directement via
  `term.write(...)` (ça ne passe pas par Tauri, donc `invoke()` n'est jamais
  sollicité), puis prendre un vrai screenshot et l'inspecter avec l'outil
  `Read`. Voir `scripts/visual-check-ghost-text.{html,client.mjs,mjs}` :
  `node scripts/visual-check-ghost-text.mjs` sert la page via le serveur Vite
  du projet (réutilise `vite.config.ts`), la charge dans Chromium headless
  (Playwright), vérifie que la géométrie de cellule calculée est plausible et
  que le curseur xterm tombe où attendu, écrit
  `scripts/.output/ghost-text-check.png` (gitignored) et sort en erreur si
  une des assertions échoue. Cette technique a permis de valider la
  transposition du sélecteur `.xterm-rows` (rendu DOM par défaut de xterm.js,
  utilisé quand aucun addon `webgl`/`canvas` n'est chargé) en géométrie
  pixel — l'hypothèse la plus risquée de l'implémentation du texte fantôme —
  sans jamais lancer l'app réelle.
  **Piège install** : `npx playwright install --with-deps chromium` invoque
  `sudo apt-get install …` pour les libs système ; dans cet environnement WSL
  `sudo` n'a pas d'accès non-interactif et la commande reste bloquée
  indéfiniment sur un prompt de mot de passe qui n'arrivera jamais (aucune
  sortie, CPU à 0%, silence total — symptôme caractéristique). Utiliser
  `npx playwright install chromium` (sans `--with-deps`) : le binaire
  Chromium seul suffit en mode headless et ne nécessite aucun privilège. Si
  un futur test échoue au lancement faute de bibliothèque système
  (`libnss3`, etc.), demander à l'utilisateur de lancer lui-même la commande
  `--with-deps` via le préfixe `!` plutôt que de rester bloqué dessus.

Ce que ces deux techniques ne couvrent toujours **pas** : tout ce qui passe
par `invoke(...)` (connexion SSH réelle, PTY local, trousseau, SFTP...) —
`window.__TAURI__` n'existe que dans la vraie webview Tauri, jamais dans un
Chromium/Playwright classique, headless ou non. C'est exactement ce que
`npm run test:e2e` (section précédente) couvre — l'utiliser dès qu'une
commande `invoke(...)` réelle doit être exercée, pas seulement son DOM.

## Stockage des secrets : trousseau OS ou coffre chiffré (opt-in)

`core/src/vault.rs` est le point de passage unique pour les mots de passe et
passphrases, avec un état à 3 modes (`Keychain` / `Locked` / `Unlocked`) — mais
`store`/`load`/`delete` gardent la même signature quel que soit le backend, donc
`ssh::authenticate` et `commands/hosts.rs` n'ont pas à s'en soucier.

- **Par défaut** : trousseau OS (`keyring`), fallback mémoire quand il n'existe
  pas (WSL/headless), perdu au redémarrage.
- **Coffre chiffré (opt-in)** : dès qu'un mot de passe maître est défini, un
  fichier `secrets.enc` (Argon2id + XChaCha20-Poly1305, schéma à enveloppe
  DEK/KEK — `core/src/{crypto,master_vault}.rs`) remplace le trousseau. Portable/
  syncable, marche sans trousseau OS, verrouillé au lancement (UI : modale de
  déverrouillage + Paramètres → Sécurité, auto-lock configurable).

**Cas particulier des clés privées** : leur contenu PEM reste dans
`workspace.json` (0600) en mode trousseau, mais bascule dans le coffre chiffré
quand il est déverrouillé — via `vault::{load,store,delete}_key_content` +
`is_unlocked`, PAS via le trousseau OS (taille limitée sous Windows, et le
fallback WSL le perdrait). `ssh::authenticate` lit le PEM dans l'ordre
**coffre → workspace.json → fichier d'origine** ; la migration dans les deux sens
(activer/désactiver) et l'export/import « avec clés » sont gérés dans
`commands/{vault,export}.rs`.

Ne pas tester le flux complet activer→migrer→se-connecter en E2E automatique :
il faut un vrai `sshd` ET ça mut­erait le `secrets.enc` réel du profil. Le crypto
est couvert par tests unitaires (`crypto.rs`, `master_vault.rs`).

## RDP intégré (rendu réel) : architecture sidecar

Le rendu RDP intégré (`RdpTab.tsx`, onglet « Aperçu intégré ») ne tourne
**pas** dans le binaire principal `gui-termius` : c'est un processus séparé,
`rdp-sidecar`, lancé par `commands/rdp_view.rs` et piloté par un protocole
maison sur stdin/stdout (`rdp-ipc`). Ce n'est pas un choix d'architecture
arbitraire — c'est la seule option qui compile, pour une raison précise
détaillée ci-dessous. Le mode « lanceur » historique (`core/src/rdp.rs`,
commande `connect_rdp`, bouton principal des hôtes RDP — shell-out vers
`mstsc.exe`/`xfreerdp`) reste en place tel quel et reste l'action par défaut,
accessible en un clic ; l'aperçu intégré (phase 2 : affichage + souris/
clavier, toujours pas de presse-papiers/audio/lecteurs partagés) reste une
action secondaire, pas un remplacement.

### Pourquoi un processus RDP séparé

`ironrdp-connector` (client RDP) dépend transitivement de `picky`
(`picky-asn1-x509`), qui pin une version *exacte* de `ecdsa`
(`=0.17.0-rc.22` au moment d'écrire ceci). `russh` — déjà utilisé partout
dans `core/` pour SSH — pin lui aussi une version exacte, mais différente
(`=0.17.0-rc.18`). Deux pins exacts différents de la même crate dans un seul
graphe de dépendances Cargo ne peuvent **jamais** être résolus : ce n'est pas
une histoire de version « pas encore assez récente » (vérifié aussi bien sur
la dernière version publiée de `picky` que sur sa branche `master` — toujours
désaligné avec `russh`), donc pas un problème qu'un `cargo update` ou
l'attente d'un futur release réglerait. Toute tentative d'ajouter
`ironrdp-connector` comme dépendance directe ou transitive de `core/` (ou de
tout crate qui dépend de `core/`, donc `src-tauri` inclus) échoue la
résolution Cargo dès `cargo check`.

**Un membre de workspace n'isole rien.** Première tentative : mettre
`rdp-sidecar` dans le workspace racine (`members = [...]`) sans le faire
dépendre de `core`/`russh`, en supposant que l'absence de dépendance directe
suffirait. Ça échoue avec exactement la même erreur `ecdsa` : Cargo résout
**un seul** graphe de dépendances unifié pour tous les membres d'un même
workspace, qu'ils dépendent les uns des autres ou non. La seule vraie
isolation est un `[workspace]` **séparé** — son propre `Cargo.lock` — relié
au reste du dépôt uniquement par une dépendance `path = "..."` ordinaire vers
un crate qui n'a lui-même aucune dépendance à risque. C'est exactement la
structure ici :

```
Cargo.toml (workspace racine : core, src-tauri, rdp-ipc)
  └─ src-tauri dépend de rdp-ipc (path) — jamais de rdp-sidecar
rdp-sidecar/Cargo.toml ([workspace] séparé, members = ["."])
  └─ dépend de rdp-ipc (path) + ironrdp — jamais de core/russh
```

`rdp-ipc` (protocole de communication, voir plus bas) ne dépend que de
`tokio`/`serde`/`serde_json` — sûr à partager tel quel entre les deux
workspaces sans jamais réintroduire le conflit.

### Build : ce qui est déjà vérifié, ce qui ne l'est pas

`rdp-sidecar` compile et passe `cargo clippy --all-targets -- -D warnings`
propre, à la fois sous WSL (`x86_64-unknown-linux-gnu`) et nativement sous
Windows (`x86_64-pc-windows-msvc`, testé le 2026-07-10 en conditions réelles
sur cette machine — `ironrdp-tokio`'s feature `reqwest-rustls-ring` utilise
`ring`, qui a lui aussi besoin de NASM pour ses routines assembleur sous
MSVC, déjà installé sur cette machine pour la même raison que `aws-lc-sys`
— voir la section E2E). Ce qui n'est **pas** vérifié : une vraie connexion
contre un serveur RDP réel (aucun accessible dans cet environnement — même
limitation que le mode lanceur). La logique de connexion/décodage a été
portée depuis `ironrdp-client/src/rdp.rs` (le client de référence du dépôt
`Devolutions/IronRDP`) en vérifiant chaque type/signature contre les sources
de la version réellement résolue par Cargo plutôt que par mémoire — un
premier essai basé sur une version plus récente de l'API que celle
effectivement résolue (`ActiveStageBuilder`, `AsyncReadWrite` exporté par
`ironrdp-tokio`) a échoué à la compilation et a dû être corrigé contre les
sources réelles (`ActiveStage::new(connection_result)` direct, trait
`AsyncReadWrite` local à définir soi-même — absent de `ironrdp-tokio`).

**Bug réel trouvé au premier test interactif** (2026-07-10, via `npx tauri
dev` + un vrai clic sur « Aperçu intégré » — exactement le genre de chose
qu'aucune compilation/clippy ne peut attraper) : le process `rdp-sidecar`
plantait immédiatement à la première tentative de connexion avec *"Could
not automatically determine the process-level CryptoProvider from Rustls
crate features"*. Cause : `ironrdp-tls`'s chemin rustls appelle
`ClientConfig::builder()` (le constructeur « process default »), qui panique
si aucun `CryptoProvider` n'a été installé — et rustls 0.23 refuse de choisir
implicitement dès que plus d'un provider (`ring` et `aws-lc-rs`) se retrouve
dans le graphe de dépendances, ce qui est le cas ici (`reqwest`/`ironrdp-tls`
ne s'accordent pas sur un défaut). Fix : dépendance directe sur `rustls`
(juste pour appeler l'API) + `rustls::crypto::ring::default_provider()
.install_default()` tout au début de `main()`, avant toute connexion — voir
`rdp-sidecar/src/main.rs`. Ce genre de panique runtime-only, invisible à
`cargo check`/`clippy`, est exactement pourquoi la mention « non testé contre
un vrai serveur » plus haut est prise au sérieux plutôt que traitée comme un
détail — le code compilait proprement et plantait quand même dès le premier
usage réel.

Pour builder `rdp-sidecar` (son propre workspace, donc `cd` dedans avant
toute commande `cargo`) :

```bash
# WSL/Linux
wsl.exe -e bash -lc "cd ~/gui-termius/rdp-sidecar && cargo build --release"
```
```powershell
# Windows natif (mêmes pièges de PATH/NASM que la section E2E)
$env:PATH += ";$env:USERPROFILE\.cargo\bin;C:\Program Files\NASM"
Set-Location "\\wsl.localhost\Ubuntu-24.04\home\glorin\gui-termius\rdp-sidecar"
& "$env:USERPROFILE\.cargo\bin\cargo.exe" build --release
```

### Où placer le binaire compilé

`tauri.conf.json` déclare `bundle.externalBin: ["binaries/rdp-sidecar"]`.
Deux emplacements différents comptent, pour deux étapes différentes :

- **Pour `tauri build`/`tauri-action`** (packaging, `release.yml`) : le
  binaire compilé doit être copié vers
  `src-tauri/binaries/rdp-sidecar-<triple-cible>[.exe]` (suffixe de triple
  obligatoire — c'est ainsi que `tauri build` sait choisir le bon binaire par
  plateforme). `release.yml` le fait déjà pour les deux plateformes du
  matrix, juste avant l'étape `tauri-action`.
- **Pour un `cargo build` direct côté `src-tauri`, ou `cargo run`/`tauri
  dev`** : `tauri_plugin_shell::Command::sidecar()` résout le binaire au
  runtime en le cherchant **à côté de l'exécutable principal en cours
  d'exécution, sans suffixe de triple**
  (`<dossier de gui-termius[.exe]>/rdp-sidecar[.exe]`, donc
  `target/debug/rdp-sidecar` en dev). Contrairement à ce qu'on pourrait
  supposer, ce n'est **pas** une copie faite par la CLI `tauri dev`/`tauri
  build` elle-même — c'est `tauri-build`'s `build.rs` (déclaré en
  `[build-dependencies]` de `src-tauri`, donc déclenché par **n'importe
  quel** `cargo build`/`cargo check`/`cargo run` sur `gui-termius`, CLI
  Tauri ou pas) qui copie automatiquement depuis
  `src-tauri/binaries/rdp-sidecar-<triple-hôte>[.exe]` vers cet emplacement
  sans suffixe. Vérifié le 2026-07-10 : supprimer
  `target/debug/rdp-sidecar` puis relancer un `cargo build -p gui-termius`
  ne le recrée pas tant que `build.rs` n'a pas de raison de re-tourner (il
  ne re-tourne que sur ses `rerun-if-changed`, notamment `tauri.conf.json`
  et `capabilities/`) ; forcer sa ré-exécution (`touch tauri.conf.json`)
  recrée bien la copie automatiquement, sans intervention manuelle. Donc :
  **la seule chose à placer à la main** est le binaire triple-suffixé dans
  `src-tauri/binaries/` (voir ci-dessous) — dès qu'il y est, n'importe quel
  `cargo build`/`cargo run`/`tauri dev` sur `gui-termius` qui redéclenche
  `build.rs` s'occupe de la copie finale. Exemple WSL (chemin utilisé pour
  le test E2E réel du 2026-07-10) :
  ```bash
  wsl.exe -e bash -lc "cd ~/gui-termius/rdp-sidecar && cargo build && \
    cp target/debug/rdp-sidecar ../src-tauri/binaries/rdp-sidecar-x86_64-unknown-linux-gnu"
  ```
  **Piège** : ce même `build.rs` vérifie que le chemin `bundle.externalBin`
  existe **même pour un simple `cargo check`** sur `gui-termius` — sans le
  fichier triple-suffixé ci-dessus déjà en place, même la compilation du
  crate principal échoue (`resource path "binaries/rdp-sidecar-<triple>"
  doesn't exist`), pas seulement le packaging. `src-tauri/binaries/` est
  gitignored (binaire par plateforme, jamais committé) — après un `git
  clone` frais ou un changement de plateforme de build, ce binaire doit
  être reconstruit et recopié avant que `cargo check`/`cargo build`/`tauri
  dev` sur `gui-termius` refonctionne.

  `npx tauri dev` fonctionne pour tester l'app avec ce sidecar déjà en
  place, lancé depuis WSL (vérifié le 2026-07-10 : `beforeDevCommand`
  démarre Vite, `cargo run` compile et lance une vraie fenêtre WebKitGTK,
  aucune erreur liée au sidecar). Deux limites à garder en tête : (a) ça
  reste le rendu **WebKitGTK** (voir le tableau de la section E2E), pas
  WebView2 — pour tester le rendu que les utilisateurs auront réellement il
  faut le binaire Windows (`rdp-sidecar-x86_64-pc-windows-msvc.exe`, même
  procédure de build que la section précédente) et lancer `npx tauri dev`
  **depuis PowerShell natif**, jamais testé sur cette machine ; vu le piège
  déjà documenté (`npm run ...` échoue sur un `cwd` UNC, section E2E), ça
  pourrait buter sur la même limitation — à vérifier avant de compter
  dessus. (b) `tauri dev` ne teste que le lancement du process sidecar et
  la connexion IPC/UI — **pas une vraie session RDP** : il faut un hôte de
  type RDP déjà configuré dans l'app (avec un mot de passe enregistré) et
  un serveur RDP réellement joignable, ce qu'aucun environnement de dev
  utilisé jusqu'ici ne fournit.

### CI : ce que `--workspace` ne couvre pas

Les commandes `cargo clippy --workspace`/`cargo build --workspace` du job
`windows-workspace` (`.github/workflows/ci.yml`) résolvent le workspace
racine (`core` + `src-tauri` + `rdp-ipc`) — **elles ne touchent jamais
`rdp-sidecar`**, exactement pour la raison qui a motivé sa séparation
(isolation de workspace). Il n'y a pour l'instant **aucun job CI dédié** qui
compile/lint `rdp-sidecar` : un warning clippy ou une régression de
compilation là-dedans passerait inaperçu jusqu'à ce que quelqu'un tente une
release. À ajouter si ce code continue d'évoluer (job séparé, `cd
rdp-sidecar && cargo clippy --all-targets -- -D warnings`, sur
`ubuntu-latest` a minima — Windows recommandé aussi vu le lien avec
`ring`/NASM ci-dessus).

### Protocole `rdp-ipc`

`rdp-ipc/src/lib.rs` définit la trame entre les deux processus, dans les deux
sens, testée par 10 tests unitaires (`tokio::io::duplex`, aucun process réel
nécessaire) :
- **stdin du sidecar** : d'abord une unique ligne JSON `ConnectRequest {
  host, port, username, password }`, envoyée une fois au lancement, puis un
  flux continu de lignes JSON `ClientMessage` pour le reste de la vie du
  process — `MouseMove { x, y }`, `MouseButton { x, y, button, pressed }`
  (`button` = `MouseEvent.button` DOM brut), `MouseWheel { x, y, deltaY }`,
  `Key { code, pressed }` (`code` = `KeyboardEvent.code` DOM, indépendant du
  layout clavier), `ReleaseAll` (relâche tout côté serveur — envoyé par
  `RdpTab.tsx` sur perte de focus/visibilité, pour éviter une touche/bouton
  qui reste « collé »). **Piège vérifié empiriquement, pas juste supposé** :
  `#[serde(rename_all = "camelCase")]` sur un enum à tag interne ne renomme
  que les noms de variantes (la valeur de `"type"`), **pas** les champs des
  variantes struct — `delta_y` restait `delta_y` en JSON malgré l'attribut
  d'enum, nécessitant un `#[serde(rename = "deltaY")]` explicite sur ce
  champ précis. Un décalage de casse ici échoue silencieusement côté
  désérialisation Rust (aucune erreur de compilation d'aucun côté) ; couvert
  par un test dédié (`mouse_wheel_delta_y_field_is_camel_case_on_the_wire`)
  plutôt que seulement par le roundtrip Rust→Rust (qui ne prouve rien sur la
  casse réelle du JSON, seulement que l'écriture et la lecture Rust
  s'accordent entre elles).
- **stdout du sidecar** : une suite de `SidecarMessage` — `Image { width,
  height, pixels }` (RGBA8 brut, toute l'image renvoyée à chaque mise à jour,
  pas de diff — simple et correct pour l'instant, à revisiter si la bande
  passante devient un problème réel), `Error(String)`, ou `Closed` — encodés
  en tag-byte + longueur préfixée (pas de framing texte, contrairement à
  `ConnectRequest`/`ClientMessage` : les pixels bruts pourraient contenir
  n'importe quel octet, y compris `\n`).

Côté `rdp-sidecar`, `main.rs` garde `stdin` ouvert après avoir lu le
`ConnectRequest` et lance une tâche séparée qui boucle sur
`ClientMessage::read_from` jusqu'à fermeture/erreur, poussant chaque message
dans un `mpsc::unbounded_channel` — lu par `active_session`'s `tokio::select!`
en parallèle de `reader.read_pdu()`. Piège évité : un `mpsc::Receiver` fermé
renvoie `None` immédiatement à *chaque* poll, donc le sélectionner sans
précaution ferait tourner la boucle en boucle infinie CPU dès que stdin se
ferme — `recv_or_pending` bascule la branche sur un `std::future::pending()`
après le premier `None`, qui ne se résout plus jamais.

La conversion `ClientMessage` → PDU RDP passe par `ironrdp::input` (feature
`input` de la crate `ironrdp`, alias vers `ironrdp-input`) : son
`Database::apply(operations)`/`release_all()` porte déjà toute la machine à
états presser/relâcher et l'encodage en `FastPathInputEvent`, y compris
`MouseButton::from_web_button()` qui convertit directement la valeur DOM
`MouseEvent.button`. La seule pièce qu'il ne fournit pas : convertir
`KeyboardEvent.code` (ex. `"KeyA"`, `"ArrowLeft"`) en scancode PS/2 Set 1 —
table faite à la main dans `rdp-sidecar/src/input.rs::scancode_for`, capable
mais pas exhaustive (couvre lettres/chiffres/ponctuation/flèches/modifieurs/
F1-F12/pavé numérique ; touches média et impr-écran volontairement absentes).
Un code non reconnu est silencieusement ignoré plutôt que deviné.

Côté `commands/rdp_view.rs`, le sidecar est lancé via
`app.shell().sidecar("rdp-sidecar")` (`tauri_plugin_shell`) avec
`.set_raw_out(true)` — **obligatoire** : sans ce flag, le plugin découpe
stdout ligne par ligne pour le mode par défaut (pensé pour des logs texte),
ce qui corromprait silencieusement le framing binaire de `SidecarMessage` dès
qu'un octet `\n`/`\r` apparaît dans des pixels ou une longueur. Les
`CommandEvent::Stdout` bruts (chunks de taille arbitraire, pas alignés sur
les trames) sont réassemblés via un `tokio::io::duplex()` : un premier task
recopie les chunks dedans, un second relit des `SidecarMessage` complets avec
`rdp_ipc::SidecarMessage::read_from` — réutilisant le même parseur testé côté
`rdp-ipc`, plutôt que de dupliquer la logique de framing dans
`rdp_view.rs`. Chaque `SidecarMessage::Image` est réémis vers le frontend en
événement Tauri `rdp-view-frame` (pixels ré-encodés en base64, même
convention que `terminal-data`) ; `RdpTab.tsx` les peint directement sur un
`<canvas>` via `ImageData`/`putImageData` — pas de bibliothèque de rendu.

**Aucune entrée de capability n'est nécessaire** dans
`src-tauri/capabilities/default.json` pour `tauri-plugin-shell` : les
permissions de capacités ne gouvernent que les appels `invoke()` du
frontend vers les commandes *du plugin lui-même* (`plugin:shell|spawn`,
etc.) — jamais utilisées ici. `app.shell().sidecar(...)` est appelé
uniquement côté Rust, à l'intérieur de la commande `connect_rdp_view` (elle
déjà exposée au frontend via `invoke_handler`, comme toute commande maison
de ce projet) : c'est un appel Rust-vers-Rust qui ne passe jamais par le pont
IPC/capabilities.

### Portée phase 2 et limites connues

Le forward souris/clavier (`send_rdp_view_input`, `RdpTab.tsx`) est
implémenté et a été **validé pour de vrai contre un serveur RDP réel** le
2026-07-10 par l'utilisateur (voir plus haut : premier test réel avait
révélé le bug de `CryptoProvider`, corrigé, puis une connexion + interaction
réelles ont fonctionné, via `npx tauri dev` sous WSL). Côté frontend,
`RdpTab.tsx` :
- Coalesce `mousemove` à une frame d'animation max (`requestAnimationFrame`)
  plutôt que d'envoyer un événement IPC par pixel parcouru.
- Capture le relâchement de bouton au niveau `window`, pas juste sur le
  `<canvas>` : un drag qui sort du canvas avant le `mouseup` doit quand même
  être vu, sinon le bouton reste « collé » côté serveur RDP.
- Attache `wheel` manuellement via `addEventListener(..., { passive: false
  })` plutôt que la prop JSX `onWheel` — React délègue cet événement en mode
  passif par défaut (perf de scroll), ce qui rendrait `preventDefault()`
  silencieusement sans effet et laisserait la page défiler au lieu de la
  session distante.
- Réutilise `shouldBubbleToShortcut` (`lib/shortcuts.ts`, même mécanisme que
  `TerminalTab`) pour laisser passer les raccourcis de l'appli (changement
  d'onglet, etc.) plutôt que de tout intercepter aveuglément.
- Envoie `ReleaseAll` quand la vue perd le focus ou que l'onglet devient
  inactif, pour éviter qu'une touche modificatrice reste « enfoncée » côté
  serveur après un alt-tab ou un changement d'onglet.

**Presse-papiers (CLIPRDR) — synchronisation automatique bidirectionnelle,
Windows uniquement.** Option volontairement plus ambitieuse que le forward
souris/clavier : l'utilisateur a explicitement choisi cette voie plutôt
qu'une alternative plus simple (déclenchement manuel du côté local→distant)
après que je lui aie présenté les deux avec leurs coûts respectifs. Entièrement
contenu dans `rdp-sidecar` (`clipboard.rs`) — **aucun nouveau message
`rdp-ipc`, aucune nouvelle commande Tauri, aucun changement frontend** : le
sidecar parle directement au presse-papiers OS, une ressource partagée au
niveau du système donc déjà visible de n'importe quel process, pas besoin de
la faire transiter par l'app principale.

- `ironrdp-cliprdr-native`'s `WinClipboard` (Windows) fait tout le travail
  OS-spécifique (lecture/écriture réelle du presse-papiers, négociation de
  format) ; `StubClipboard` (toute autre plateforme) est un backend complet
  qui ne fait rien plutôt qu'une implémentation partielle — le canal CLIPRDR
  est quand même négocié avec le serveur, il ne produit/n'accepte simplement
  jamais de données. Aucune capacité fichier n'est annoncée par l'un ou
  l'autre (`client_capabilities()` renvoie `empty()`), donc pas de transfert
  de fichiers par presse-papiers — texte seulement.
- **Piège central, propre à ce choix** : `WinClipboard` s'appuie sur
  `WM_CLIPBOARDUPDATE` livré à une fenêtre cachée qu'elle possède — ce qui
  exige qu'un vrai thread Win32 tourne une boucle de messages
  (`GetMessageW`/`TranslateMessage`/`DispatchMessageW`) quelque part dans le
  process. `rdp-sidecar` est un process tokio pur, sans jamais aucune boucle
  de messages Win32 — contrairement à `ironrdp-client` (le client de
  référence) qui en a une gratuitement via sa propre fenêtre applicative
  (winit). Solution : un thread OS dédié (`std::thread::spawn`, jamais
  joint — tourne pour toute la durée du process), sur lequel `WinClipboard`
  est créée (elle est `!Send` : liée à la fenêtre/au thread qui l'a créée,
  ne peut pas être construite ailleurs puis déplacée) et où la boucle de
  messages tourne indéfiniment. Le `backend_factory()` qu'elle produit, lui,
  est `Send` et remonte vers le monde async via un `tokio::sync::oneshot`
  attendu avec un simple `.await` — pas de `blocking_recv()` ni d'autre
  primitive de blocage à l'intérieur du runtime tokio.
- Les messages sortants (`ClipboardMessage::SendInitiateCopy`/
  `SendFormatData`/`SendInitiatePaste`, émis par le backend quand le
  presse-papiers local change ou que le serveur demande des données)
  remontent via un `mpsc::UnboundedSender` — sûr à appeler depuis un thread
  non-tokio, `send()` n'est pas bloquant et ne requiert pas d'exécuteur.
  Troisième branche du `tokio::select!` de `active_session` (même piège
  « channel fermé = busy-loop » que pour `input_rx`, `recv_or_pending`
  généralisé pour servir les deux) : dépile ces messages et pilote
  `active_stage.get_svc_processor_mut::<CliprdrClient>()` (`initiate_copy`/
  `submit_format_data`/`initiate_paste`) pour produire la trame à envoyer.
- **Vérifié** : compilation + `clippy --all-targets -- -D warnings` propres
  à la fois sous WSL (chemin `StubClipboard`, aucun appel Win32) et
  nativement sous Windows (chemin `WinClipboard` réel, y compris les appels
  `unsafe` à `GetMessageW`/`DispatchMessageW` — leurs signatures exactes
  dans `windows` 0.62.2 n'étaient pas connues à l'avance, vérifiées par la
  compilation plutôt que par mémoire). **Validé pour de vrai le 2026-07-10**
  par l'utilisateur, copier-coller dans les deux sens, contre un vrai
  serveur RDP — nativement sous Windows (`gui-termius.exe` natif pointé sur
  le serveur Vite de WSL, port-forwarding WSL2 automatique confirmé
  fonctionnel plutôt que supposé). C'est la seule partie de l'aperçu intégré
  qui ne peut **pas** être exercée via `npx tauri dev` sous WSL seul (voir
  plus haut) : `StubClipboard` y tourne silencieusement à la place de
  `WinClipboard`, donc un test sous WSL seul ne prouverait rien sur le
  chemin réel — un lancement natif Windows est nécessaire (Vite peut rester
  côté WSL, joignable depuis Windows via le port-forwarding).

**Redimensionnement dynamique — résolution suit la taille de l'onglet.**
Ajouté après le forward souris/clavier, sur demande explicite de
l'utilisateur (« je peux redimensionner cette fenêtre, ça doit pouvoir
s'adapter »). Deux moments distincts partagent le même mécanisme serveur :

- **Taille initiale** : `ConnectRequest` transporte désormais `width`/
  `height`, mesurés par `RdpTab.tsx` sur son conteneur (toujours mis en page
  via flex, même avant que le canvas ne devienne visible — pas la peine
  d'attendre la première frame) au moment de `connect_rdp_view`, plutôt
  qu'une résolution fixe codée en dur. `build_config` les fait passer par
  `MonitorLayoutEntry::adjust_display_size` (`ironrdp_displaycontrol::pdu`)
  avant de les mettre dans `DesktopSize` : MS-RDPEDISP exige 200..=8192 et
  une largeur paire.
- **Redimensionnement en cours de session** : `RdpTab.tsx` observe son
  conteneur via `ResizeObserver`, débounce à 400 ms (chaque redimensionnement
  déclenche un aller-retour Display Control **et** une séquence de
  réactivation complète côté serveur — pas un simple redraw local, donc pas
  question d'en envoyer un par frame intermédiaire d'un drag de fenêtre) et
  envoie `ClientMessage::Resize { width, height }`. Côté `rdp-sidecar`,
  `active_stage.encode_resize(width, height, None, None)` (Display Control
  Virtual Channel, déjà attaché sans condition dans `build_connector` depuis
  la phase 1) encode la demande ; si le canal n'est pas disponible/pas encore
  connecté (`encode_resize` renvoie `None`), la demande est simplement
  ignorée — la session reste à la résolution courante plutôt que de tenter
  le fallback plus lourd du client de référence (reconnexion complète avec
  nouvelle taille, `RdpControlFlow::ReconnectWithNewSize` — non porté, coût
  jugé disproportionné pour un cas déjà rare avec les serveurs modernes).
- **Séquence de Désactivation-Réactivation** (MS-RDPBCGR §1.3.1.3) : la
  réponse du serveur à un resize accepté (et plus généralement à tout
  `ActiveStageOutput::DeactivateAll`, y compris ceux que le serveur
  déclenche de son propre chef). Longtemps traité comme fatal
  volontairement (« pas testable sans serveur réel ») — implémenté
  maintenant que le test réel contre un vrai serveur (voir plus haut,
  presse-papiers) a validé que l'architecture générale fonctionne.
  `handle_deactivate_all` (nouveau, `main.rs`) rejoue le mini
  échange capacités/finalisation en pilotant directement `reader`/`writer`
  (`ironrdp_tokio::single_sequence_step_read`, même primitive que celle
  utilisée par la connexion initiale) jusqu'à
  `ConnectionActivationState::Finalized`, puis reconstruit `image`
  (nouvelle résolution) et repointe `active_stage` dessus
  (`set_fastpath_processor`/`set_share_id`/`set_enable_server_pointer`).
  Porté depuis `ironrdp-client/src/rdp.rs`, mais adapté à l'API réellement
  résolue plutôt que copié tel quel — encore une fois deux signatures
  différentes de ce que la référence (une version plus récente) utilise :
  `ActiveStageOutput::DeactivateAll` transporte directement un
  `Box<ConnectionActivationSequence>` tout prêt (pas besoin d'un
  `activation_factory.create()` séparé comme dans la référence), et
  `ConnectionResult.connection_activation` (pas `activation_factory`) est le
  nom du champ correspondant en amont — vérifié en lisant les sources
  réelles de `ironrdp-connector` 0.9.0/`ironrdp-session` 0.10.0 avant
  d'écrire le code, pas supposé par analogie avec la référence.
- **Bug réel n°1 trouvé au premier test interactif** (2026-07-10,
  redimensionnement contre un vrai serveur RDP via l'app native Windows) :
  plantage immédiat au premier redimensionnement, `"traitement d'une trame :
  [Fast-Path ...] custom error"`. Cause : `handle_deactivate_all`
  reconstruisait le fast-path processor avec `bulk_decompressor: None`
  **sans condition** — alors que `build_config` négocie
  `compression_type: Some(CompressionType::K64)` et que la construction
  INITIALE (`ActiveStage::new`, code interne à `ironrdp-session`, non
  modifié) construit elle bien un décompresseur à partir de ce type négocié.
  Une fois la séquence de réactivation terminée, le serveur continue
  d'envoyer des mises à jour **compressées** (la compression se négocie une
  fois au niveau transport, pas rejouée à chaque réactivation) — sans
  décompresseur, la toute première mise à jour compressée échoue à se
  décoder. Fix initial (2026-07-10) : reconstruire un `BulkCompressor` frais
  à chaque réactivation via `build_bulk_decompressor` (nouveau, `main.rs`),
  qui reproduisait `to_bulk_compression_type` — une fonction **privée** de
  `ironrdp-session`, donc pas réutilisable telle quelle — en dépendance
  directe sur `ironrdp-bulk`.
- **Bug réel n°2 trouvé au retest** (2026-07-11, même scénario, une fois un
  environnement de test Windows natif à nouveau disponible) : le fix n°1
  arrêtait bien le plantage, mais l'affichage RDP restait **définitivement
  noir** après un redimensionnement — sans plus aucune erreur ni fermeture
  de session, donc rien à lire dans l'onglet. Diagnostiqué en lançant
  `gui-termius.exe` directement (pas `Start-Process`, qui ne capture rien)
  avec stdout/stderr redirigés vers un fichier, pour lire les logs `debug!`
  d'`ironrdp-session` (activés via `RUST_LOG`, voir l'infrastructure
  conservée plus bas) : les compteurs cumulés `total_compressed`/
  `total_uncompressed` loggués à chaque trame **repartaient de zéro** juste
  après chaque réactivation — un tout nouveau `BulkCompressor`, donc un
  historique glissant MPPC vide, était construit à chaque fois. Or cet
  historique doit rester continu avec celui du serveur, jamais réinitialisé
  de son côté : une Deactivation-Reactivation Sequence renégocie les
  capacités, elle ne relance pas la compression bulk au niveau transport. Un
  décompresseur frais et désynchronisé produit un flux dont la **longueur
  reste correcte** (elle dépend uniquement du bitstream compressé reçu, pas
  du contenu de l'historique local) mais dont le contenu est faux, et
  certaines trames finissent par échouer au décodage de structure en aval
  (`BitmapData::decode`, `"not enough bytes"`) — exactement le symptôme du
  bug n°1, simplement retardé de quelques trames au lieu d'immédiat, une
  fois le décompresseur *absent* remplacé par un décompresseur *présent mais
  désynchronisé*.
- **Fix final** : `handle_deactivate_all` n'appelle plus du tout
  `active_stage.set_fastpath_processor(...)` — le `fast_path::Processor`
  existant (avec son `bulk_decompressor`/`complete_data`/`pointer_cache`/
  `palette` intacts) reste en place tel quel à travers la réactivation ;
  seuls `image` (nouvelle taille), `share_id` et `enable_server_pointer`
  sont mis à jour via les setters dédiés d'`ActiveStage`
  (`set_share_id`/`set_enable_server_pointer`, déjà existants pour un autre
  usage). Contrepartie acceptée : le `share_id`/`io_channel_id`/
  `user_channel_id` internes au processor (utilisés uniquement pour encoder
  `FrameAcknowledgePdu`, un indice de pacing bande-passante, pas le rendu)
  restent ceux de la connexion initiale — `fast_path::Processor` n'expose
  aucun setter pour les corriger isolément, et c'était le seul compromis
  possible sans forker `ironrdp-session`. `build_bulk_decompressor`, la
  dépendance directe sur `ironrdp-bulk`, et le paramètre `compression_type`
  de `handle_deactivate_all` ont tous été supprimés (plus nécessaires, la
  fonction n'existe plus).
- **Vérifié** : compilation + `clippy --all-targets -- -D warnings` propres
  nativement sous Windows pour les deux bugs et leurs fixs (WSL indisponible
  pour la seconde vérification — panne réseau DNS temporaire sur cette
  machine ce jour-là, sans lien avec le code). **Le scénario complet
  (connexion RDP réelle + plusieurs redimensionnements de la fenêtre
  principale) a été validé pour de vrai par l'utilisateur le 2026-07-11**,
  sans erreur ni écran noir.
- **Infrastructure de diagnostic ajoutée en marge (conservée, pas
  temporaire)** :
  - `commands/rdp_view.rs` : la tâche qui relaie les événements du sidecar se
    contentait auparavant d'ignorer silencieusement `CommandEvent::Error`/
    `CommandEvent::Terminated` en cas de plantage réel (panique) du process
    — invisible pour l'utilisateur en release (`windows_subsystem =
    "windows"`, aucune console attachée). Elle capture maintenant les 4
    derniers Ko de stderr et, sur une sortie anormale (code ≠ 0) ou une
    erreur de pipe, émet un vrai `rdp-view-error` avec ce texte — visible
    directement dans l'onglet plutôt qu'un « Session RDP terminée »
    générique sans cause. Utile pour tout futur plantage du sidecar, pas
    seulement celui-ci ; et le fait que ce chemin ne se soit PAS déclenché
    pour le bug n°2 (le sidecar ne plantait pas, il continuait de tourner en
    boucle sur des trames désynchronisées) a aidé à orienter le diagnostic
    vers autre chose qu'un crash process.
  - `rdp-sidecar` : `tracing-subscriber` a maintenant la feature
    `env-filter`, et `main.rs` construit son subscriber via
    `EnvFilter::try_from_default_env()` (repli sur `"info"` si absent —
    même comportement qu'avant par défaut). Auparavant `fmt().init()`
    ignorait totalement `RUST_LOG` (feature absente de
    `tracing-subscriber`), rendant les `debug!`/`trace!` internes
    d'`ironrdp-session` impossibles à activer sans recompiler le tout. Pour
    un futur diagnostic similaire : ajouter
    `.env("RUST_LOG", "ironrdp_session=debug")` (ou toute autre cible de
    log) sur le `Command` du sidecar dans `connect_rdp_view`
    (`commands/rdp_view.rs`) avant `.spawn()`, provisoirement.

Limites connues restantes :
- **Pas de curseur rendu.** Les événements `PointerDefault`/`PointerHidden`/
  `PointerPosition`/`PointerBitmap` sont ignorés côté `rdp-sidecar`.
- **Molette approximative.** Chaque événement `wheel` du navigateur envoie
  un cran fixe (±120, la valeur RDP conventionnelle) dans le sens du signe
  de `deltaY`, pas la magnitude réelle — évite un piège de troncature côté
  encodage RDP (`MousePdu` tronque la rotation sur un octet signé ; relayer
  un `deltaY` de navigateur brut sur un gros geste de scroll risquerait un
  wraparound n'importe quoi).
- **Pas de fallback reconnexion si le Display Control Virtual Channel est
  indisponible.** Un redimensionnement demandé dans ce cas est simplement
  ignoré (voir plus haut) plutôt que de reconnecter avec la nouvelle taille
  — acceptable pour l'instant (rare avec des serveurs RDP modernes), mais à
  garder en tête si un serveur plus ancien est testé un jour.

## Pièges déjà rencontrés (pour ne pas les redécouvrir)

- **Drag-and-drop natif vs Tauri.** Sur Windows, le drag-and-drop OS-level de
  Tauri (nécessaire pour déposer des fichiers depuis l'Explorateur, cf.
  `dragDropEnabled` / `onDragDropEvent`) désactive le drag-and-drop HTML5 natif
  du navigateur pour toute la fenêtre. Résultat : un `draggable`/`onDragStart`
  classique ne fonctionne pas pour un drag *interne* à l'app (ex. glisser un
  fichier entre deux panneaux SFTP) tant que ce mécanisme OS reste actif. La
  solution retenue ici (voir `TransferTab.tsx`, `TabBar.tsx`) : implémenter le
  drag interne à la souris (`mousedown`/`mousemove`/`mouseup`) plutôt qu'avec
  l'API HTML5 Drag and Drop.

- **xterm.js avale les raccourcis clavier.** xterm.js appelle
  `stopPropagation()` sur toute touche qu'il traite lui-même (dès que
  `attachCustomKeyEventHandler` ne renvoie pas explicitement `false`). Un
  raccourci global écouté en bulle sur `window` ne se déclenche donc **jamais**
  tant qu'un terminal a le focus — ce qui est presque tout le temps. Le
  correctif n'est pas de passer l'écoute globale en phase de capture (ça
  casserait des raccourcis shell essentiels, voir point suivant) : chaque
  `TerminalTab`/`LocalTerminalTab` laisse explicitement passer (renvoie `false`
  pour) les combinaisons qui correspondent à un raccourci app connu pour ne pas
  entrer en collision avec le shell (`shouldBubbleToShortcut` dans
  `lib/shortcuts.ts`).

- **Collisions raccourcis app ↔ shell.** Plusieurs combinaisons Ctrl+lettre
  « naturelles » sont déjà prises par readline/le shell : Ctrl+W (supprime le
  mot précédent), Ctrl+K (kill-line), Ctrl+U (kill jusqu'au début de ligne),
  Ctrl+\ (SIGQUIT), Ctrl+R (recherche d'historique). Avant de proposer une
  combinaison par défaut pour une nouvelle action, vérifier `shellBindingWarning`
  dans `lib/shortcuts.ts` (et l'étendre si la collision n'y est pas déjà
  répertoriée) plutôt que de la découvrir après coup.

- **Préférences = `localStorage` de la webview, pas un fichier.** Changer une
  valeur par défaut dans `DEFAULT_PREFERENCES` (`lib/preferences.ts`) n'a aucun
  effet rétroactif sur une installation déjà utilisée : la valeur précédente
  reste persistée. Utile à savoir avant de dire à l'utilisateur qu'un nouveau
  défaut « s'applique » — ça ne concerne que les prefs jamais modifiées/jamais
  sauvegardées.

- **Compat ascendante du `workspace.json`.** Toute nouvelle propriété ajoutée à
  un struct Rust sérialisé dans le workspace (`Host`, `Group`, `Snippet`, …)
  doit être `#[serde(default)]` (ou `Option<T>` avec default) pour rester
  compatible avec les fichiers déjà sauvegardés par les utilisateurs existants.

- **`sudo` dans ce WSL n'a pas d'accès non-interactif.** Toute commande qui
  invoque `sudo` (directement, ou en cascade via `npx playwright install
  --with-deps`) reste bloquée indéfiniment sur un prompt de mot de passe qui
  n'arrivera jamais — silence total, 0% CPU, aucune sortie : c'est le
  symptôme caractéristique, pas une commande lente. Ne pas attendre plus
  longtemps en espérant que ça débloque. Tuer le processus bloqué (`kill -9`,
  pas `sudo pkill` qui a le même problème) et demander à l'utilisateur de
  lancer la commande `sudo` lui-même via le préfixe `!`.

- **Un process lancé en arrière-plan via `wsl.exe -e bash -lc "cmd &"` meurt
  dès que cet appel `wsl.exe` se termine** (même avec `nohup`) — ce n'est PAS
  un process persistant. Pour un serveur de longue durée (Vite, tauri-driver,
  un serveur de test), utiliser le paramètre `run_in_background: true` de
  l'outil Bash lui-même sur la commande *au premier plan* (sans `&` interne)
  — c'est ce mécanisme, pas un `&` shell, qui garde le process vivant entre
  deux appels d'outil.

- **GTK sous WSLg rend en Wayland natif par défaut, invisible pour les outils
  X11** (`scrot`, `xwininfo`, et le pilote `WebKitWebDriver` utilisé pour les
  tests E2E). La fenêtre existe et le process tourne, mais n'apparaît dans
  aucun arbre de fenêtres X11 et un screenshot X11 classique donne un écran
  noir. Forcer `GDK_BACKEND=x11` (en plus de `DISPLAY=:0`) dans l'environnement
  du process pour que la fenêtre soit une vraie fenêtre XWayland pilotable —
  `scripts/e2e-run.mjs` le fait déjà pour Vite et `tauri-driver`.

- **Le binaire Tauri charge `build.devUrl` (`http://localhost:1420`) par
  défaut, y compris en `--release`** — seul le feature flag Cargo
  `tauri/custom-protocol` (activé automatiquement par la CLI `tauri build`,
  jamais par un `cargo build` direct) fait basculer sur `frontendDist`
  embarqué. Sans serveur Vite qui tourne en parallèle et sans ce feature, la
  fenêtre affiche juste « Could not connect to localhost » — ce n'est pas un
  bug de l'app. Détail complet et commande exacte dans la section « Tests E2E
  réels » ci-dessus (piège `custom-protocol`).

- **Écritures des fichiers de config = atomiques, obligatoirement.** Tout ce qui
  écrit un fichier de config/secret passe par `secure_file::write_private`, qui
  écrit un temp 0600 puis `rename` (atomique) — jamais une troncature-écriture
  sur place. Deux raisons : (a) les lectures sont désormais *fail-closed* (un
  `known_hosts.json`/`workspace.json` tronqué par un crash en cours d'écriture
  serait refusé, verrouillant l'utilisateur hors de tous ses hôtes) ; (b) les
  tests d'intégration `core/tests/` partagent le **vrai** `known_hosts.json`
  (`ProjectDirs`, aucune isolation de chemin) et tournent en parallèle — une
  écriture non atomique laisse un autre thread lire un fichier à moitié écrit.
  C'est exactement ce qui a fait échouer `ssh_integration::bastion_chain…` en CI
  (« Unknown server key ») une fois le fail-closed introduit. Ne pas revenir à un
  `std::fs::write` direct pour ces fichiers.

- **`$env:CARGO_TARGET_DIR` (comme le reste de `$env:PATH`/`Set-Location`) ne
  survit pas d'un appel PowerShell à l'autre.** Chaque invocation de l'outil
  PowerShell démarre un état frais — seul le répertoire de travail persiste,
  pas les variables d'environnement ni le `$env:PATH` étendu. Oublier de
  reposer `$env:CARGO_TARGET_DIR = "$env:USERPROFILE\..."` dans un appel
  `cargo build` séparé du précédent fait retomber silencieusement sur le
  target dir par défaut (relatif au `Cargo.toml`, donc sur le chemin UNC) et
  reproduit exactement le piège du lock file incrémental documenté plus haut
  (« Lock file de compilation incrémentale impossible à créer sur un chemin
  UNC ») — rencontré deux fois de suite pendant la même session de debug RDP
  avant d'y penser systématiquement à chaque nouvel appel `cargo`.

- **Un process `rdp-sidecar.exe`/`gui-termius.exe` resté ouvert après un test
  précédent verrouille le binaire que le build suivant essaie d'écrire.**
  Symptôme trompeur : `cargo build` compile sans erreur (« Finished... »),
  mais l'étape de copie automatique du sidecar dans le `build.rs` de
  `tauri-build` panique avec `Os { code: 5, kind: PermissionDenied, message:
  "Accès refusé." }` — rien à voir avec le code qui vient d'être modifié.
  `Get-Process rdp-sidecar,gui-termius -ErrorAction SilentlyContinue | Stop-Process
  -Force` avant de relancer le build ; particulièrement facile à oublier
  quand on itère vite sur plusieurs correctifs RDP d'affilée (voir plus haut).

## Roadmap / prochaines features (décidées avec l'utilisateur)

Features majeures retenues, dans l'ordre de priorité restant :

1. **Coffre chiffré (mot de passe maître)** — ✅ **fait** (secrets, passphrases
   et clés privées chiffrés au repos ; voir la section « Stockage des secrets »).
2. **Tunnel SOCKS dynamique (`-D`)** — ✅ **fait**. `PortForwardKind::Dynamic`
   dans `core/src/model.rs` ; serveur SOCKS5 local minimal (handshake sans
   auth, `CONNECT` uniquement, IPv4/domaine/IPv6 — les noms de domaine sont
   passés tels quels à `channel_open_direct_tcpip`, résolus côté serveur SSH)
   dans `core/src/port_forward.rs` (`start_dynamic` / `handle_socks_connection`).
   `commands/forward.rs` inchangé (générique sur `PortForwardKind`). UI dans
   `TunnelsPanel.tsx` : option « SOCKS dynamique (-D) », champs « destination »
   masqués pour ce type. Couvert par un test d'intégration réel (vrai `sshd`)
   dans `core/tests/sftp_and_forward_integration.rs`
   (`dynamic_port_forward_reaches_a_local_service`).
3. **Génération + déploiement de clés SSH** — ✅ **fait**. `core/src/keygen.rs` :
   `generate()` (ed25519 par défaut, RSA 4096 en option, passphrase optionnelle)
   via `russh::keys`/`ssh_key::PrivateKey::random` — la source d'aléa OS passe
   par `getrandom::SysRng` (nouvelle dépendance directe de `core/`), pas par
   `ssh_key::rand_core::OsRng` : ce type a disparu de la version de `rand_core`
   (0.10) que `ssh-key` résout réellement dans ce projet, malgré ce que
   suggèrent encore les doctests de la crate (RC, features `getrandom` non
   activée par `russh`). `public_key_line()` redérive la clé publique depuis le
   PEM stocké (vault → workspace.json → fichier d'origine, même ordre que
   `ssh::authenticate`) — pas de champ public persisté séparément.
   `deploy_public_key()` = équivalent `ssh-copy-id` (crée `~/.ssh` en 700,
   ajoute la clé publique à `authorized_keys` en 600 via SFTP, dédoublonnage
   par matériel de clé — pas par commentaire). Logique de fusion pure
   (`merge_authorized_keys`) extraite et testée unitairement plutôt qu'en
   intégration réelle : contrairement aux autres tests SFTP qui opèrent dans
   un sous-dossier jetable `gui-termius-test-{uuid}`, cette fonction cible le
   `~/.ssh/authorized_keys` réel et sensible du compte qui fait tourner les
   tests (le harnais `TestSshd` n'isole pas `$HOME`) — même logique que pour
   le coffre chiffré, voir plus haut. Commandes Tauri dans
   `commands/keys.rs` (nouveau fichier, pas `hosts.rs`) : `generate_private_key`,
   `get_public_key`, `deploy_public_key`. UI dans `KeychainPanel.tsx` : bascule
   Importer/Générer dans le formulaire existant, actions par clé « Copier la
   clé publique » (presse-papiers direct, pas de zone de texte) et « Déployer
   sur un hôte » (sélecteur d'hôte inline).
4. **Accès multi-protocole (Docker exec / K8s exec / RDP)** — 🚧 **en cours**.
   `HostKind` (`core/src/model.rs`, enum `Ssh` défaut / `DockerExec` / `K8sExec`
   / `Rdp`) généralise `Host` sans réécrire son schéma : chaque kind repurpose
   un sous-ensemble des champs existants au lieu d'en faire pousser de
   nouveaux (voir le doc-comment de `HostKind` pour le détail exact par
   kind — ex. `address` devient le socket/hôte Docker, ou un contexte
   kubeconfig). **Docker exec est réel** : `core/src/docker.rs` (crate
   `bollard`, connexion directe au démon — socket unix, named pipe Windows,
   ou tcp/http — jamais via SSH, contrairement à l'hypothèse SSH-shaped
   envisagée puis abandonnée en cours de route) expose `list_containers` et
   `open_exec`, ce dernier bridgé sur le même type `ssh::ShellSession` que
   les shells SSH pour être rejoué tel quel par `write_terminal`/
   `resize_terminal`/`close_terminal` (`state::TerminalBackend` généralise
   `TerminalSession` pour porter soit une `Connection` SSH soit rien de plus
   pour Docker). `TerminalTab.tsx` accepte un `dockerContainerId` optionnel
   qui bascule l'appel de connexion, sans dupliquer le composant. **RDP a
   deux modes, tous deux réels.** Le mode « lanceur » historique
   (`core/src/rdp.rs`, commande `connect_rdp`, action principale des hôtes
   RDP) shell-out vers le client du système : sous Windows, `mstsc.exe` sur
   un fichier `.rdp` temporaire, identifiants pré-chargés via `cmdkey`
   (Gestionnaire d'identifiants Windows, cible `TERMSRV/<adresse>`) puis
   supprimés — ainsi que le fichier `.rdp` — une fois `mstsc.exe` fermé
   (tâche `tokio::spawn` qui attend la fin du process ; ne fonctionne que
   parce que le runtime Tauri est long-lived — un test `#[tokio::test]`
   classique tue cette tâche en fond avant qu'elle s'exécute, d'où le test
   réel ci-dessous marqué `#[ignore]` plutôt que vérifié par un test
   normal) ; sous Linux, `xfreerdp`/`xfreerdp3` si présent sur le `PATH`
   (non testé en conditions réelles — absent de ce WSL) ; macOS non
   supporté (pas de client scriptable). Le mode « aperçu intégré »
   (`RdpTab.tsx`, action secondaire dans le menu de l'hôte) rend le flux RDP
   directement dans l'appli, avec **forward souris/clavier, presse-papiers
   bidirectionnel (Windows) et redimensionnement dynamique** (toujours pas
   d'audio/lecteurs partagés/rendu de curseur), via un processus séparé
   (`rdp-sidecar`, IronRDP) communiquant par un protocole maison sur
   stdin/stdout (`rdp-ipc`) — voir la section « RDP intégré (rendu réel) :
   architecture sidecar » plus haut pour le pourquoi (conflit de version
   `ecdsa` insoluble entre `russh` et `ironrdp-connector`) et le comment.
   **K8s exec reste une maquette UI sans backend** (sélecteur de type,
   formulaire contexte+namespace, picker avec bandeau « aperçu »). Pas de
   daemon Docker joignable dans ce WSL pour tester l'exec réellement
   (intégration WSL non activée dans Docker Desktop) : couvert par tests
   unitaires (classification d'hôte Docker) + relecture attentive de l'API
   `bollard` vendue. Le lancement RDP (mode lanceur) a été vérifié pour de
   vrai en natif Windows (`cargo test -p termius-core rdp:: -- --ignored`) :
   vraie fenêtre `mstsc.exe` lancée contre une adresse TEST-NET-3 (RFC 5737,
   non routable), identifiant confirmé présent dans `cmdkey /list` pendant
   que `mstsc` tournait. Le mode « aperçu intégré », lui, **a été validé
   contre un vrai serveur RDP par l'utilisateur le 2026-07-10**, en
   plusieurs passes : connexion + affichage réels (un premier essai avait
   crashé immédiatement sur `CryptoProvider` rustls non installé, voir plus
   haut, corrigé puis reconfirmé), puis forward souris/clavier confirmé
   fonctionnel, puis presse-papiers confirmé fonctionnel dans les deux sens
   (nativement sous Windows, Vite resté côté WSL via le port-forwarding
   WSL2). Le redimensionnement dynamique, ajouté juste après, a été retesté
   contre ce même serveur le 2026-07-11 : un second bug y a été trouvé
   (écran noir permanent après un resize, historique de décompression MPPC
   désynchronisé par la reconstruction du fast-path processor à chaque
   réactivation) et corrigé en ne reconstruisant plus ce processor du tout —
   voir la section dédiée plus haut pour le détail complet des deux bugs et
   du fix final ; limites restantes détaillées dans la même section.

**Avant de proposer une feature « évidente » de client SSH, vérifier
`src/components/` : elle existe probablement déjà.** L'app est déjà très complète
— palette de commandes (`CommandPalette`), broadcast/cluster (`BroadcastBar`),
split panes (`SplitPane`), recherche terminal (`TerminalSearchBar`), reconnexion
auto (pref `autoReconnect`), 8 thèmes de terminal, restauration d'onglets. Les
vraies lacunes restantes sont côté protocole/ops : auth keyboard-interactive
(MFA/OTP, absente de `AuthMethod`) et K8s exec (maquette UI sans backend) —
l'aperçu RDP intégré (voir le point 4 ci-dessus) a désormais l'affichage, le
forward souris/clavier, le presse-papiers et le redimensionnement dynamique,
il ne lui manque plus que le rendu du curseur et le transfert de fichiers.

## Habitudes de collaboration sur ce projet

- Ne jamais committer sans demande explicite — même après plusieurs tours de
  changements validés. Cette instruction a été redonnée plusieurs fois dans les
  sessions passées ; par défaut, laisser les changements non committés et le
  dire clairement en fin de réponse.
- L'utilisateur écrit et pense en français ; les réponses, les libellés UI, les
  messages de commit et la documentation du projet suivent cette convention.
- Avant une fonctionnalité un peu ambiguë (ex. « menu contextuel » vs « action
  instantanée » pour un clic droit), une question courte à choix (2-3 options)
  vaut mieux qu'une supposition — surtout quand les deux implémentations sont
  d'ampleur comparable mais donnent une UX très différente.
- Sur les demandes larges (« qu'est-ce que tu améliorerais ? »), il vaut mieux
  proposer une liste concrète et ancrée dans le code réel (pas des idées
  génériques de client SSH) puis laisser l'utilisateur choisir ce qui vaut le
  coup d'être implémenté, plutôt que de tout construire d'un coup.
