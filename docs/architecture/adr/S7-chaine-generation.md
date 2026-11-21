# ADR S7 — Chaîne de génération de code figée sur `build` 4

- **Date** : 2026-11-21
- **Statut** : essai terminé — **succès** sur Linux
- **Référence** : `ARCHITECTURE-AGENT.md` §20.2 (option A retenue par l'utilisateur), §8, §11
- **Prototype** : `spikes/s7-codegen/` (jetable ; application et `.feature` copiés de `spikes/s4-gherkin/`)

## Question

L'utilisateur a choisi de garder `injectable`, alors que `injectable_generator` plafonne à `build` 4 et que `bdd_widget_test` 3.1.0 exige `build` 5. Avec `bdd_widget_test` 3.0.5 et des versions figées :

1. la génération de code s'exécute-t-elle réellement, pour tous les générateurs de la pile ?
2. le `.feature` de l'essai S4, validé en 3.1.0, passe-t-il encore en 3.0.5 ?

## Versions réellement utilisées

| Composant | Version | Épinglé dans `pubspec.yaml` |
|---|---|---|
| Flutter / Dart | 3.50.1 / 3.14.0 | — |
| `analyzer` | 14.1.0 | non (résolu) |
| `build` / `build_runner` | 4.0.11 / 2.16.1 | `build_runner` oui |
| `source_gen` | 4.3.1 | non |
| `injectable` / `injectable_generator` | 3.0.0 / 3.1.3 (`lean_builder` 1.2.2 en dépendance) | oui |
| `bdd_widget_test` | 3.0.5 | oui |
| `freezed` / `freezed_annotation` | 4.0.3 / 3.1.0 | `freezed` oui |
| `json_serializable` / `json_annotation` | 6.15.2 / 4.12.0 | `json_serializable` oui |
| `retrofit` / `retrofit_generator` / `dio` | 4.10.0 / 10.3.3 / 5.11.1 | non |
| `get_it` / `flutter_bloc` | 9.3.0 / 9.1.1 | oui |
| `flutter_rust_bridge` (paquet Dart) | 2.13.0 | oui |
| `material_ui` | 1.11.1 | non |
| `very_good_analysis` | 11.0.0 | oui |

`pubspec.lock` est versionné dans le prototype : il fait foi pour l'ensemble exact.

## Ce qui a été construit

Dans `spikes/s7-codegen/app/lib/src/` : une passerelle enregistrée par `@LazySingleton` et un `@InjectableInit` ; une classe `@freezed` avec `fromJson` ; une classe `@JsonSerializable` ; un client `@RestApi` de retrofit. Les tests et étapes Gherkin sont ceux de S4, inchangés.

## Commandes exactes

```bash
cd spikes/s7-codegen/app
flutter pub get
dart run build_runner build          # sans --delete-conflicting-outputs (voir constats)
dart analyze lib
flutter test --tags gui
flutter test test/di
dart run tool/sync_features.dart --check
```

## Résultats

| Vérification | Résultat |
|---|---|
| Résolution de la pile complète, FRB compris | OK |
| Génération `injectable_generator` (`injection.config.dart`) | OK |
| Génération `freezed` + `json_serializable` (`note.freezed.dart`, `note.g.dart`, `reglage.g.dart`) | OK |
| Génération `retrofit_generator` (`notes_api.g.dart`) | OK |
| Génération `bdd_widget_test` 3.0.5 (`notes_test.dart`) | OK, **fichier identique octet pour octet** à celui produit par la 3.1.0 dans S4 |
| `dart analyze lib` avec les règles par défaut, code généré inclus | aucun problème |
| Scénarios `@gui` de S4 (`flutter test --tags gui`) | 5 sur 5 |
| Test d'exécution : injection, égalité `freezed`, aller-retour JSON | 1 sur 1 |
| `sync_features --check` | OK |
| Signatures des étapes générées à neuf en 3.0.5 | mêmes 9 fonctions et mêmes paramètres (`num`) qu'en 3.1.0 |

Durées : première génération 33 s (compilation des générateurs), puis 9 s.

**Réponse : oui aux deux questions.** L'option A est viable, et le dialecte Gherkin établi par S4 (§8.3) vaut pour `bdd_widget_test` 3.0.5.

## Constats

1. **`--delete-conflicting-outputs` n'existe plus** dans `build_runner` 2.16.1 : l'option est ignorée avec un avertissement. Les commandes de l'étude qui la passent sont à corriger.
2. **`build.yaml` doit lister `lib/**`** dans `sources`. La configuration de S4 ne listait que `test/**` : les générateurs n'ont alors traité aucun fichier de `lib/`, sans erreur. Piège silencieux.
3. **Le défaut de corruption des fichiers d'étapes existe aussi en 3.0.5** : sur trois générations à neuf, deux ont produit des fichiers d'étapes en erreur d'analyse. La règle du §8.3 (lancer `dart analyze` après toute création d'étapes) reste nécessaire.
4. `json_serializable` avertit « Could not read the pubspec.yaml file » sur la classe `freezed`, même avec `pubspec.yaml` dans les sources. Sans effet observé sur le code généré.
5. Les générateurs passent par `build_runner` (dont `injectable_generator:injectable_builder`) : `lean_builder` est une dépendance, il n'a pas eu à être lancé séparément.
6. Avec `very_good_analysis` 11.0.0, le code écrit à la main du prototype lève 22 remarques de niveau info (documentation publique, longueur de ligne) : aucune ne vient de la chaîne de génération. Le prototype, jetable, n'a pas été mis au propre.

## Non couvert

- Le générateur de `flutter_rust_bridge` n'a pas été exécuté : seul le paquet Dart 2.13.0 a été résolu avec le reste. Le cas où FRB déclenche `freezed` (enums à champs, §5.4) n'est pas essayé.
- `analyzer` ≥ 14.6 et `build_runner` ≥ 2.17.1, où le rapport de veille situe l'incompatibilité d'`injectable_generator` : la résolution reste en 14.1.0 et 2.16.1, donc ce cas n'est pas rencontré tant que les versions sont figées.
- Tests de widgets réels, `integration_test`, autres plateformes que Linux.
- Compatibilité de `widgetbook` et `alchemist` avec cet ensemble.

## Recommandation

Retenir l'ensemble de versions de cet essai comme base du côté Flutter, épinglé exactement, avec `pubspec.lock` versionné. Ne monter ni `build_runner` ni `bdd_widget_test` tant qu'`injectable_generator` ne supporte pas `build` 5.

## Corrections à apporter à l'étude

- §20.2 : lever le `[NV]` sur l'exécution de la génération ; marquer l'essai S7 réussi ; préciser que le dialecte du §8.3 vaut pour la 3.0.5.
- §11.2 et §11.3 : retirer `--delete-conflicting-outputs`.
- §8.1 : `build.yaml` doit lister `lib/**` et `test/**`.
- §8.3 : le défaut de corruption des étapes concerne aussi la 3.0.5.
- §17 : `bdd_widget_test` 3.0.5 et `build_runner` 2.16.1 deviennent les versions de référence.
