# Publier une release

L'app se met à jour toute seule (bouton "Vérifier les mises à jour" dans
Paramètres > Général, plus une vérification silencieuse au lancement) via
GitHub Releases. Ce document décrit comment déclencher une nouvelle version.

## Vue d'ensemble

1. `.github/workflows/release.yml` construit l'installeur Windows et publie
   une **release brouillon** à chaque tag `v*` poussé sur GitHub.
2. Tant que la release reste en brouillon, elle est invisible pour
   l'updater (et pour la page "latest release" de GitHub) — c'est un choix
   délibéré : ça laisse le temps de vérifier que le build est sain avant
   qu'il ne soit proposé à qui que ce soit.
3. Publier le brouillon manuellement est ce qui rend la mise à jour réelle.

## Étapes

```bash
# 1. Bumper la version (package.json + Cargo.toml en une commande ;
#    tauri.conf.json n'a volontairement pas de champ "version" — Tauri lit
#    celle de src-tauri/Cargo.toml, qui hérite elle-même du workspace).
npm run bump-version -- 1.4.1

# 2. Commit + push
git add -A && git commit -m "Bump version to 1.4.1"
git push

# 3. Tag + push du tag : déclenche le build
git tag v1.4.1
git push origin v1.4.1
```

4. Suivre l'avancement dans l'onglet **Actions** du dépôt (~10-15 min pour
   un build Windows natif).
5. Une fois le run vert, ouvrir **Releases**, ouvrir le brouillon `v1.4.1`,
   vérifier les notes puis cliquer **Publish release**.
6. Les installations existantes verront la mise à jour au prochain
   lancement, ou immédiatement via "Vérifier les mises à jour".

## En cas d'erreur de build

- Voir le log de l'étape en échec dans Actions (nécessite d'être connecté :
  l'API publique ne permet pas de télécharger les logs sans jeton).
- Si le tag pointe déjà vers un commit dont le build a échoué, corriger le
  problème sur `master`, committer, puis redéplacer le tag :
  ```bash
  git tag -f v1.4.1
  git push --force origin v1.4.1
  ```

## Clé de signature de l'updater

Les artefacts sont signés (obligatoire pour que l'updater les accepte) avec
une clé générée via `tauri signer generate`. La clé privée vit dans le
secret GitHub Actions `TAURI_SIGNING_PRIVATE_KEY` (+ éventuellement
`TAURI_SIGNING_PRIVATE_KEY_PASSWORD` si elle est protégée par un mot de
passe) — jamais dans le dépôt. La clé publique correspondante est dans
`src-tauri/tauri.conf.json` (`plugins.updater.pubkey`).

**Si la clé privée est perdue**, il devient impossible de signer de
nouvelles releases que les installations existantes accepteront : il
faudrait redistribuer l'app avec une nouvelle clé publique. Gardez une
copie de la clé privée en lieu sûr (gestionnaire de mots de passe, coffre).
