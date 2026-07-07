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

Le dépôt est monté depuis WSL (`\\wsl.localhost\Ubuntu-24.04\...`), mais **cargo
n'est pas dans le PATH côté Windows** dans cet environnement — seul `npm`/`node`
Windows natifs le sont. Pour tout ce qui touche Rust (`cargo check`, `cargo
build`, tests), passer par WSL explicitement :

```bash
wsl.exe -e bash -lc "cd ~/gui-termius/src-tauri && cargo check"
```

Le frontend (`npm`, `npx tsc`, `vite build`) peut tourner indifféremment côté
Windows natif ou via ce même mécanisme `wsl.exe -e bash -lc "cd ~/gui-termius && ..."`
— rester cohérent sur un seul des deux pour un même enchaînement de commandes
évite les surprises de cache/`node_modules` dupliqués.

Il n'y a pas de moyen de piloter l'interface graphique réelle dans cet
environnement (pas de `tauri dev` interactif, pas de captures d'écran). La
vérification d'un changement se limite donc à : `cargo check` (+ `--tests` si
la modif touche des structs partagées), `npx tsc --noEmit`, `npm run build`.
Le dire explicitement à l'utilisateur plutôt que de laisser croire à un test
fonctionnel réel — voir le skill `verify` pour la nuance entre les deux.

## Tester réellement l'app, pas juste la compiler

`cargo check` / `tsc --noEmit` / `npm run build` prouvent que le code compile,
pas qu'une fonctionnalité marche. Si une session future dispose d'un accès
écran (contrairement à celle-ci, voir plus haut), voici comment vérifier pour
de vrai plutôt que de s'arrêter à la compilation :

- **Lancer l'app réellement** : `npm run tauri dev` ouvre la fenêtre native
  (WebView2). C'est la seule façon de voir fonctionner quoi que ce soit qui
  passe par `invoke(...)` — c'est-à-dire à peu près toute fonctionnalité
  utile (connexion SSH, SFTP, trousseau, terminaux...). Utiliser le skill
  `run` s'il est disponible : il sait déjà lancer un projet Tauri et prendre
  le relais pour piloter/observer la fenêtre.
- **Puppeteer/Playwright ne fonctionnent pas ici comme sur un site web.**
  Pointer un de ces outils sur `http://localhost:1420` (le serveur Vite,
  lancé par `npm run dev` seul) charge bien le frontend, mais tout ce qui
  appelle `invoke()` échoue silencieusement ou plante : l'objet `__TAURI__`
  n'existe que dans la vraie webview Tauri, pas dans un Chrome/Chromium
  classique. Concrètement, ça permet de vérifier une mise en page ou un état
  purement visuel (thèmes, disposition), mais pas une connexion SSH, du SFTP,
  un terminal, le trousseau, etc. Ne pas perdre de temps à déboguer des
  erreurs `invoke is not a function` dans ce contexte : c'est normal, changer
  d'approche plutôt que de contourner.
- **Pour du vrai bout-en-bout automatisé** sur une fenêtre Tauri, l'outil
  officiel est [`tauri-driver`](https://tauri.app/develop/tests/webdriver/)
  (protocole WebDriver, comme Selenium) — pas Puppeteer. Il n'est pas
  installé dans ce dépôt aujourd'hui ; si l'utilisateur veut de vrais tests
  E2E automatisés, ce serait le point de départ (nouvelle dépendance à
  ajouter, pas quelque chose à improviser en une réponse).
- **Captures d'écran** : si l'environnement d'exécution permet de piloter le
  bureau (outil de capture d'écran, contrôle souris/clavier au niveau OS),
  une capture après avoir lancé `npm run tauri dev` est plus fiable qu'une
  déduction à partir des logs — utile en particulier pour tout ce qui touche
  au rendu (thèmes, drag-and-drop, disposition du split, popovers).
- Utiliser le skill `verify` pour la méthodologie générale (driver le
  changement de bout en bout plutôt que de se fier aux seuls tests/typecheck)
  et le distinguo à faire à l'utilisateur entre « ça compile » et « ça
  marche ».
- Si aucun accès écran n'est disponible (le cas dans cette session), le dire
  explicitement plutôt que de laisser croire à un test fonctionnel réel.

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
