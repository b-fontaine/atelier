# ADR S6 — Jetons DTCG vers palette ratatui

- Statut : essai **réussi** (voie Style Dictionary, sans repli)
- Date : 2026-11-18
- Code : `spikes/s6-jetons/` (prototype jetable)
- Référence : `ARCHITECTURE-AGENT.md` §9 et §14 (S6)

## Résultat

Critère du §14 atteint : la palette ratatui est générée depuis un fichier DTCG par un format Style Dictionary personnalisé, utilisée par un atome `Badge`, et 13 tests passent. Les sorties CSS et Dart sortent de la même source. Le repli `build.rs` maison n'a **pas** été nécessaire.

## Versions réellement utilisées

| Composant | Version |
|---|---|
| Node / npm | 26.13.1 / 12.2.2 |
| Style Dictionary | 5.8.0 (exact ; dépend de `colorjs.io` 0.5.2 et `tinycolor2` 1.6.0) |
| rustc / cargo | 1.100.0 / 1.100.0 (édition 2024) |
| ratatui | 0.30.2 (crossterm 0.29.0, ratatui-core 0.1.2) |

Rien d'autre n'est installé : Dart/Flutter n'ont **pas** été testés (le fichier Dart est généré mais non compilé).

## Commandes exactes

```bash
cd spikes/s6-jetons
npm ci
npm run build     # node build.mjs
npm run check     # node check.mjs : régénère dans un dossier temporaire, compare octet à octet, exit 1 si dérive
cd atelier-tui && cargo test && cargo clippy --all-targets   # clippy sans avertissement
```

Le contrôle de dérive a été vérifié : une ligne ajoutée à `generated/tokens.css` donne `DÉRIVE : generated/tokens.css diffère…` et le code de sortie 1 ; fichier restauré, code 0. Il ne modifie pas l'arbre de travail (contrairement à « régénérer puis `git diff` »), et détecte aussi un fichier versionné manquant.

## Ce qui a été réalisé

- `tokens/atelier.tokens.json` : primitives `color.*` en objets DTCG 2025.10 (`colorSpace: "srgb"`, `components`, `hex`), sémantiques `surface`, `texte`, `accent`, `danger` par alias (`{color.blue.700}`), jeton `emphasis` (`fontWeight` 700 + `$extensions["atelier.tui"].modifiers = ["bold"]`). Deux modes (`mode.light`, `mode.dark`).
- `formats.mjs` : trois formats enregistrés via `hooks.formats` (`atelier/rust`, `atelier/css`, `atelier/dart`) ; transforms `name/kebab` + `color/hex` (intégrés).
- Rust généré : `pub struct Palette { surface, texte, accent, danger: Color, emphasis: Modifier }`, constantes `LIGHT_SURFACE: Color = Color::Rgb(..)`… et `pub const LIGHT: Palette`, `DARK: Palette` (constants, utilisables en `const`).
- `atelier-tui` : `ColorSupport::detect_with(env)` (injectable, donc tests sans toucher à l'environnement du processus ; `NO_COLOR` non vide > `COLORTERM` truecolor/24bit > `TERM` *256color* > 16), `degrade()` (index xterm-256 le plus proche : cube 6×6×6 ou rampe de gris ; 16 couleurs par distance euclidienne aux valeurs xterm), atome `Badge`.
- Tests : assertions directes sur `Buffer` d'un `TestBackend` (`buf[(x, y)].bg == Color::Rgb(29, 78, 216)`, valeur de `#1d4ed8` du JSON), modes clair/sombre, dégradation, `NO_COLOR` (couleurs `Reset` mais `REVERSED | BOLD` conservés pour que le badge reste lisible), `COLORTERM`.

## Voie retenue et pourquoi

**Style Dictionary 5.8.0 + formats personnalisés.** Les parties dont dépend la chaîne fonctionnent : objets couleur DTCG 2025.10, alias `{a.b}`, résolution avant le format, `$extensions` transmis au format, plusieurs formats sur une même source. Un format se résume à une fonction de ~25 lignes ; pas besoin de `build.rs`. Le générateur maison reste un repli viable, mais n'apporte rien tant que ces points tiennent.

## Limites rencontrées du support DTCG (sondées sur 5.8.0)

| Cas | Comportement observé |
|---|---|
| Couleur `srgb` en objet, alias `{…}` | OK (alias résolu, `original.$value` garde l'alias) |
| `oklch`, `display-p3` | Convertis en hex sRGB par `color/hex` (p3 `[1,0,0]` → `#ff0b0c`, écrêtage de gamut) |
| `alpha` | Devient un hex à 8 chiffres ; mon format **lève une erreur** (TUI sans transparence) |
| `components` contenant `none` | Silencieusement converti (`#000000`) |
| `$extends` (héritage de groupe) | **Non résolu** : le groupe dérivé ne reçoit pas les jetons du parent |
| `$ref` (pointeur JSON) | **Non résolu** : la valeur reste un objet `{"$ref": …}` |
| Dimension objet `{value, unit}` | Passe telle quelle, sans transform : rien de généré pour l'espacement |
| **Modes** | Pas de notion de mode dans 2025.10 (c'est le rôle du format Resolver, séparé). Convention retenue : groupes `mode.<nom>` dans le même fichier |

Autres constats :
- Sans transform `name/*`, les noms de feuille collisionnent entre modes (`surface` clair/sombre) et `warnings: 'error'` fait échouer : il faut `name/kebab`.
- `npm audit` signale un avis modéré transitif (`glob`) ; la « correction » proposée rétrograde vers Style Dictionary 3.9.2 : à ignorer, sans incidence (outil de build).
- Non vérifié : composition de plusieurs fichiers `.tokens.json` avec le même mode, jetons `shadow`/`typography`/`border` composites, cycle d'alias.

## Corrections à apporter au §9

1. §9.1, schéma : préciser que le fichier source contient les modes sous une convention (`mode.<nom>.*`), car DTCG 2025.10 n'a pas de modes natifs ; l'étude ne le dit pas.
2. §9.1, « Style Dictionary 5.8.0 … incrémental » : remplacer par le constat vérifié — couleur objet, alias et `$extensions` OK ; **`$extends` et `$ref` non résolus**. Ne pas les utiliser dans les fichiers de jetons.
3. §9.1, « aucun format Rust intégré » : confirmé ; ajouter que le format personnalisé tient en une fonction (~25 lignes) via `hooks.formats`, `[V]`.
4. §9.1, décision : ajouter la contrainte `name/kebab` (ou équivalent) dans la plateforme, et `log.warnings: 'error'`.
5. §9.1, contrôle CI : recommander `check.mjs` (régénération dans un dossier temporaire + comparaison) plutôt que « régénérer puis différer » ; il ne salit pas l'arbre.
6. §9.2 : ajouter la règle `NO_COLOR` vérifiée (valeur **non vide**, no-color.org) et le comportement retenu : conserver les modificateurs, ajouter `REVERSED` pour garder les atomes colorés lisibles. Précédence testée : `NO_COLOR` > `COLORTERM` > `TERM`.
7. §9.2 : la détection `COLORTERM` / `TERM` est désormais `[V]` (testée) au lieu de `[NV]` ; la table 16 couleurs dépend de la palette du terminal de l'utilisateur, donc approximative par nature.
8. §9.2 : `Modifier` est `const`, donc `Palette` peut être `pub const` sans `LazyLock`.
9. §9.1 : la sortie Dart n'a été produite que sous forme de constantes `Color` ; la génération du `ThemeExtension` (cible du §9.1) reste à faire et à compiler dans l'essai S1/design system.
10. §17 : ajouter Node 26.13.1 observé ; ratatui 0.30.2 compile et teste avec rustc 1.100.0.
