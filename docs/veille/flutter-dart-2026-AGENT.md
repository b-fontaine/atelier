# Flutter et Dart en 2026 — référence pour agent

> **Destinataire** : un agent de code travaillant sur un projet Dart/Flutter.
> **Origine** : rapport de veille fourni par l'utilisateur le 2026-11-21, restructuré ici sans ajout de faits, sauf la section 2 (« Vérifications locales »), produite dans ce dépôt.
> **Page compagnon pour lecteur humain** : `flutter-dart-2026.html`, même dossier.
> **Impacts sur l'architecture de ce dépôt** : `docs/architecture/ARCHITECTURE-AGENT.md`, §20.

## 0. Mode d'emploi

### 0.1 Niveaux de fiabilité

| Marqueur | Signification |
|---|---|
| `[P]` | Le rapport s'appuie sur une source primaire (blog ou docs Flutter/Dart, changelog, issue GitHub, pub.dev). |
| `[S]` | Source secondaire seulement (article tiers, newsletter). |
| `[NV]` | Le rapport dit explicitement ne pas avoir pu vérifier. |
| `[E]` | Vérifié dans ce dépôt le 2026-11-21 (section 2). Prime sur le rapport en cas de désaccord. |
| `[A]` | Avis ou recommandation de l'auteur du rapport, pas un fait. |

Les numéros entre crochets, par exemple `[7]`, renvoient à la liste des sources (section 12). Cette étude n'a pas rouvert ces sources : la fiabilité indiquée est celle que le rapport déclare.

### 0.2 Règles d'usage pour l'agent

- Les versions et statuts datent du 2026-11-21. Revérifie-les sur pub.dev et dans les changelogs avant de les écrire dans un `pubspec.yaml`.
- Un `[NV]` ou un `[A]` n'est pas un fait : ne le présente pas comme tel.
- Les chiffres promotionnels de Google (section 9.3) sont à citer comme des déclarations, jamais comme des mesures.

---

## 1. L'essentiel

1. **Flutter 3.50.0 est la version stable depuis le 2026-11-11, avec Dart 3.14.0. Un correctif 3.50.1 a suivi le 2026-11-20.** `[P]` Cible : 3.50.1 directement, jamais 3.50.0 `[A]`.
2. **Le changement structurant de l'année est la sortie de Material et Cupertino du SDK** : `package:flutter/material.dart` et `package:flutter/cupertino.dart` sont `@Deprecated` depuis 3.50, au profit de `material_ui` et `cupertino_ui` `[P]`.
3. **La chaîne d'outils bouge en même temps** : Gradle/AGP 9, SwiftPM, analyzer 14, build 5. Elle fragilise toute pile fondée sur la génération de code `[A]`.
4. **2026 = quatre stables Flutter** (3.41 en février, 3.44 en mai, 3.47 en août, 3.50 en novembre) **et quatre stables Dart** (3.11 à 3.14) `[P]`.
5. **Langage Dart : deux vrais ajouts seulement** : paramètres nommés privés (3.12) et *primary constructors* (stables en 3.13, avec rupture : `final`/`var` interdits sur les paramètres non déclarants). Dart 3.11 et 3.14 n'ajoutent aucune fonctionnalité de langage `[P]`.
6. **Les macros restent abandonnées** : `build_runner` est la seule voie de génération de code `[P]`.
7. **Conflit de contraintes de génération** : `injectable_generator` ne suit pas `build` 5. Détail et correction du rapport en section 2 `[E]`.
8. **GenUI est en alpha** : pas un acquis de production `[A]`.

---

## 2. Vérifications locales du 2026-11-21 `[E]`

Faites dans ce dépôt avec Flutter 3.50.1 et Dart 3.14.0, par `flutter pub get` sur un `pubspec.yaml` de test. **Résolution de dépendances uniquement : aucune génération de code n'a été exécutée.**

| Constat | Détail |
|---|---|
| Flutter 3.50.1 est bien diffusé | Installé localement, révision `ae9b586f34` du 2026-11-18. |
| `bdd_widget_test` ≥ 3.1.0 est **incompatible** avec `injectable_generator` | `bdd_widget_test` ≥ 3.1.0 dépend de `build ^5.0.0` ; toutes les versions de `injectable_generator`, 3.1.3 comprise, plafonnent à `build ^4.0.4`. |
| `bdd_widget_test` **3.1.0 existe** sur pub.dev | Le rapport ne connaissait que 3.0.x. La 3.0.5 y est aussi présente (le rapport ne l'avait pas vérifié). |
| Une combinaison se résout, `freezed` compris | `analyzer` 14.1.0, `build` 4.0.11, `build_runner` 2.16.1, `injectable` 3.0.0, `injectable_generator` 3.1.3, `bdd_widget_test` 3.0.5, `freezed` 4.0.3, `json_serializable` 6.15.2, `get_it` 9.3.0, `flutter_bloc` 9.1.1, `very_good_analysis` 11.0.0. |
| `json_serializable` | 6.15.2 résolu ; le rapport citait 6.14.1 comme dernière version. |

**Correction à apporter au rapport.** Il affirme qu'on ne peut pas avoir `freezed` et `injectable` « à jour » simultanément. La résolution ci-dessus réunit `freezed` 4.0.3 (dernière version citée par le rapport) et `injectable_generator` 3.1.3. Ce qui est impossible, c'est de les réunir avec `build` 5, donc avec `build_runner` 2.17.x et `bdd_widget_test` 3.1.0. Reste non vérifié : que la génération s'exécute correctement avec cet ensemble.

---

## 3. Flutter 3.50

### 3.1 Fiche d'identité

| Élément | Valeur | Fiabilité |
|---|---|---|
| Version stable | 3.50.0, publiée le 2026-11-11 `[1]` | `[P]` |
| Dart associé | 3.14.0 `[2]` | `[P]` |
| Correctif | 3.50.1 : changelog fusionné le 19/11, diffusé le 20/11/2026 `[3][4]` | `[P]` (issue #196743, PR tiers) ; `[E]` |
| Coupe de branche | 2026-10-06 `[5][6]` | `[P]` |
| Bêta en cours | 3.51.0-0.1.pre / Dart 3.15.0-118.1.beta `[6]` | `[S]`, `[NV]` |
| Volume | 188 contributeurs dont 69 nouveaux, plus de 1 200 commits `[7]` | `[P]` |

Le billet officiel (Emma Twersky) est sous-titré « Investing in Flutter's next decade » `[5][7]`. L'essentiel de la version termine des chantiers ouverts en 2024-2025 : interop, découplage du design, SwiftPM, Impeller desktop `[A]`.

### 3.2 Interop native directe

- **FFIgen (Objective-C) et JNIgen (Java/Kotlin) deviennent stables**, à côté du support C historique de FFIgen `[7][8]`.
- Appels natifs directs, **synchrones possibles, y compris depuis des isolates d'arrière-plan**, sans sérialisation par *platform channel* `[7][8]`.
- **Pigeon** gagne un « native interop mode » qui génère des liaisons via FFIgen/JNIgen : **préversion seulement** `[7]`.
- Lecture `[A]` :
  - Le `MethodChannel` cesse progressivement d'être la voie par défaut ; ce n'est pas une obligation, les plugins existants fonctionnent.
  - En Clean Architecture, cela relève de la couche *data/datasource* : une source native synchrone peut supprimer des `Future` artificiels.
  - Les liaisons générées sont plus verbeuses, plus couplées aux SDK natifs et plus difficiles à simuler : placer une interface de source de données devant elles.

### 3.3 Découplage Material/Cupertino

Chronologie officielle :

| Version | Étape |
|---|---|
| 3.44 | Gel (« frozen ») de Material/Cupertino dans le cœur `[9][10]` |
| 3.47 | `material_ui` et `cupertino_ui` en 1.0 sur pub.dev, adoption facultative `[11]` |
| 3.50 | Dépréciation formelle de `package:flutter/material.dart`, `package:flutter/cupertino.dart` et de leurs délégués de `flutter_localizations` `[12]` ; `flutter create` importe `package:material_ui` par défaut `[7]` |

Migration :

- Commande : `dart fix --apply --code=migrate_design_widgets` `[12]`.
- `MaterialUiCompatibilityBridge` permet de migrer l'app alors que des dépendances utilisent encore les anciens imports `[11]`.
- `GlobalMaterialLocalizations.delegates` inclut désormais Cupertino et Widgets `[11]`.

Points durs :

- **Aucune date de suppression** des anciens imports n'est annoncée `[12][13]`.
- **Bogue connu de l'outil de migration** sur `pubspec.yaml` : faire `flutter pub add material_ui` à la main, puis relancer `dart fix` `[11]`.
- Les auteurs de paquets sont priés de **traiter la migration comme une version majeure** `[11]` : montées majeures en cascade dans les dépendances d'interface.
- Retour de terrain : le projet koniz-dev/flutter-starter a vu sa porte de qualité CI casser le lendemain de la sortie, sur **237 diagnostics de membres dépréciés** `[1]`. Une CI en `--fatal-infos` ou `--fatal-warnings` subira le même effet.

Bénéfice : `material_ui`/`cupertino_ui` visent des **publications hebdomadaires** indépendantes du SDK `[14]`, utile pour suivre Material 3 Expressive ou Liquid Glass `[5]`. C'est aussi une surface de rupture plus fréquente : épingler la version `[A]`.

### 3.4 Web

- **Wasm** :
  - Chromium (Chrome/Edge 137+) l'exécutait déjà par défaut `[7][8]`.
  - Safari 26.2+ et Firefox 147+ : le billet revendique « over 2x the frame rate of JavaScript on heavy widget benchmarks » `[15]`. Banc d'essai interne, non reproduit indépendamment.
  - Activation toujours **sur demande**, par `wasmAllowList` dans `flutter_bootstrap.js`. Le passage par défaut est promis « in a future stable release » `[7]`.
- **`--web-content-hash`** : hachage des noms de fichiers pour invalider proprement les caches, et génération d'un `precache_manifest.json`. Les jetons de `flutter_service_worker.js` sont dépréciés `[7]`.
- **Interop JS héritée** :
  - `dart:html`, `dart:js`, `dart:js_util` et `package:js` sont dépréciés, suppression annoncée dans une future version de Dart `[7][8]`.
  - `--no-deprecated-js-interop` fait échouer le build si une dépendance transitive les importe `[7][8]`.
  - dart2wasm ne supporte plus `dart:js_util` depuis Dart 3.11 `[16]`.
- **`WebParagraph`** (mise en page du texte par le navigateur, via l'API expérimentale Enhanced `TextMetrics` de Chrome) : CanvasKit passe d'environ 5,46 Mo à 3,75 Mo. **Expérimental** (issue #172561) `[7]`.
- Rechargement à chaud web : `--web-hot-reload` est **déprécié depuis 3.44** `[9]`.

### 3.5 Desktop

- **Impeller est le moteur de rendu par défaut sur macOS, Windows et Linux depuis 3.47** `[5][17]`.
- En 3.50 : rendu SDF étendu aux lignes, bordures fines, coins arrondis et dégradés ; Windows demande **OpenGL ES 3 par défaut** (repli ES 2) et gère les chemins non ASCII `[7]`.
- **Multi-fenêtres : toujours expérimental.**
  - En 3.44, API disponibles sur le canal *main* seulement, « not yet intended for production use » `[9]`.
  - En 3.50 : `RegularWindow` renommé `Window`, `sizedToContent` devient `shrinkWrap`, `WindowManager` passe au-dessus de `WidgetsApp` `[7]` ; `WindowControllerLinux` pour les barres de titre personnalisées `[7]`.
  - Conclusion `[A]` : pas de multi-fenêtres en production en 2026.
- **Gouvernance** : depuis 3.44, Canonical est responsable du desktop (Linux, Windows, macOS). Annonce de Kate Lovett à Google I/O 2026, puis de @FlutterDev le 2026-05-20 : « Canonical will now serve as the lead maintainer and strategic steward for Flutter Desktop ».
- Régressions desktop de 3.50 :

| Régression | Issue | État |
|---|---|---|
| Contenu décalé sous Linux sur les pilotes sans `glBlitFramebuffer`, Nvidia compris ; reproduit par Fladder avec Impeller comme avec Skia | #195885 `[18][19]` | Corrigé en 3.50.1 |
| Plantage Windows au redimensionnement avec Impeller désactivé | #196587 `[20]` | Non précisé |
| Dégradés GLES qui sautent à la couleur de fin (dépassement fp16 dans l'UberSDF) | #196865, ouverte le 18/11 `[21]` | Ouverte |

### 3.6 iOS et macOS

- **Latence tactile** : événements livrés immédiatement au framework, soit −16 ms à 60 Hz et −8 ms à 120 Hz (travail de Matej Knopp, mesures jusqu'à l'iPhone 6s) `[7]`.
- **iOS 27 / Xcode 27** : préparation automatique des appareils, correction de plantages intermittents de `flutter run`, retour de `flutter screenshot` sur appareil physique `[7]`.
- **SwiftPM** :
  - Par défaut depuis 3.44 `[9][22]`.
  - Le dépôt central CocoaPods passe en lecture seule le **2026-12-02** (plan publié le 2024-11-30 par Orta Therox : « I will switch trunk to not accept new Podspecs permanently »). Un test en lecture seule était prévu du 1er au 7 novembre 2026.
  - En 3.50, désactiver SwiftPM déclenche un avertissement à chaque commande. L'option de désactivation puis le repli CocoaPods seront supprimés `[7]`.
  - En 3.47, 92 des 100 premiers plugins iOS étaient migrés `[11]`.
- **Minimums depuis 3.47** : iOS 15 (contre 13), macOS 12 (contre 10.15). Le cycle de vie `UIScene` est **obligatoire avec le SDK iOS 27** : sans lui, l'app ne se lance pas `[11]`.
- **Mac Intel** : plus testés, avertissement en ligne de commande ; une future version exigera Apple Silicon pour construire iOS, macOS **et Android** `[13]`. `flutter config --enable-macos-arm64-only` est disponible `[7]`.

### 3.7 Android

- **Durcissement des `Intent` (rupture)** `[7]` :
  - En release, les extras `route`, `dart_entrypoint` et `cached_engine_id` venant d'`Intent` externes sont ignorés.
  - Les drapeaux moteur (`--dart-flags`…) sont rejetés.
  - Tester des drapeaux moteur sur un binaire préconstruit exige le mode profile.
- **HCPP (Hybrid Composition++)** : sur demande depuis 3.44 `[9]` ; par défaut en bêta depuis 3.49.0-0.1.pre, pas encore en stable `[7]` ; exige Android 14 (API 34) et Impeller sur Vulkan, sinon repli automatique `[7]`.
- **Kotlin intégré à AGP** : les plugins qui appliquent encore manuellement le Kotlin Gradle Plugin perdront des points pub.dev et **ne se construiront plus sous AGP 10** `[7][12]`. Contrainte à poser : `flutter: ">=3.44.0"` `[7][9]`.
- **Nouveau DSL d'AGP 9** : sur demande en 3.50, seulement si **tous** les plugins sont passés au Kotlin intégré `[7]`.
- **Valeurs par défaut des nouveaux projets** `[7]` : AGP 9.3.1, Gradle 9.5.0, KGP 2.4.20, Java 17 minimum, `compileSdk`/`targetSdk` 36, `minSdk` 24.
- **Régressions de 3.50.0 corrigées en 3.50.1** :

| Régression | Issue |
|---|---|
| Tout type de build personnalisé utilisant `initWith(getByName("release"))`, `isShrinkResources`, `buildConfigField` ou `resValue` casse la configuration Gradle dès qu'un plugin est présent, même en debug. Cas typique des *flavors* d'entreprise. | #196412 / #196696 `[19][23]` |
| Les assets dont le nom commence par un point (**`.env`**) ou placés sous un dossier préfixé `_` sont **exclus en silence** de l'APK/AAB. | #196444 `[19]` |

### 3.8 Framework, accessibilité, tests, outillage

- **Slivers et listes** : `SliverClipRect`/`SliverClipRRect`, `ListView.separated.itemExtentBuilder`, `ScrollPhysics.shouldUpdate` `[7]`.
- **Saisie et texte** : `TextInputType.numberWithOptions(password:)`, `AutofillHints.emailOTPCode`, enum `Hyphens` `[7]`.
- **Images** : `DecorationImage.placeholder`, `ResizeImage.useLogicalSize` `[7]`.
- **Accessibilité et dialogues** : `SemanticsRole.slider`, `MediaQueryData.reduceMotion`, Échap ferme les infobulles, `showDialog(barrierBuilder:)`, `FocusableActionDetector.skipTraversal` `[7]`.
- **Ruptures de 3.50** :
  - Dépréciation de Material/Cupertino `[24]`.
  - Restriction des drapeaux Android en release et vérification stricte des `Intent` `[24]`.
  - Trait d'union visible sur les césures conditionnelles `[24]`.
  - Ordre naturel des `WidgetSpan` en texte de droite à gauche `[24]`.
  - `SafeArea` web avec `viewport-fit=cover` `[24]`.
  - `DropdownButtonFormField.enabled` `[24]`.
  - Suppression de `ignoringSemantics` (`IgnorePointer`, `AbsorbPointer`, `SliverIgnorePointer`) `[7]`.
  - `computeDryBaseline` renvoie `null` au lieu de lever une assertion `[7]`.
- **Effet sur les tests** : césures et texte de droite à gauche **changent le rendu**, donc les goldens `[6]`. Côté `flutter_test` `[25]` : `TestWidgetsApp` déplacé dans `flutter_test` (#189435) ; rappels exécutés dans la zone de test pour `LiveTestWidgetsFlutterBinding` (#191595, à surveiller en `integration_test`) ; `precacheTestImage` (#191435) ; goldens locaux sur le web (#191386).
- **Outillage** : renommage à froid de l'Analysis Server 57 % plus rapide sur les espaces de travail multi-paquets (2,0 s → 0,9 s sur DevTools) `[7]` ; DevTools tourne sur `material_ui` `[7]`.
- **IA** :
  - « Official Flutter agent plugins » : *skills*, configuration MCP, agents spécialisés, règles persistantes, dont `flutter-hot-reload`, qui déclenche un rechargement à chaud après toute édition sous `lib/` `[7]`.
  - *Package skills* : dossier `skills/` dans les paquets pub.dev, installables par `dart run skills@ get`, validés par `package:skills_lint` `[7][26]`.
- **Gouvernance** : processus **RFC public** (`flutter/rfc`), et équivalent côté Dart `[7][8]`.

### 3.9 Correctif 3.50.1 (2026-11-20)

- Gradle et types de build personnalisés (#196412) `[19]`.
- Assets `.env` et dossiers `_` (#196444) `[19]`.
- Contenu décalé sous Linux (#195885) `[19]`.
- `index.html` supprimé après un build web avec puis sans `--web-content-hash` (#196452) `[19]`.
- Plantages de `flutter_tools` : `screenshot --type=skia` avec Impeller (#196119) ; `RPCError` sur dossier temporaire plein ou inaccessible (#196118) ; `flutter test` sur volume externe macOS (#196121) `[19]`.
- Gel du rendu de texte en `--wasm` sous isolation cross-origin `[4]`.

Verdict `[A]` : la 3.50.0 est sortie avec au moins deux régressions bloquantes pour des apps d'entreprise. La politique « .0 = bêta prolongée » reste justifiée.

---

## 4. Chronologie Flutter stable 2026

| Version | Date | Dart | Nouveautés marquantes | Ruptures et dépréciations |
|---|---|---|---|---|
| **3.41** « Year of the Fire Horse » | 2026-02-11 | 3.11 `[27]` | Fenêtres de publication publiques ; assets par plateforme ; vues dimensionnées par le contenu (add-to-app) ; `decodeImageFromPixelsSync`, textures 128 bits ; `Navigator.popUntilWithResult` ; matchers `isSemantics`/`accessibilityAnnouncement` ; DevTools compilé en dart2wasm ; `UIScene` par défaut `[5]` ; fils fusionnés par défaut sous Linux | **Interdiction explicite de passer en AGP 9** (#181383) `[5]` ; `material_color_utilities` 0.11.1 → 0.13.0, qui peut décaler les couleurs générées par graine `[28]` |
| **3.44** (Google I/O) | 2026-05-18 selon les docs `[29]` ; billet du 05-20 | 3.12 | SwiftPM par défaut ; HCPP sur demande ; rechargement à chaud par agent via MCP ; Agent Skills ; GenUI/A2UI mis en avant ; Canonical responsable du desktop `[30]` ; Widget Previews (expérimental, −50 % de mémoire IDE) `[31]` ; DevTools en Wasm `[32]` ; outils sans Rosetta ; `CupertinoMenuAnchor` ; `CarouselView` infini ; Toyota RAV4, LG webOS | Gel de Material/Cupertino ; migration Kotlin intégré/AGP 9 ; ordre de fermeture de `RawMenuAnchor` ; `onReorder` → `onReorderItem` ; builder de `showCupertinoSheet` déprécié `[9]` ; `IconData` final ; `cacheExtent` déprécié `[24]` ; `--web-hot-reload` déprécié ; filtres ABI (`-Pdisable-abi-filtering=true`) |
| **3.47** | 2026-08-12 | 3.13 | `material_ui`/`cupertino_ui` 1.0 (facultatif) ; **Impeller par défaut sur desktop** ; Widget Previews **stable** ; *flavors* Windows/Linux ; chargement différé Wasm (main, sous drapeau) ; popups Linux/Windows ; `windowHandle` ; genui 0.10.0 | **iOS 15 / macOS 12 minimum** ; `UIScene` obligatoire avec Xcode 27 ; avertissements Intel ; inversion de coordonnées dans les shaders GLES `[11]` |
| **3.50** | 2026-11-11 ; 3.50.1 le 11-20 | 3.14 | Section 3 | Section 3 |

Notes :

- **Date de 3.44** : la page « What's new » des docs indique le 18 mai, le billet est daté du 20 mai, le changelog de Dart 3.12 indique le 18 mai `[16][33][34]`. Publication du SDK la plus probable : 18/05, annonce le 20/05.
- **Date de Dart 3.11** : la page « language evolution » indique le 9 février, le changelog et le billet le 11 février `[16][35]`.
- **Cadence** : 3.41 a officialisé **quatre stables par an avec dates de coupe publiques** (6 janvier, 7 avril, 7 juillet, 6 octobre) `[5]`. Elle a été tenue : 3.50 est sortie 36 jours après sa coupe `[6]`. C'est le principal changement de politique de publication de l'année.

---

## 5. Chronologie Dart stable 2026

| Version | Date | Langage | Outils, runtime, bibliothèques | Ruptures |
|---|---|---|---|---|
| **3.11** | 2026-02-11 | **Aucune fonctionnalité nouvelle** (meilleur support IDE des *dot shorthands*) `[36]` | Analysis server à dépendances fines ; cache des plugins analyzer (environ 10 s gagnées au démarrage) ; globs dans les espaces de travail pub (`pkg/*`) ; `dart pub cache gc` ; `publish --dry-run --ignore-warnings` `[16][37]` ; outil MCP `read_package_uris` `[38]` ; AF_UNIX sous Windows | dart2wasm ne supporte plus `dart:js_util` ni `package:js` (erreur de compilation) `[16]` |
| **3.12** | 2026-05-18 (billet : 05-20) | **Paramètres nommés privés** (`this._x` → paramètre `x`) `[35]` ; *primary constructors* **expérimentaux** | Rechargement à chaud par agent (DTD exposé au serveur MCP) ; `dart info record-performance` ; Git LFS dans pub ; Genkit Dart en préversion ; Dart sur Cloud Functions for Firebase (**expérimental**) `[34]` | `prefer_initializing_formals` signale les nouveaux cas `[16]` |
| **3.13** | 2026-08-12 | ***Primary constructors* stables** `[16]` ; syntaxe `new`/`factory` concise ; corps vide `;` `[35]` | 6 nouveaux lints (`use_declaring_parameters`, `empty_container_bodies`…) ; formateur : imports en sections, heuristiques de chaînes ; chargement différé dart2wasm (préversion) ; élagage natif `@RecordUse` + `package:record_use` `[39]` ; `no_raw_types`/`no_dynamic_casts` remplacent les options `strict-*` ; `Future.pause`, `List/Map.unmodifiableOf` | **`final`/`var` interdits sur les paramètres non déclarants** (selon la version de langage) `[40]` ; correction de solidité de la promotion de type (#62889) `[39]` ; `NetworkInterface.addresses` → `List<InterfaceAddress>` ; API `Isolate` synchrones exposées par erreur `[16]` |
| **3.14** | 2026-11-11 | **Aucune fonctionnalité nouvelle** (correctif de solidité sur la promotion par sous-motif) `[35]` | **`dart migrate`** ; DartPad réécrit (bêta ; compilateur, analyzer et pub en Wasm ; paquets pub.dev) ; dart2wasm : `try_table` standard, SIMD 128 bits matériel (jusqu'à 4x), `--standalone` sans hôte JS (Wasmtime) ; ETag dans pub (environ 1,5 Mo économisés sur 144 dépendances) ; FFIgen/JNIgen stables `[8][35]` ; `dart run -e` ; `pub upgrade pkg@constraint` ; lint `future_sync_value` | Retrait des API `Isolate.runSync`… exposées par erreur en 3.13 ; `toString()` SIMD → `'V128'` ; `dart.library.isolate` = false sous dart2wasm ; **`dart:mirrors` déprécié** ; `--standalone` ne supporte plus `dart:js_interop` ; `posix_spawn` sur macOS (résolution du `PATH` modifiée) `[16]` |

État des fonctionnalités de langage :

| Fonctionnalité | État |
|---|---|
| Macros | Abandonnées le 2025-01-29 (billet de Vijay Menon, « An update on Dart macros & data serialization » : « we've made the difficult decision to stop our work on macros »). Rien ne les remplace en 2026. |
| Augmentations | Aucune livraison stable en 2026 dans les sources du rapport `[NV]`. |
| *Dot shorthands* | Livrés en 3.10 (novembre 2025), polis en 3.11 `[35][41]`. |
| *Null-aware elements* | Livrés avant 2026 (3.8). |
| *Primary constructors* | Stables en 3.13 `[42][43]`. |
| Dart 3.15 (prévisionnel, non sorti) | Promotion de `this` ; promotion préservée aux points de jonction ; une seule clause `show`/`hide` par directive (erreur de compilation) `[35]`. Le billet 3.14 cite aussi les « static extensions » en cours `[8][35]`. |

Lecture `[A]` : deux versions sur quatre sans fonctionnalité de langage. L'effort Dart de 2026 a porté sur l'outillage (analyzer, pub, migrate), le web/Wasm et l'interop.

---

## 6. Écosystème, gouvernance, critiques

- **Google I/O 2026**, chiffres revendiqués par Google dans le billet 3.44 :
  - « the second most popular development SDK for mobile on both major app stores with over 1.5 million monthly developers, a 50% increase in just one year » ;
  - 1,3 milliard de téléchargements pub.dev sur 30 jours ; plus de 1 700 contributeurs ; 972 commits de 178 contributeurs (dont 61 nouveaux) pour le seul cycle 3.44 ;
  - Flutter dans le système multimédia du Toyota RAV4 2026 `[9][30]` ;
  - SDK webOS de LG annoncé « in the coming weeks » `[9]` ; sortie effective `[NV]`.
- **3.50** : plus d'un million d'apps Flutter publiées sur Google Play (chiffre Google) `[7]`.
- **Gouvernance** :
  - Canonical responsable du desktop ; RFC publiques Flutter et Dart ; Google dit vouloir élargir la gouvernance « with more partners » `[9]`.
  - Enquête du T2 2026 (citée par ecorpit.com, `[S]`) : « over 70% of currently open PRs in the Flutter project come from contributors outside of Google » ; proposition d'une échelle de contributeurs à 4 niveaux (Contributor, Reviewer, Committer, Maintainer).
  - Lecture `[A]` : externalisation partielle de la maintenance. Signal positif de pérennité, et aveu que Google ne finance pas seul tous les fronts.
- **Critiques** :
  - Le fork **Flock**, annoncé le 2024-10-27 par Matt Carroll (« We're forking Flutter. This is why. », getflocked.dev : « The Flutter team has a labor shortage »), reste le symbole des frustrations : investissement jugé insuffisant, desktop délaissé, revues lentes.
  - Matt Carroll le présente comme une soupape (« The Three Stages of Flock », 2024-10-31) : « a community fork of Flutter that acts as a pressure release valve for Flutter users ».
  - Une source secondaire (devnewsletter, « State of Flutter 2026 ») note que l'investissement de Google est passé « from expansion to maintenance and refinement » `[S]`.
  - Activité réelle de Flock en 2026 : aucune source primaire `[NV]`.
  - Volume d'issues : numéros au-delà de #196 000 en novembre 2026 ; pas de métrique fiable de l'arriéré.
- **Événements communautaires** (FlutterCon, Fluttercon Europe…) : non couverts `[NV]`.

---

## 7. IA et Flutter : livré ou promis

| Brique | État réel fin 2026 |
|---|---|
| Serveur MCP Dart/Flutter + rechargement à chaud par agent | Livré (3.12/3.44) ; découverte automatique de l'app via DTD `[31]` ; outils regroupés pour réduire les jetons `[9][34]` |
| Agent Skills, plugins d'agent officiels, *package skills* (`skills/`, `skills_lint`) | Livré (3.44 → 3.50) `[7]`. Le plus utile pour un agent de code `[A]` |
| GenUI SDK (`genui`) | **Alpha** : 0.10.x, A2UI v0.9 ; refonte annoncée en `a2ui_core`/`a2ui_agent`/`a2ui_flutter`, cassante. +500 % de téléchargements revendiqués, sur une base inconnue `[9][44]` |
| A2UI | Protocole ouvert de Google ; v0.9 publiée le 2026-04-17 (moteurs de rendu Flutter, Lit, Angular, React) ; v0.9.1 actuelle ; **v1.0 en version candidate** `[45][46]` `[S]` |
| Genkit Dart | Préversion à I/O `[34]` ; **1.0 stable annoncée en 3.50** `[7]` |
| Firebase AI Logic | Disponible (appels Gemini côté client) `[47]` ; Server Prompt Templates ; skills Firebase `[9]` |
| Sur l'appareil | Paquet MediaPipe (20 tâches, LiteRT) ; LiteRT-LM dans `flutter_gemma` annoncé « coming soon » à I/O `[7][47]` ; livraison `[NV]` `[9]` |
| Flutter AI Toolkit | 1.0.0 (publié le 2025-12-15 selon une source secondaire) `[44]` |

Verdict `[A]` : GenUI/A2UI convient à une présentation ou à un prototype, pas à la production. Si adopté, l'isoler derrière une interface dans la couche *presentation* ; aucun schéma A2UI ne doit atteindre le domaine.

---

## 8. Impact sur une pile bloc + get_it/injectable + retrofit + freezed + bdd_widget_test

### 8.1 État des paquets

| Paquet | État 2026 selon le rapport | Risque `[A]` | Vérification locale `[E]` |
|---|---|---|---|
| **bloc / flutter_bloc** | bloc 9.2.1 (2026-05-12), flutter_bloc 9.1.1 ; pas de majeure en 2026 `[48][49]` | Faible. Seuls les imports `material.dart` des widgets sont concernés | flutter_bloc 9.1.1 résolu |
| **get_it** | 9.3.0 (2026-09-19) : détection des auto-résolutions circulaires de singletons paresseux, extension DevTools `[50][51]` | Faible ; attention à la **libération stricte en ordre LIFO** introduite en 9.0.0 `[52]` (date contradictoire entre changelog et pub.dev) | 9.3.0 résolu |
| **injectable_generator** | 3.1.3 (via `lean_builder`) ; analyzer `>=10 <15` `[53]` | **Élevé** : incompatible avec `build_runner` ≥ 2.17.1 (qui exige `build` 5) et analyzer ≥ 14.6 (issue #554 du 2026-10-29, #552 depuis le 08-15) `[54][55]` ; aucun correctif publié trouvé `[53][54]` | 3.1.3 plafonne à `build ^4.0.4` ; se résout avec analyzer 14.1.0 et `lean_builder` 1.2.2 |
| **retrofit / retrofit_generator / dio** | retrofit_generator 10.2.12 (2026-10-13) `[56]` ; 10.3.0 annonce analyzer `>=13.3 <15` ; dio 5.11.1 `[56][57]` | Moyen : support de `build` 5 `[NV]` | Non testé |
| **freezed** | 4.0.0 (2026-08-22, *primary constructors*, analyzer 13) ; **4.0.2 exige analyzer 14** ; 4.0.3 dernière | Moyen à élevé : `final` dans les paramètres n'est plus supporté (Dart 3.13), d'où des réécritures possibles des `@unfreezed` `[58][59]`. Une PR de doc (#1386) indique que les *primary constructors* ne supportent pas `fromJson`/`toJson` `[60]` | 4.0.3 résolu avec `build` 4.0.11 |
| **json_serializable** | 6.14.1 (2026-07-30) ; 6.13.1 exigeait analyzer `<13` et `build ^4.0.4` `[61]` | Moyen : contraintes de 6.14.1 `[NV]` `[62][63]` | 6.15.2 résolu avec `build` 4.0.11 |
| **build_runner** | 2.17.1 (2026-10-14), exige **build 5.0.0** ; 2.17.0 retirée `[64][65][66]` ; 2.13 apportait des builds 1,4x à 4x plus rapides `[54][64]` | Au centre du conflit | 2.16.1 résolu dès que `injectable_generator` est présent |
| **bdd_widget_test** | 3.0.x : analyseur Gherkin officiel ; **les fichiers `.feature` mal formés font échouer le build** ; SDK ≥ 3.8 ; étape golden intégrée (`matchesGoldenFile`) en remplacement de `golden_toolkit` (abandonné) `[67]` ; 3.0.5 sur GitHub le 11/11 `[68]` | Moyen : migration vers un Gherkin strict ; compatibilité analyzer 14 non documentée | 3.1.0 existe et exige `build` 5 ; 3.0.5 se résout avec `build` 4 |

### 8.2 Les trois options face au conflit

Constat du rapport : en novembre 2026, aucune combinaison publiée ne réunit `freezed` dernière version, `injectable_generator` et `build_runner` 2.17.x `[54]`. La vérification locale confirme l'impossibilité avec `build_runner` 2.17.x, et montre que `freezed` 4.0.3 n'est pas en cause (section 2).

| Option | Contenu | Avis du rapport |
|---|---|---|
| (a) Geler | analyzer 13 / build 4 / `build_runner` 2.16.x / freezed 4.0.0-4.0.1 | Recommandée à court terme. La vérification locale montre que le gel peut inclure analyzer 14.1.0 et freezed 4.0.3 `[E]` |
| (b) `dependency_overrides` | Forcer la résolution | Fragile, à proscrire en CI de production |
| (c) Remplacer injectable | Enregistrements get_it à la main dans le module d'injection | À réévaluer : les *primary constructors* ont beaucoup réduit le code de déclaration |

### 8.3 Tests

- Aucune rupture frontale sur `flutter_test` ni `integration_test` en 2026.
- Les goldens sont exposés à : césures conditionnelles, `WidgetSpan` de droite à gauche, Impeller desktop par défaut depuis 3.47 (rendu SDF, anticrénelage Windows/Linux modifié), `material_color_utilities` 0.13 depuis 3.41.
- Goldens générés sur desktop ou CI Linux : **les régénérer à chaque montée de version** et les isoler dans un job dédié `[A]`.

### 8.4 Clean Architecture

- *Primary constructors* et paramètres nommés privés réduisent le code des entités et cas d'usage du domaine, sans générateur `[34]`.
- `dart migrate --step=prepare` retire automatiquement les `final` sur paramètres, incompatibles avec 3.13 `[8][69]`.

---

## 9. Recommandations du rapport `[A]`

1. **Montée de version** : 3.47.x → **3.50.1** directement. Avant, vérifier les types de build Android personnalisés et les assets `.env`.
2. **Quatre PR distinctes**, jamais mélangées (sinon la bissection d'une régression devient impossible) :
   1. le SDK seul ;
   2. `dart migrate --dry-run`, puis `--apply --step=prepare`, avant de monter la contrainte SDK à ^3.13 ou ^3.14 ;
   3. `dart fix --apply --code=migrate_design_widgets` avec `MaterialUiCompatibilityBridge` ;
   4. la régénération des goldens.
3. **Génération de code** : figer `build_runner`, `analyzer`, `freezed`, `json_serializable`, `injectable_generator` et `retrofit_generator` dans un `pubspec.lock` versionné. Suivre l'issue injectable #554 ; si elle n'est pas résolue d'ici la 3.53 (février 2027), sortir injectable.
4. **CI** : ne pas passer `--fatal-infos` tant que la migration `material_ui` n'est pas terminée. Ajouter `flutter build web --no-deprecated-js-interop` si le web est ciblé.
5. **iOS** : réactiver SwiftPM avant le 2026-12-02. Auditer les `AppDelegate` personnalisés pour `UIScene`. Planifier le remplacement des Mac Intel de CI.
6. **Android** : migrer vers le Kotlin intégré. Auditer les plugins internes, qui ne se construiront plus sous AGP 10.
7. **Agents de code** : adopter le serveur MCP Dart/Flutter et les *package skills*. Désactiver ou encadrer la règle `flutter-hot-reload` dans un flux TDD : un rechargement après chaque édition n'apporte rien quand la boucle de retour est `flutter test`. Ajouter ses propres *skills* (Clean Architecture, conventions bloc, étapes BDD) dans un dossier `skills/` versionné.
8. **Desktop et GenUI** : multi-fenêtres et GenUI restent en veille technologique, pas en feuille de route produit 2026.

### 9.1 Ce qui compte

- Le découplage Material/Cupertino : dette de migration certaine.
- Le conflit de contraintes de la génération de code.
- Le passage forcé à SwiftPM, au Kotlin intégré, à `UIScene` et à Apple Silicon.
- L'interop native stable.
- `dart migrate`.
- La cadence trimestrielle désormais prévisible.

### 9.2 Ce qui est du bruit ou prématuré

- Les chiffres « 2x Wasm » et « +500 % GenUI » : bancs d'essai et bases non publiés.
- Les annonces embarquées (LG webOS, LiteRT-LM « coming soon ») non vérifiées comme livrées.
- Le multi-fenêtres, expérimental depuis un an.

### 9.3 Chiffres à citer comme déclarations de Google

1,5 million de développeurs mensuels (+50 % en un an) ; 1,3 milliard de téléchargements pub.dev sur 30 jours ; plus d'un million d'apps sur Google Play ; plus de deux fois la fréquence d'images du JavaScript en Wasm ; +500 % de téléchargements de GenUI.

---

## 10. Limites déclarées par le rapport

Non vérifié sur source primaire `[NV]` :

- la bêta 3.51 ;
- l'état actuel de Flock ;
- les annonces de FlutterCon et Fluttercon Europe 2026 ;
- la sortie effective de LG webOS et de LiteRT-LM ;
- les contraintes exactes de json_serializable 6.14.1 et de retrofit_generator 10.2.12 vis-à-vis de `build` 5 ;
- la présence de bdd_widget_test 3.0.5 sur pub.dev (levée par la section 2 : présente) ;
- l'état ouvert ou fermé des issues injectable #552 et #554 au 2026-11-21.

Autres réserves :

- Plusieurs dates diffèrent d'un à deux jours entre docs, billets et changelogs (Dart 3.11, Dart 3.12, Flutter 3.44).
- Plusieurs articles secondaires contiennent des erreurs manifestes (« Dart 3.41 », « Flutter 4.0 » inexistants) `[70]`. Seules les sources primaires fondent les faits structurants.

---

## 11. Aide-mémoire de commandes

```bash
dart fix --apply --code=migrate_design_widgets   # migration vers material_ui / cupertino_ui
flutter pub add material_ui                      # si l'outil de migration échoue sur pubspec.yaml
dart migrate --dry-run                           # aperçu des migrations de langage
dart migrate --apply --step=prepare              # retire les final sur paramètres (Dart 3.13)
flutter build web --no-deprecated-js-interop     # échoue si une dépendance importe dart:html, dart:js…
flutter build web --web-content-hash             # noms de fichiers hachés + precache_manifest.json
flutter config --enable-macos-arm64-only         # outils sans Rosetta
dart run skills@ get                             # installe les skills fournis par les paquets
dart pub cache gc                                # nettoyage du cache pub (Dart 3.11)
```

---

## 12. Sources

1. https://github.com/koniz-dev/flutter-starter/issues/1225
2. https://flutterreleases.com/release/3.50.0/
3. https://github.com/flutter/flutter/issues/196743
4. https://github.com/vespr-wallet/ledger-cardano-plus/pull/32
5. https://blog.flutter.dev/whats-new-in-flutter-3-41-302ec140e632
6. https://devopen.io/flutter-3-50-release-date/
7. https://flutter.dev/blog/whats-new-in-flutter-3-50
8. https://dart.dev/blog/announcing-dart-3-14
9. https://flutter.dev/blog/whats-new-in-flutter-3-44
10. https://www.fluttersolution.com/2026/05/flutter-344-dart-312-everything-new-at.html
11. https://flutter.dev/blog/whats-new-in-flutter-3-47
12. https://verygood.ventures/blog/flutter-3-50-dart-3-14/
13. https://blurbrahlab.medium.com/flutter-3-50-is-out-and-material-dart-is-deprecated-top-10-ai-flutter-news-november-12-2026-a842b887c439
14. https://docs.flutter.dev/release/breaking-changes/material-ui-and-cupertino-ui
15. https://daily.dev/posts/what-s-new-in-flutter-3-50-51qn8sa9l
16. https://dart.dev/changelog
17. https://docs.flutter.dev/perf/impeller
18. https://github.com/DonutWare/Fladder/issues/1186
19. https://github.com/canonical/flutter-sdk/pull/30
20. https://github.com/flutter/flutter/issues/196587
21. https://github.com/flutter/flutter/issues
22. https://flutter.dev/blog
23. https://github.com/flutter/flutter/issues/196696
24. https://docs.flutter.dev/release/breaking-changes
25. https://docs.flutter.dev/release/release-notes/release-notes-3.50.0
26. https://dart.dev/resources/whats-new
27. https://newblog.flutterforbeginners.com/flutter/flutter-3-41-release-notes-2026/
28. https://alexanderobregon.substack.com/p/what-changed-in-flutter-341-and-why
29. https://docs.flutter.dev/release/whats-new
30. https://dev.to/ianjasperrr/flutter-344-highlights-from-google-io-2026-whats-new-and-what-matters-g6f
31. https://ranveergour781.medium.com/flutter-3-44-dart-3-12-everything-you-need-to-know-from-google-i-o-2026-8c641a87d75c
32. https://yawarothman.medium.com/flutter-3-44-dart-3-12-everything-that-changed-at-google-i-o-2026-16f73a0c03d5
33. https://dart.dev/resources/language/evolution?s=09
34. https://dart.dev/blog/announcing-dart-3-12
35. https://dart.dev/resources/language/evolution
36. https://dart.googlesource.com/sdk/+/stable/CHANGELOG.md
37. https://dart.dev/blog/announcing-dart-3-11
38. https://blog.dart.dev/announcing-dart-3-11-b6529be4203a?gi=c4643f6fbdf5
39. https://dart.dev/blog/announcing-dart-3-13
40. https://sesamedisk.com/dart-programming-language-stable-primary-constructors/
41. https://ecorpit.com/dart-3-12-primary-constructors-dot-shorthands-guide-2026/
42. https://gamesinflutter.com/news/dart-3-13-primary-constructors-stable
43. https://dev.to/marwamejri/where-dart-stops-and-freezed-starts-4d8a
44. https://www.afaqzahir.com/blog/flutter-genui-sdk-generative-ui-guide
45. https://developers.googleblog.com/a2ui-v0-9-generative-ui/
46. https://akanshajain.dev/talks/genui/
47. https://blog.flutter.dev/whats-new-in-flutter-3-44-b0cc1ad3c527?gi=854b1b40da44
48. https://github.com/c0ldplasma/coldchat/pull/42
49. https://pub-web.flutter-io.cn/packages/bloc/changelog
50. https://github.com/sifterstudios/tava/pull/36
51. https://pub.dev/packages/get_it
52. https://github.com/flutter-it/get_it/blob/main/CHANGELOG.md
53. https://pub.dev/packages/injectable_generator/changelog
54. https://github.com/Milad-Akarie/injectable/issues/554
55. https://github.com/Milad-Akarie/injectable/issues
56. https://pub.dev/packages/retrofit_generator/changelog
57. https://pub.dev/packages/dio/changelog
58. https://github.com/rrousselGit/freezed/blob/master/packages/freezed/CHANGELOG.md
59. https://pub.dev/packages/freezed/changelog
60. https://github.com/rrousselGit/freezed/pull/1386
61. https://pub.dev/packages/json_serializable/changelog
62. https://github.com/trakli/mobile/pull/231
63. https://github.com/mobxjs/mobx.dart/pull/1090
64. https://github.com/dart-lang/build/blob/master/build_runner/CHANGELOG.md
65. https://pub.dev/packages/build_runner/versions
66. https://github.com/dart-lang/build/releases/tag/build_runner-v2.17.1
67. https://pub.dev/packages/bdd_widget_test/changelog
68. https://github.com/olexale/bdd_widget_test/releases/tag/v3.0.5
69. https://dart.dev/tools/dart-migrate
70. https://www.200oksolutions.com/blog/flutter-3-41-new-features-performance-updates-guide/
