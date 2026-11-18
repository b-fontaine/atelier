# ADR S4 — Un fichier `.feature` partagé entre cucumber-rs et `bdd_widget_test`

Statut : essai terminé le 2026-11-18. **Critère de réussite atteint** : les deux exécutants passent sur le même fichier source, et `sync_features --check` détecte un écart. Le dialecte commun est cependant plus étroit que celui du §8.3, et deux défauts de l'outillage obligent à des garde-fous (voir « Corrections au §8 »).

Prototype jetable : `spikes/s4-gherkin/`.

## Versions réellement utilisées

| Composant | Version |
|---|---|
| Flutter / Dart | 3.50.0 / 3.14.0 |
| Cargo | 1.100.0 |
| cucumber (cucumber-rs) | 0.23.0 (épinglé `=0.23.0`), `cucumber-expressions` 0.5.0, `gherkin` 0.16.0 |
| bdd_widget_test | 3.1.0 |
| build_runner / build | 2.17.1 / 5.0.0 |
| cucumber_gherkin (Dart, parseur de bdd_widget_test) | 43.0.0 |
| material_ui (import par défaut des tests générés) | 1.11.1 |

## Structure du prototype

```
spikes/s4-gherkin/
  features/notes.feature      source unique (# language: fr)
  core/                       crate Rust, cible de test `bdd` (harness = false)
  app/                        paquet Flutter
    build.yaml                literalParameters: true
    dart_test.yaml            déclare les étiquettes core / gui
    tool/sync_features.dart   copie + --check
    test/features/            COPIE de la source (+ notes_test.dart généré + step/)
```

## Commandes exactes

```bash
# Cœur (depuis spikes/s4-gherkin/core). Étiquette par défaut : core
cargo test --test bdd
CUCUMBER_TAG=gui cargo test --test bdd       # filtre par étiquette
FEATURES_DIR=/chemin/autre cargo test --test bdd

# Flutter (depuis spikes/s4-gherkin/app)
dart run tool/sync_features.dart             # copie les .feature contenant @gui
dart run tool/sync_features.dart --check     # code 1 en cas d'écart (modifié, manquant, orphelin)
dart run build_runner build                  # génère notes_test.dart et les stubs step/
flutter test --tags gui                      # exécute uniquement les scénarios @gui
```

Côté Rust, le lanceur est `NotesWorld::cucumber().fail_on_skipped().filter_run_and_exit(dir, |f, r, s| …)` (le filtre examine les étiquettes du scénario, de sa règle et de la feature).

## Résultats

- cucumber-rs `@core` : 6 scénarios, 23 étapes, tous passent (le Plan du scénario compte pour 2 scénarios).
- cucumber-rs `@gui` : 5 scénarios, 18 étapes, tous passent.
- `flutter test --tags gui` : 5 tests, tous passent.
- `fail_on_skipped` : une étape sans définition fait échouer la suite (code 101) — **à condition d'utiliser `filter_run_and_exit` / `run_and_exit`** (voir ci-dessous).
- `sync_features --check` : code 0 sur copie propre ; code 1 après modification de la source (`différent : notes.feature`) ; code 1 pour un `.feature` orphelin dans la copie ; retour à 0 après restauration.

## Les points du §8.3 non vérifiés, tranchés

### 1. `literalParameters` sur un flottant et un mot isolé

- **Flottant** : `1.5` bare devient un paramètre, typé `num` côté Dart (`jappliqueUnCoefficientDe(tester, 1.5)`). Côté Rust `{float}` correspond. **Marche.** Un entier est aussi typé `num` (pas `int`).
- **Mot isolé** (`le statut est ouvert`) : **pas** un paramètre. Le mot reste dans le nom de l'étape (`leStatutEstOuvert`). Il n'existe pas de forme littérale pour un mot ; le Rust peut utiliser `{word}` ou le texte en dur, mais pour rester symétrique, **écrire le mot entre guillemets** (`"ouvert"`) ou l'inclure dans la phrase fixe. Règle à retenir : seuls `"texte"` et les nombres sont des paramètres.
- Chaîne vide `""` : acceptée des deux côtés (`''` en Dart, `{string}` vide en Rust).

### 2. Scénario sans `@gui` côté Dart et exclusion propre

- Il est **généré quand même** dans `notes_test.dart`, avec `tags: ['core']`, et ses étapes sont créées en stubs `throw UnimplementedError()`.
- Sans filtre, `flutter test` **échoue** sur ce scénario.
- `flutter test --exclude-tags core` est **inutilisable** : les scénarios `@core @gui` portent aussi `core`, tout est exclu (« No tests match »). `--tags "core && !gui"` sélectionne à l'inverse le scénario cœur seul.
- Exclusion correcte : **inclusion** par `flutter test --tags gui`. L'étiquette doit être déclarée dans `dart_test.yaml` (`tags: { gui: , core: }`), sinon Flutter avertit.
- Conséquence : les stubs du scénario `@core`-seul doivent exister et compiler. Deux options : les laisser en `UnimplementedError` (viole la règle « aucune étape vide » du §8.4 ; à justifier), ou **séparer les scénarios cœur seul dans des fichiers sans `@gui`**, que `sync_features` ne copie pas. L'outil du prototype copie un fichier dès qu'il contient `@gui` quelque part ; la granularité est donc le fichier. **Recommandé : granularité fichier.**

### 3. `Règle` (Rule)

Supportée des deux côtés avec `# language: fr` (`Règle:`). Dart : un `group('Une note vide est refusée', …)` ; Rust : « 1 rule » dans le rapport, étiquettes du scénario honorées. Le `Contexte` de la feature s'applique aux scénarios de la règle des deux côtés. Non testé : un `Contexte` propre à la règle.

### 4. `build_runner` et les liens symboliques

Essai dans un paquet jetable avec `test/features/shared -> …/features` :
- `build_runner` **suit** le lien : il génère `notes_test.dart` et `step/` **à travers le lien, donc dans le dossier source `features/`** (pollution du dossier partagé ; nettoyé après l'essai).
- `flutter test` **ne découvre pas** les tests sous un répertoire lié : « Test directory "test" does not appear to contain any test files ».
- Conclusion : **le lien symbolique ne remplace pas la copie.** Elle reste la décision du §8.1.

## Dialecte Gherkin commun effectivement supporté

**Marche des deux côtés** : `# language: fr` ; `Fonctionnalité`, `Contexte`, `Scénario`, `Plan du scénario` + `Exemples`, `Règle` ; étiquettes sur scénario/plan ; paramètres `"texte"` et nombres (entiers et flottants) avec `literalParameters: true` ; table de données sous une étape ; chaîne vide.

**Ne marche pas / pièges** :

1. **Cellules de table de données** : `bdd_widget_test` insère le texte de chaque cellule **comme code Dart**. `| Courses |` génère `[Courses]` et ne compile pas. Il faut écrire `| "Courses" | 1 |` (chaîne Dart valide) et, côté Rust, retirer les guillemets (`trim_matches('"')`). Les en-têtes aussi (`"titre"`). Les nombres restent bruts. Les tables `Exemples` d'un Plan du scénario sont en revanche du texte brut (pas de guillemets).
2. **`<paramètre>` de Plan du scénario hors guillemets** : le stub généré le type en `dynamic` (à retyper en `num` à la main). Entre guillemets : `String`.
3. **Pluriel / forme optionnelle** : `note(s)` de cucumber-expressions n'existe pas côté Dart ; « 1 note » et « 2 notes » donnent deux étapes Dart distinctes (`laListeContientNote`, `laListeContientNotes`). Écrire une phrase sans variante, ou accepter la duplication.
4. **Défaut du générateur** : la première génération des stubs peut **corrompre** un fichier d'étape utilisé par plusieurs scénarios/lignes d'exemple (octets résiduels après `}`, non-déterministe : constaté 2 fois sur 3). Conséquence : après toute génération initiale, `dart analyze` et vérification des stubs ; les fichiers d'étapes étant créés une fois puis maintenus à la main, ceci n'affecte que la création.
5. **Les noms d'étapes Dart changent** avec `literalParameters` (déjà documenté par le paquet) ; le doc-comment des stubs contient `<…>` qui déclenche un avertissement `unintended_html_in_doc_comment`.
6. Doc strings, `After`, étiquettes avec espace : non essayés ici, restent interdits comme au §8.3.

## Corrections à apporter au §8 de `ARCHITECTURE-AGENT.md`

1. **§8.4 et §8.3 (`fail_on_skipped`)** : ajouter qu'il faut `run_and_exit` / `filter_run_and_exit`. Avec `.run()` / `.filter_run()` le processus **rend le code 0 même si des scénarios échouent** (constaté). Sans cela, la CI ne voit rien.
2. **§8.2 filtrage Dart** : remplacer « `--tags gui` / `--exclude-tags` » par « **`--tags gui`** ; `--exclude-tags core` est faux dès qu'un scénario porte `@core @gui` ». Ajouter `dart_test.yaml` pour déclarer les étiquettes.
3. **§8.3, ligne « Paramètres »** : lever les `[NV]` — flottant OK (`num`), mot isolé non supporté (le mettre entre guillemets), entiers typés `num`.
4. **§8.3, ligne « Rule, Background, … tables »** : préciser que les **cellules de table sont du code Dart** → `"…"` obligatoire pour les chaînes, et Rust doit retirer les guillemets ; les `Exemples` sont du texte brut.
5. **§8.3, dernière phrase** : remplacer `[NV]` par le fait établi : le scénario non `@gui` est généré (étiquette `core` seule) et ses étapes sont des stubs `UnimplementedError` ; exclusion par `--tags gui` ; recommandation de séparer les scénarios cœur seul dans des fichiers sans `@gui`.
6. **§8.1, liens symboliques** : lever le `[NV]` : `build_runner` suit le lien mais écrit les fichiers générés dans le dossier source, et `flutter test` ne les découvre pas ; la copie est confirmée.
7. **§8.1, `sync_features`** : l'outil Dart de 60 lignes du prototype (`spikes/s4-gherkin/app/tool/sync_features.dart`) convient ; il doit aussi supprimer les orphelins et les signaler en `--check`.
8. **§14, S4** : marquer réussi, avec les réserves 1 et 4 des pièges ci-dessus. Le repli `gherkart` n'est pas nécessaire.

## Limites de l'essai

- Un seul fichier `.feature`, un domaine trivial ; pas de `Contexte` de règle, pas de doc string, pas de `@serial`.
- Tests de widgets sans widget réel (faux en mémoire seulement) ; Linux uniquement.
- Le défaut de corruption des stubs a été constaté mais pas diagnostiqué dans le code du générateur ni signalé en amont.
