# Architecture de référence « cœur Rust + coques » — dossier pour agent

> **Destinataire** : un agent Claude Code (modèle frontière) chargé d'amorcer puis de faire évoluer un produit déployé sur desktop (Windows, Linux, macOS), mobile (iOS, Android), web et terminal.
> **Date de l'étude** : 2026-11-16. Les versions et statuts cités sont ceux de cette date ; revérifie-les avant de les figer dans un fichier de build.
> **Document compagnon** : `architecture-multiplateforme.html` (même dossier), version illustrée pour lecteur humain.

## 0. Comment lire ce document

### 0.1 Niveaux de preuve

Chaque affirmation factuelle porte un marqueur. Ne promeus jamais un `[J]` ou un `[NV]` au rang de fait dans un commit, une ADR ou une réponse à l'utilisateur.

| Marqueur | Signification |
|---|---|
| `[V]` | Vérifié par vote contradictoire (3 vérificateurs indépendants, 3-0) sur source primaire. |
| `[P]` | Lu sur source primaire (doc officielle, dépôt, registre) par un seul agent de recherche, sans contre-vérification. |
| `[E]` | Établi par un essai mené dans ce dépôt, consigné dans une ADR de `docs/architecture/adr/`. Vaut pour les versions et le périmètre de l'essai. |
| `[S]` | Source secondaire (blog, retour d'expérience d'un seul auteur, contenu d'éditeur). |
| `[J]` | Jugement d'ingénierie de l'auteur de l'étude : raisonné, mais non sourcé. |
| `[NV]` | Non vérifié ou non trouvé. À établir par un essai (voir §14) avant de s'y fier. |

### 0.2 Mots-clés normatifs

`DOIT` / `NE DOIT PAS` : règle d'architecture, à faire respecter par linter ou revue. `DEVRAIT` : défaut recommandé, dérogation possible avec ADR. `PEUT` : option.

### 0.3 Ce que ce document n'est pas

- Les extraits de code sont **illustratifs et non compilés**. Valide chaque API contre la documentation de la version épinglée (outil context7 ou doc officielle) avant de l'écrire.
- Le backend serveur n'est traité que par son contrat avec le client (protocole de synchronisation).
- Aucune mesure de performance n'a été faite.

---

## 1. Contexte et contraintes fixées par l'utilisateur

| Sujet | Décision de l'utilisateur |
|---|---|
| Cibles | GUI Flutter sur Windows, Linux, macOS, iOS, Android ; web ; TUI en Rust. |
| Modèle applicatif | **Offline-first avec synchronisation serveur.** |
| Web | **Deux besoins** : partie publique indexable (SEO, chargement rapide, accessibilité) **et** partie applicative riche derrière authentification. |
| Répartition Rust/Dart | Laissée à l'étude (tranchée en §3.2). |
| TUI | Parité fonctionnelle **ou** sous-ensemble, selon le projet : l'architecture doit permettre les deux. |
| Flutter | Feature-Sliced Design + Clean Architecture ; `injectable` + `get_it` ; `flutter_bloc` **sans Cubit**, events/states en hiérarchies `sealed` ; BDD Gherkin via `bdd_widget_test`. |
| Rust | Architecture hexagonale pilotée par des tests de comportement Gherkin. |
| Méthode | Double boucle BDD/TDD ; linters vierges à chaque étape ; DDD ; design system en atomic design documenté dans Widgetbook ; validation continue (CI, gates). |

---

## 2. Résumé exécutif

1. **Un cœur Rust hexagonal** porte le domaine, les cas d'usage, la persistance locale et la synchronisation. Il est consommé directement par la TUI et le site public (appel Rust natif), et par Flutter à travers un pont généré. `[J]`, cohérent avec les pratiques de Bitwarden, Element X, AppFlowy `[P]`.
2. **Pont : `flutter_rust_bridge` v2** (2.13.0 stable). C'est le seul pont vérifié qui annonce les six cibles, web compris, avec async et streams `[V]`. Il a un risque de mainteneur unique `[P]` : on le confine dans un crate mince et remplaçable.
3. **La présentation reste en Dart** : BLoC `sealed` sans Cubit, inchangé. Rust expose des commandes et des flux de modèles de lecture ; le BLoC n'a plus de règle métier `[J]`. L'option « état de présentation dans Rust » est écartée par défaut : elle n'a aucun outillage pour Flutter `[V]` `[P]`.
4. **Web applicatif : Flutter Web + cœur Rust en WASM**, servi sur un sous-domaine isolé (COOP/COEP) `[V]`. **Web public : site DOM séparé**, Leptos par défaut `[J]`, comme le recommande la doc Flutter pour tout contenu indexable `[V]`.
5. **Synchronisation : moteur maison dans le cœur** (file de mutations, tirage par curseur, serveur d'autorité) `[J]`. Raison décisive : le SDK Rust de PowerSync ne fonctionne pas dans le navigateur `[P]`, ce qui casserait le cœur unique.
6. **Une seule spécification Gherkin** à la racine, exécutée par cucumber-rs (cœur, TUI) et par `bdd_widget_test` (Flutter) via une copie synchronisée vérifiée en CI `[J]` sur faits `[P]`.
7. **Point le plus incertain** : la persistance SQLite hors ligne dans le navigateur depuis Rust. Les briques existent `[P]`, leur assemblage avec le pont n'est pas vérifié `[NV]`. Un essai (S2, §14) doit trancher avant tout engagement sur le web applicatif.

---

## 3. Décisions d'architecture (ADR condensées)

### 3.1 ADR-01 — Cœur Rust partagé, coques par cible

**Décision.** Tout ce qui n'est pas rendu, navigation ou intégration plateforme vit dans un Cargo workspace Rust.

**Preuves.**
- Bitwarden : « everything except presentational logic belongs in the SDK » ; UniFFI pour le mobile, wasm-bindgen pour le web `[P]`.
- Element X (matrix-rust-sdk + UniFFI), Signal (libsignal, crypto et protocole seulement), Mozilla application-services : même forme `[P]`.
- AppFlowy : Flutter + Rust, quatre couches DDD réparties entre Dart et Rust, `flutter_bloc` côté Dart jouant la couche application sans logique métier complexe `[P]`.
- La logique placée dans Rust se teste sans harnais Flutter ni émulateur `[S]` (Very Good Ventures).

**Contre-indication à connaître.** Very Good Ventures juge une couche de données Rust excessive pour une app qui n'est qu'une API REST plus un SQLite local, et la réserve aux cas où la correction est non négociable `[S]`. Ici le cœur se justifie par une contrainte que VGV n'avait pas : **la TUI Rust et le site public doivent réutiliser la même logique** `[J]`.

### 3.2 ADR-02 — Répartition : Rust jusqu'aux cas d'usage, présentation en Dart

Deux options étaient à comparer.

| | (a) Rust = domaine + cas d'usage ; BLoC Dart = présentation | (b) Rust porte aussi l'état de présentation |
|---|---|---|
| Outillage Flutter | FRB v2 : six cibles annoncées, async, flux `[V]` | Aucun : Crux ne génère pas de types Dart (Swift, Kotlin, TypeScript, C# seulement) `[V]` ; l'essai `ui_state` de l'auteur de FRB est resté expérimental, issue close sans suite `[P]` ; le seul exemple public FRB + cœur façon Crux se déclare obsolète `[P]` |
| Pratiques utilisateur | Intactes : BLoC sealed, `bloc_test`, `bdd_widget_test` | BLoC réduit à un passe-plat ; les tests de BLoC perdent leur objet |
| Références de production | AppFlowy `[P]`, VGV `[S]` | 1Password (view models dans Rust, UI non Flutter) `[P]`, Proton et Photoroom avec Crux et UI natives `[S]` ; **aucun cas Flutter trouvé** `[NV]` |
| Duplication GUI/TUI | La machine à états de présentation existe deux fois (BLoC Dart, modèle TUI Rust) | Aucune |
| Trafic sur le pont | Appels au niveau cas d'usage, grain grossier | Chaque interaction traverse le pont ; flux signalés instables sur le web `[V]` |

**Décision : (a), avec une soupape.** `[J]`

- Rust DOIT porter : entités, objets-valeur, invariants, cas d'usage, modèles de lecture, persistance, synchronisation.
- Dart DOIT porter : états d'écran, navigation, formulaires, animations, intégrations plateforme propres à Flutter.
- **Soupape (a+)** : quand une logique d'écran est assez complexe pour que GUI et TUI ne doivent pas diverger (tri, regroupement, filtrage, droits d'affichage, validation de formulaire), le cœur DEVRAIT exposer un **modèle de lecture au niveau de l'écran** (requête qui renvoie une structure prête à afficher). Le BLoC et le modèle TUI l'enveloppent sans la recalculer. On obtient l'essentiel du partage de (b) sans dépendre d'un outillage inexistant.
- Un BLoC NE DOIT PAS contenir de règle métier. Critère de revue : si la même règle devrait être réécrite dans la TUI, elle appartient au cœur.

### 3.3 ADR-03 — Pont Rust↔Dart : flutter_rust_bridge v2

**Comparatif.**

| Pont | État au 2026-11-16 | Web | Verdict |
|---|---|---|---|
| **flutter_rust_bridge** | 2.13.0 stable (2026-08-23), 2.14.0-beta.2 (2026-09-12) `[V]`. Six cibles annoncées, async, streams Rust→Dart `[V]`. Flutter Favorite `[P]`. | Oui, via wasm-bindgen et Web Workers `[V]` | **Retenu** |
| UniFFI + Dart | `uniffi-dart` v0.2.1, badge « experimental », absent de pub.dev ; `uniffi-bindgen-dart` 0.2.0, 3 étoiles `[P]`. Une bibliothèque publiée l'utilise (bdk_dart 1.0.1) `[V]`. | Non attesté `[V]` | Écarté : pas de web, immature |
| Rinf | 8.11.0 (2026-10-07). Modèle messages/acteurs, état recommandé dans Rust, web « tested and supported » `[P]`. | Oui `[P]` | Écarté : avis « Looking for a New Maintainer » en tête du README `[P]` |
| Crux | Cœur façon Elm, trois méthodes, tout en octets sérialisés `[V]`. | Oui (TypeScript) | Écarté pour Flutter : pas de typegen Dart `[V]` |
| FFI brut (`ffigen` + `cbindgen`) | Aucun retour de production trouvé `[NV]`. | Non | Écarté : tout à écrire à la main, pas de web |

**Risques propres à FRB, tous `[P]` sauf mention.**
- **Mainteneur unique** : 18 856 commits pour l'auteur contre 377 pour le suivant.
- **Issues fermées automatiquement** en `wontfix` par un bot d'inactivité : le faible nombre d'issues ouvertes (11) ne mesure pas la qualité.
- **Web** : plusieurs issues de démarrage ou d'édition de liens WASM closes sans correctif (#3326, #2829, #3009, #2757).
- **Panics** : les `.unwrap()` du code généré tuent le processus sous `panic = "abort"` (#3067, clos « not planned »).
- **Isolates** : mutex bloqué après l'arrêt d'un isolate (#2560).
- **Types opaques** : `Arc<dyn Trait>` génère deux classes (#2856).
- **RustDesk**, principal utilisateur public, est resté sur FRB **v1** (1.80.1) ; la raison n'est pas documentée `[NV]`.
- Runtime, macros et codegen sont épinglés à la même version exacte `[P]`.

**Règles qui en découlent.** `[J]`
1. Le crate `bridge` est une **couche anticorruption mince**. Aucun autre crate NE DOIT dépendre de `flutter_rust_bridge`.
2. L'API du pont DOIT être **à gros grain** : une fonction exposée appelle exactement une commande ou une requête de `application`, et n'échange que des DTO simples. Pas de `Arc<dyn Trait>`, pas de types à durée de vie, pas de types de crates tiers.
3. Toute fonction exposée DOIT renvoyer `Result<T, BridgeError>`. `unwrap`, `expect` et `panic!` sont interdits par clippy dans `bridge`. Le profil release NE DOIT PAS utiliser `panic = "abort"`.
4. La version de FRB DOIT être épinglée exactement, identique dans `Cargo.toml`, `pubspec.yaml` et l'outil de génération.
5. Le code généré est versionné et la CI vérifie qu'il est à jour (§11.3).
6. Si FRB devient intenable, seul `bridge` et le paquet `core_bridge` sont à réécrire (Rinf ou UniFFI) : c'est le bénéfice attendu de la règle 1.

### 3.4 ADR-04 — Compilation du Rust dans le build Flutter : Native Assets

- FRB propose deux backends via `--integration-backend` : `cargokit` (défaut, « pour compatibilité ») et `native-assets` (conseillé aux nouveaux projets sur SDK récent) `[P]`.
- Le dépôt amont `irondash/cargokit` est archivé (dernier push 2025-06-07) ; FRB maintient son propre fork `[P]`.
- Native Assets : `hooks` 2.2.0, `native_toolchain_rust` 1.1.1 (éditeur tiers, mainteneur unique), `flutter_rust_bridge_hooks` 2.13.0 `[P]`. Exige `crate-type = ["staticlib", "cdylib"]` et un `rust-toolchain.toml` qui épingle une version concrète et liste les cibles `[P]`.
- Les hooks couvrent Android, iOS, Linux, macOS, Windows, **pas le web** `[P]`.

**Décision.** Nouveau projet : `native-assets` `[J]`. Repli : `cargokit` si l'essai S1 échoue dans un Cargo workspace (cas non vérifié `[NV]`).

### 3.5 ADR-05 — Web applicatif : Flutter Web + cœur Rust en WASM

**Faits.**
- Le mode par défaut (threadé) de FRB sur le web exige `Cross-Origin-Opener-Policy: same-origin` et `Cross-Origin-Embedder-Policy: credentialless` ou `require-corp` `[V]`.
- Safari ne supporte pas `credentialless` (jusqu'à 27.x) ; Firefox Android non plus `[P]`. **Il faut donc `require-corp`**, qui oblige toute ressource tierce à porter CORP ou CORS.
- Mode sans en-têtes possible mais déconseillé : `default_dart_async: false`, tout tourne en synchrone sur le fil principal `[V]`.
- `flutter_rust_bridge_codegen build-web` est obligatoire (pas de hook web). Il lance `wasm-pack` avec la **toolchain nightly**, `-Z build-std=std,panic_abort`, `--target no-modules`, `+atomics,+bulk-memory,+mutable-globals` `[P]`.
- Flutter 3.50 : un build `--wasm` ne tourne en dart2wasm par défaut que sur Chromium 137+ ; Safari 26.2+ et Firefox 147+ retombent sur dart2js + CanvasKit sauf activation par `wasmAllowList` `[V]`.
- La sortie Dart WebAssembly de FRB n'existe que dans 2.14.0-beta.2, pas dans 2.13.0 stable `[P]`.
- Accessibilité Flutter Web : arbre sémantique désactivé par défaut, à activer par `SemanticsBinding.instance.ensureSemantics()` `[P]`. URL en hash par défaut ; `usePathUrlStrategy()` exige une réécriture serveur `[P]`.
- Poids indicatif (un seul banc d'essai, Flutter 3.47.4, brotli) : skwasm ≈ 1,65 Mo, CanvasKit ≈ 1,98 à 2,70 Mo, plus le code applicatif `[S]`.

**Décisions.** `[J]`
- Tant que FRB stable est en 2.13 : construire l'app web en **dart2js + CanvasKit** (`flutter build web`, sans `--wasm`). Réévaluer à la sortie stable de 2.14.
- L'app web est servie sur une **origine dédiée** (`app.exemple.com`) avec `COOP: same-origin` et `COEP: require-corp`, en HTTPS.
- Authentification tierce **par redirection**, pas par popup : sous COOP `same-origin`, la référence `opener` vers un popup non isolé est coupée `[P]`.
- Aucune ressource tierce sans CORP/CORS dans l'app (polices, images, scripts d'analyse) : auto-héberger.
- Appeler `ensureSemantics()` au démarrage et `usePathUrlStrategy()`.

### 3.6 ADR-06 — Web public : site DOM séparé

**Fait fondateur.** La documentation Flutter dit que Flutter Web n'est pas adapté aux contenus statiques riches en texte, que sa sortie ne correspond pas à ce que les moteurs de recherche attendent, et recommande de séparer l'expérience applicative des pages d'accueil, marketing et aide `[V]`.

**Candidats.**

| Outil | Version | Rendu | Réutilise le cœur Rust | Risque |
|---|---|---|---|---|
| **Leptos** | 0.8.23 (2026-10-31), 0.9.0-rc1 `[P]` | SSR + hydratation, îlots `[P]` | **Oui, nativement** côté serveur et en WASM côté client | Pré-1.0 |
| Dioxus | 0.7.10, 0.8.0-alpha.3 `[P]` | SSR, SSG `[P]` | Oui, idem | Pré-1.0, plus dispersé |
| Jaspr | 0.23.5, 0.24.0-beta.1 `[P]` | SSG, SSR, client `[P]` | Non (Dart) ; partage le code Dart et peut embarquer des widgets Flutter `[P]` | 0.x, projet largement mono-mainteneur `[P]` ; mais cité par la doc Flutter et utilisé pour dart.dev et flutter.dev `[V]` |
| Astro | 7.4.12 `[P]` | Îlots, SSG/SSR | En WASM côté client seulement, par plugin Vite ; voie non vérifiée avec Astro 7 `[NV]` | Ajoute TypeScript à la pile |
| SvelteKit | 3.2.0 `[P]` | SSR/SSG | Idem Astro `[NV]` | Ajoute TypeScript |

**Décision : Leptos par défaut.** `[J]` Raisons : la pile reste à deux langages (Rust, Dart) ; les crates `domain` et `application` sont appelés directement côté serveur ; les mêmes pratiques s'appliquent (hexagonal, cucumber-rs, clippy). Confiance moyenne : aucun des candidats n'a fait l'objet d'une comparaison vérifiée.

**Variantes.**
- **Jaspr** si le site public doit surtout réutiliser du code Dart ou embarquer des widgets Flutter. Contraintes de l'embarquement : Flutter ≥ 3.24, taille explicite (la vue Flutter ne se dimensionne pas seule), import côté client uniquement `[P]`.
- **Astro** si le site est essentiellement éditorial et tenu par des non-développeurs. Types TypeScript générables depuis Rust par `tsify` 0.5.8 ou `ts-rs` 12.0.1 `[P]`.

**Cohabitation.**
- `www.exemple.com` (public) : **sans** COOP/COEP, pour garder scripts tiers, intégrations et popups.
- `app.exemple.com` (Flutter) : isolée (ADR-05).
- Partage de session entre les deux par cookie de domaine parent : `[NV]`, à concevoir avec le backend. Un domaine personnalisé est nécessaire (les domaines d'hébergeur type `netlify.app` sont dans la Public Suffix List) `[P]`.
- En-têtes configurables sur Cloudflare Pages, Netlify, Vercel, Firebase Hosting `[P]`. **Pas sur GitHub Pages** (contournement par service worker `[S]`).

### 3.7 ADR-07 — Persistance locale

**Natif (desktop, mobile, TUI).** SQLite via `rusqlite` dans un crate adaptateur. Confiance haute `[J]`.

**Navigateur.** Faits :
- `rusqlite` 0.43.0 (2026-11-15) active par défaut `ffi-sqlite-wasm-rs`, qui remplace `libsqlite3-sys` par `sqlite-wasm-rs` sur `wasm32-unknown-unknown` `[P]`.
- `sqlite-wasm-rs` 0.6.3 : VFS mémoire par défaut, SQLite compilé avec `SQLITE_THREADSAFE=0` `[P]`.
- `sqlite-wasm-vfs` 0.3.0 : un VFS persistant documenté, `sahpool` (OPFS). Il exige un contexte sécurisé et un **Worker dédié** ; pas de connexions concurrentes, pas de WAL `[P]`.
- D'après la doc SQLite : `opfs-sahpool` ne demande pas COOP/COEP, mais **un second onglet sur la même origine échoue à l'initialisation** `[P]`.
- `diesel` 2.4.0 documente aussi le WASM via `sqlite-wasm-rs` `[P]`. `sqlx` : support WASM non trouvé `[NV]`.

**Non vérifié** `[NV]` : le fonctionnement de `sahpool` dans le pool de Workers de FRB ; la compatibilité d'un SQLite `THREADSAFE=0` avec un build `+atomics`.

**Décision : trois plans, départagés par l'essai S2.** `[J]`

| Plan | Description | Quand |
|---|---|---|
| **A** | Même adaptateur `rusqlite`, VFS `sahpool` sur le web. Verrou d'onglet unique (Web Locks) avec message « déjà ouvert ailleurs ». | S2 réussit. Cœur 100 % partagé. |
| **B** | Web sans stockage durable : adaptateur mémoire + réseau. L'app web exige une connexion. | S2 échoue et le hors-ligne web n'est pas prioritaire. |
| **C** | Sur le web, le port de stockage est implémenté en Dart (`sqlite3` WASM sur OPFS, voie utilisée par le SDK Flutter de PowerSync, dont le web est en bêta `[V]`) et injecté dans le cœur par rappel Rust→Dart. | S2 échoue et le hors-ligne web est requis. Coût : un adaptateur de plus, trafic accru sur le pont. |

Les ports de persistance (§5.3) rendent ces plans interchangeables sans toucher au domaine.

### 3.8 ADR-08 — Synchronisation

**État des solutions.**

| Solution | Faits | Compatible cœur Rust unique ? |
|---|---|---|
| PowerSync | SDK Rust bêta sur `rusqlite`, crate `powersync` 0.2.0 `[V]`. **Web non supporté** : issue #6 ouverte, mainteneur : pas de plan sans forte demande `[P]`. Dépend de `rusqlite 0.39` `[P]`, alors que le support WASM de `rusqlite` est lu sur la 0.43 : les deux ne cohabitent pas dans un même build. SDK Flutter 2.6.1 : web en bêta, OPFS, « dart2wasm has issues » `[P]`. Serveur d'autorité, conflits à la charge de l'app `[S]`. Apache 2.0, auto-hébergeable `[S]`. | Non sur le web |
| Electric | Moteur de synchronisation en lecture seule pour Postgres (shapes HTTP) ; clients TypeScript et Elixir, pas de client Rust `[P]`. | Non |
| Turso / libSQL | Crate `turso` 0.9.0-pre.5 avec `sync` (`push`/`pull`), « last push wins », Turso Cloud requis ; rachat par Supabase annoncé le 2026-10-02 `[P]`. Usage du crate en wasm32 non documenté `[NV]`. | Incertain |
| cr-sqlite | Dépôt non archivé, mais dernière release v0.16.3 de janvier 2024 `[P]`. | Risqué |
| Automerge | Crate 0.12.0 ; équivalent Rust d'automerge-repo : `samod` 0.16.0 ; paquet Dart = coquille vide 0.0.0 `[P]`. Limites mémoire en WASM rapportées par un utilisateur `[S]`. | Oui (derrière le cœur) |
| Loro | Crate 1.18.0 ; CRDT sans transport ni stockage `[V]` ; sa doc invite à regarder ailleurs si une synchronisation client-serveur simple suffit ; WASM ≈ 970 Ko gzippé `[V]`. | Oui (derrière le cœur) |
| Yrs | `yrs` 0.28.1 `[P]`. | Oui (derrière le cœur) |

**Décision : moteur maison dans le cœur.** `[J]`

Patron retenu, décrit par Replicache `[P]` et proche de celui de Figma `[P]` :
1. Toute écriture produit une **mutation** persistée localement dans la même transaction que la modification (boîte d'envoi), avec un identifiant séquentiel par client.
2. **Push** par lots vers le serveur, qui est l'autorité et applique ses propres règles.
3. **Pull** par **curseur opaque** : le serveur renvoie les changements depuis le curseur et, par client, le dernier identifiant de mutation pris en compte.
4. Le client applique l'état serveur puis **rejoue** les mutations encore en attente.
5. Résolution par défaut : **dernier écrivain gagne par champ**, ordre fixé par le serveur. Les invariants métier sont revérifiés côté serveur avec le même crate `domain`.
6. Horloge logique hybride (`uhlc` 0.9.0 `[P]`) uniquement si un ordre entre clients est nécessaire sans passer par le serveur.

**Variantes.**
- **CRDT** (Loro, Automerge ou Yrs, derrière un port du cœur) seulement s'il existe une vraie édition collaborative concurrente sur un même document. Les bindings Dart n'existent pas `[P]`, mais ils sont inutiles : le CRDT reste dans Rust.
- **PowerSync** si le backend est Postgres et qu'on renonce au cœur unique sur le web : SDK Rust pour natif et TUI, SDK Dart pour le web. Deux implémentations du même port à maintenir.

### 3.9 ADR-09 — TUI

- `ratatui` 0.30.2 + `crossterm` 0.29.0 `[P]`. `crossterm` n'a pas publié depuis avril 2025 `[P]` : à surveiller.
- La TUI DOIT dépendre de `application` (via la façade `core`) par appel Rust direct, **jamais** de `bridge`.
- Architecture interne façon Elm (modèle, message, mise à jour, vue) `[J]`.
- Parité ou sous-ensemble : piloté par les étiquettes Gherkin (§8.2).

### 3.10 ADR-10 — Spécification Gherkin unique

Voir §8. Décision : dossier `features/` à la racine, source de vérité ; copie synchronisée dans le paquet Flutter ; étiquettes par cible.

### 3.11 ADR-11 — Design tokens

Voir §9. Décision : JSON au format DTCG 2025.10 comme source ; génération vers Dart, CSS et Rust.

---

## 4. Vue d'ensemble

```
                        features/  (Gherkin, source unique)
                            │
        ┌───────────────────┼─────────────────────┐
        ▼                   ▼                     ▼
  Flutter (GUI)         TUI ratatui          Site public Leptos
  BLoC sealed           modèle Elm           SSR + îlots
        │                   │                     │
   core_bridge (Dart)       │ appel direct        │ appel direct
        │ FRB v2            │                     │
        ▼                   ▼                     ▼
  ┌─────────────────────────────────────────────────────┐
  │ crate bridge │            crate core (façade)       │
  ├──────────────┴──────────────────────────────────────┤
  │ application : cas d'usage, ports, modèles de lecture│
  │ domain      : entités, objets-valeur, invariants    │
  ├─────────────────────────────────────────────────────┤
  │ adapters : storage-sqlite │ sync-http │ platform    │
  └─────────────────────────────────────────────────────┘
        │                                 │
   SQLite (fichier / OPFS)          Serveur de sync (autorité)
```

Sens des dépendances : toujours vers `domain`. Les adaptateurs implémentent des ports déclarés par `application`.

---

## 5. Cœur Rust

### 5.1 Découpage en crates

| Crate | Rôle | Peut dépendre de | NE DOIT PAS |
|---|---|---|---|
| `domain` | Entités, agrégats, objets-valeur, événements de domaine, erreurs métier | rien du workspace ; bibliothèques pures seulement | faire d'E/S ; dépendre d'un runtime async, d'une bibliothèque de sérialisation ou de FRB (la sérialisation passe par des types miroirs dans `sync-protocol` et `bridge`) |
| `application` | Cas d'usage (commandes, requêtes), **ports** (traits), modèles de lecture | `domain` | connaître SQLite, HTTP, FRB, ratatui |
| `sync-protocol` | Types du protocole de synchronisation, sérialisables, partagés avec le serveur | `domain` (identifiants) | contenir de la logique |
| `adapters/storage-sqlite` | Implémente les ports de persistance et la boîte d'envoi | `application`, `domain` | être référencé par `domain` ou `application` |
| `adapters/sync-http` | Implémente le port de transport | `application`, `sync-protocol` | idem |
| `adapters/platform-*` | Trousseau, fichiers, horloge, notifications | `application` | idem |
| `core` | **Façade et racine de composition** : assemble adaptateurs et cas d'usage, expose un point d'entrée unique | tous les précédents | contenir de la logique métier |
| `bridge` | API FRB, DTO, conversion d'erreurs | `core` | être référencé par un autre crate |
| `tui` | Binaire ratatui | `core`, `design-tokens` | dépendre de `bridge` |
| `web-public` | Site Leptos | `core` ou `application` | dépendre de `bridge` |
| `design-tokens` | Palette générée pour ratatui | rien | être éditée à la main |
| `test-support` | Adaptateurs en mémoire, `World` cucumber partagé | `application`, `domain` | être une dépendance hors `dev-dependencies` |

**Une seule bibliothèque native par plateforme.** Un retour d'expérience UniFFI recommande un crate façade unique pour éviter la duplication des symboles du runtime Rust à l'édition de liens `[S]`. Ici `bridge` est le seul `cdylib`/`staticlib` `[J]`.

### 5.2 Règles du domaine (DDD)

- Un **contexte borné** = un module de premier niveau dans `domain` et dans `application` (ou un couple de crates quand il grossit). Le vocabulaire des modules, des types et des étapes Gherkin est celui du langage ubiquitaire, en accord avec la langue des `.feature`.
- Les objets-valeur valident à la construction et sont immuables. Les constructeurs renvoient `Result`.
- Les agrégats exposent des méthodes de comportement, pas des champs mutables.
- Les erreurs métier sont des `enum` typés par contexte. Pas de `String`, pas d'`anyhow` dans `domain` ni `application`.
- Le temps et les identifiants passent par des ports (`Clock`, `IdGenerator`) pour rester déterministes en test.

### 5.3 Ports et cas d'usage

Illustratif, non compilé :

```rust
// application/src/notes/ports.rs
pub trait NoteRepository {
    async fn load(&self, id: NoteId) -> Result<Option<Note>, StorageError>;
    async fn save(&self, note: &Note, mutation: PendingMutation) -> Result<(), StorageError>;
}

pub trait NoteReadModel {
    /// Flux d'un modèle de lecture prêt à afficher (soupape a+, ADR-02).
    fn watch_list(&self, query: NoteListQuery) -> impl Stream<Item = NoteListView>;
}

// application/src/notes/create_note.rs
pub struct CreateNote<R, C, I> { repo: R, clock: C, ids: I }

impl<R: NoteRepository, C: Clock, I: IdGenerator> CreateNote<R, C, I> {
    pub async fn execute(&self, cmd: CreateNoteCommand) -> Result<NoteId, CreateNoteError> {
        let note = Note::create(self.ids.next(), cmd.title, self.clock.now())?;
        self.repo.save(&note, PendingMutation::created(&note)).await?;
        Ok(note.id())
    }
}
```

Règles :
- `application` DEVRAIT rester **agnostique du runtime async** (pas de `tokio` dans ses dépendances) : sur le web, l'async Rust tourne sur le fil du Worker ou le fil principal `[V]`. Le choix du runtime appartient à `core` et aux adaptateurs `[J]`.
- Une commande = une transaction = une mutation en boîte d'envoi.
- Les requêtes qui alimentent l'UI renvoient des **flux** de modèles de lecture, recalculés quand les tables sous-jacentes changent.

### 5.4 API du pont

Illustratif, non compilé ; valide les attributs FRB contre la doc de la version épinglée :

```rust
// bridge/src/api/notes.rs
pub async fn create_note(title: String) -> Result<String, BridgeError> { /* délègue à core */ }

pub fn watch_notes(query: NoteListQueryDto, sink: StreamSink<NoteListViewDto>)
    -> Result<(), BridgeError> { /* s'abonne au modèle de lecture */ }
```

- DTO : structures plates, types primitifs, `Vec`, `Option`, enums simples. Identifiants en `String`.
- `BridgeError` : enum fermée, codes stables, jamais de message destiné à l'utilisateur final (la traduction se fait en Dart).
- Les flux FRB vont de Rust vers Dart uniquement, et la doc les signale potentiellement instables sur le web `[V]`. Chaque flux DOIT avoir un test d'intégration web, et le cœur DEVRAIT offrir une requête ponctuelle équivalente (`get_notes`) utilisable en repli.
- Lints du crate : `#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]`.

### 5.5 Configuration du workspace

Illustratif :

```toml
# Cargo.toml (racine)
[workspace]
resolver = "2"
members = ["crates/*", "crates/adapters/*"]

[workspace.lints.rust]
unsafe_code = "deny"            # `forbid` ne peut pas être levé ; autoriser uniquement sur le module généré de bridge

[workspace.lints.clippy]
all = { level = "deny", priority = -1 }
pedantic = { level = "warn", priority = -1 }
```

```toml
# rust-toolchain.toml — exigé par le backend native-assets [P]
[toolchain]
channel = "<version stable concrète>"
components = ["rustfmt", "clippy"]
targets = [ "aarch64-linux-android", "armv7-linux-androideabi", "x86_64-linux-android",
            "aarch64-apple-ios", "aarch64-apple-ios-sim",
            "aarch64-apple-darwin", "x86_64-apple-darwin",
            "x86_64-pc-windows-msvc", "x86_64-unknown-linux-gnu",
            "wasm32-unknown-unknown" ]
```

Chaque crate porte `[lints] workspace = true`. Dans `bridge`, le module généré par FRB reçoit un `#![allow(unsafe_code, clippy::unwrap_used)]` ciblé ; le code écrit à la main reste soumis aux interdictions.

Toolchain du build web : `rust-toolchain.toml` reste sur la version stable ; le job `wasm` passe une nightly datée par `--wasm-pack-rustup-toolchain` `[P]`.

Intégration de FRB à un workspace existant `[P]` :
1. `flutter_rust_bridge_codegen integrate --rust-crate-dir <chemin>` depuis le paquet Flutter concerné (le codegen se lance depuis le sous-paquet dans un pub workspace `[P]`).
2. Supprimer le `Cargo.lock` généré dans le crate.
3. Ajouter le crate aux `members` de la racine (étape manuelle).
4. Le répertoire `target` partagé peut empêcher FRB de trouver la bibliothèque compilée ; la doc renvoie à l'issue #2460 pour le contournement.

---

## 6. Côté Flutter

### 6.1 Paquets

| Paquet | Contenu |
|---|---|
| `packages/core_bridge` | Dart généré par FRB, `hook/build.dart`, initialisation de la bibliothèque. Aucune logique. |
| `packages/design_system` | Tokens générés, atomes, molécules, organismes. Aucun accès au cœur. |
| `packages/widgetbook` | Catalogue Widgetbook du design system et des widgets de features. |
| `packages/bdd_steps` | Définitions d'étapes partagées (`externalSteps`). |
| `packages/app` | L'application, en Feature-Sliced Design. |

Outillage : pub workspaces + Melos 8.10.1 `[P]`. Melos 7 a supprimé `melos.yaml` au profit du `pubspec.yaml` racine `[S]` ; Melos 8.0.0 est cassant sur la configuration `exec` `[P]`.

### 6.2 Feature-Sliced Design + Clean Architecture

Couches FSD dans `packages/app/lib/` (dépendances vers le bas uniquement) :

```
app/        composition : get_it/injectable, routeur, thème, démarrage du cœur
pages/      écrans, assemblage de widgets et de features
widgets/    blocs d'interface composites réutilisés entre pages
features/   interactions utilisateur (un cas d'usage visible = une tranche)
entities/   représentations côté UI des concepts métier
shared/     utilitaires, accès au pont, extensions
```

Segments Clean Architecture **dans chaque tranche** :

```
features/create_note/
  presentation/   bloc/ (events, states sealed), widgets/
  domain/         interfaces de passerelle (NoteGateway), types d'échec sealed
  data/           implémentation de la passerelle au-dessus de core_bridge, mappers DTO → modèles UI
  create_note.dart   API publique de la tranche (seul fichier importable de l'extérieur)
```

Conséquences du cœur Rust sur la Clean Architecture Dart `[J]` :
- Le segment `domain` Dart ne contient plus de règle métier. Il déclare les **passerelles** (interfaces) dont la présentation dépend, et les types d'échec.
- Le segment `data` est l'unique endroit qui importe `core_bridge`. Il convertit les DTO en modèles d'interface et `BridgeError` en échecs `sealed`.
- Les BLoC dépendent des passerelles, jamais de `core_bridge`.

Aucun exemple Flutter officiel de FSD ni linter FSD pour Dart n'a été trouvé `[P]` (le linter officiel Steiger est JS/TS `[S]`). Pour faire respecter les couches : `import_rules` 0.0.8, fondé sur `analysis_server_plugin`, jeune `[P]` ; `dart_boundaries` 0.3.1 repose sur `custom_lint`, que sa propre page dit archivé `[P]`. À défaut, écrire un contrôle d'imports dans `tool/` et l'exécuter en CI `[J]`.

### 6.3 BLoC

Règles :
- `Bloc` uniquement, jamais `Cubit`. Le linter officiel `bloc_lint` 0.4.3 fournit la règle `prefer_bloc` `[P]` ; sa sémantique exacte et sa sévérité configurable n'ont pas été lues `[NV]` : vérifie-les avant de t'y fier, sinon ajoute un contrôle par recherche de `extends Cubit`.
- Events et states en hiérarchies `sealed`, classes finales immuables.
- Un BLoC s'abonne aux flux du cœur dans son constructeur via un event interne, et utilise `emit.forEach` / `emit.onEach` pour convertir le flux en états.
- Versions : `bloc` 9.2.1, `flutter_bloc` 9.1.1 `[P]`.

Illustratif, non compilé :

```dart
sealed class NoteListEvent { const NoteListEvent(); }
final class NoteListSubscriptionRequested extends NoteListEvent { const NoteListSubscriptionRequested(); }
final class NoteListFilterChanged extends NoteListEvent {
  const NoteListFilterChanged(this.filter);
  final NoteFilter filter;
}

sealed class NoteListState { const NoteListState(); }
final class NoteListLoading extends NoteListState { const NoteListLoading(); }
final class NoteListReady extends NoteListState {
  const NoteListReady(this.view);
  final NoteListViewModel view;   // enveloppe le modèle de lecture calculé par Rust
}
final class NoteListFailure extends NoteListState {
  const NoteListFailure(this.failure);
  final NoteFailure failure;      // sealed
}
```

### 6.4 Injection de dépendances

- `injectable` 3.0.0 + `get_it` 9.3.0 `[P]`.
- Environnements : `prod` (passerelles au-dessus du cœur Rust) et `test` (faux en mémoire, en Dart).
- Le cœur Rust est initialisé une fois, dans `app/`, avant `runApp`. Sa configuration (chemin de base, URL du serveur) est passée à l'initialisation ; **la composition des adaptateurs Rust se fait dans le crate `core`, pas dans get_it**.
- Sur le web sous Plan C (ADR-07), l'adaptateur de stockage Dart est enregistré dans get_it puis transmis au cœur à l'initialisation.

### 6.5 Contraintes web pour le code Dart

- En compilation Wasm, `dart:html`, `dart:js`, `package:js` ne sont pas supportés ; utiliser `package:web` et `dart:js_interop` `[P]`. Applique cette règle dès maintenant, même en dart2js, pour ne pas fermer la porte au Wasm.
- Tester l'app web dans les deux modes dès que le Wasm est activé, puisque Safari et Firefox reçoivent le repli JS `[V]`.

---

## 7. TUI

### 7.1 Structure

```
crates/tui/src/
  main.rs        initialisation du terminal, boucle d'événements, restauration à la sortie
  app.rs         Model, Message, update()
  screens/       un module par écran : view(model, frame)
  widgets/       atomes et molécules ratatui, alimentés par design-tokens
  effects.rs     appels au cœur, abonnements aux flux
```

- `update()` est pure : `(Model, Message) -> (Model, Vec<Effect>)`. Les effets appellent le cœur et renvoient des messages.
- La TUI consomme les mêmes modèles de lecture que Flutter.

### 7.2 Tests

| Niveau | Outil | Limite |
|---|---|---|
| Logique | tests unitaires de `update()` | — |
| Rendu | `TestBackend` + `insta` 1.49.0, taille de terminal fixe `[P]` | **Les couleurs et styles ne sont pas capturés** (ratatui #1402, toujours ouverte) `[P]` |
| Comportement | cucumber-rs pilotant `Model` + `TestBackend` | — |
| Bout en bout | binaire réel dans un pseudo-terminal ; la doc ratatui cite `termlens` (0.11.9, crate jeune) `[P]` ; alternatives `expectrl` 0.9.0, `insta-cmd` 0.7.0 `[P]` | `TestBackend` ne couvre ni boucle d'événements, ni clavier, ni code de sortie `[P]` |

Tester les styles par assertion directe sur le `Buffer` pour les quelques cas où la couleur porte une information `[J]`.

---

## 8. Stratégie de test : une spécification, trois exécutants

### 8.1 Emplacement et synchronisation

- **Source de vérité : `features/` à la racine.** Organisation par contexte borné : `features/<contexte>/<capacité>.feature`.
- **cucumber-rs** (0.23.0 `[P]`) lit ce dossier directement : le chemin des features est un argument du lanceur, non lié au crate `[P]`.
- **bdd_widget_test** (3.1.0 `[P]`) est un générateur `build_runner` : le `.dart` est produit à côté du `.feature`, dans le paquet racine ; les sources de build sont limitées aux fichiers du paquet `[P]`. Il ne peut donc pas lire `features/` à la racine.
- Liens symboliques : **écartés** `[E]` (S4, Linux). `build_runner` suit le lien mais écrit ses fichiers générés dans le dossier source, et `flutter test` ne découvre pas les tests sous un répertoire lié.
- **Décision**, confirmée par l'essai S4 `[E]` : un outil `tool/sync_features` copie vers `packages/app/test/features/` les `.feature` qui contiennent `@gui` (granularité : le fichier). Il supprime les copies orphelines. Les copies sont versionnées ; la CI exécute `sync_features --check`, qui échoue sur un fichier modifié, manquant ou orphelin. Modèle : `spikes/s4-gherkin/app/tool/sync_features.dart`.
- **Un fichier `.feature` ne mélange pas les cibles** `[E]` : un scénario sans `@gui` placé dans un fichier copié est quand même généré côté Dart, avec des étapes en `UnimplementedError`. Les scénarios réservés au cœur vont donc dans des fichiers sans aucun `@gui`. Les `_test.dart` générés ne sont jamais édités ; les fichiers d'étapes sont créés une fois puis maintenus à la main `[P]`.

Exemples réels de spécification Gherkin partagée entre implémentations : `typedb/typedb-behaviour` (dépôt de `.feature` consommé par plusieurs langages, étiquettes d'exclusion par implémentation), `open-feature/spec`, `cucumber/compatibility-kit` `[P]`.

### 8.2 Étiquettes

| Étiquette | Sens |
|---|---|
| `@core` | Exécuté contre l'API `application` avec adaptateurs en mémoire. Tout scénario métier DEVRAIT la porter. |
| `@gui` | Exécuté en test de widgets Flutter. |
| `@tui` | Exécuté contre le modèle TUI. |
| `@web-public` | Exécuté contre le site public. |
| `@e2e` | Exécuté de bout en bout avec le vrai cœur (voir §8.5). |
| `@wip` | En cours ; toléré en local, refusé sur la branche principale. |

Une TUI en parité porte `@tui` partout ; une TUI en sous-ensemble ne le porte que sur les scénarios retenus. La CI publie la **matrice scénario × cible** : c'est la mesure objective de la parité `[J]`.

Filtrage `[E]` (S4) :
- Dart : **par inclusion**, `flutter test --tags gui`. `--exclude-tags core` est faux dès qu'un scénario porte `@core @gui` : tout est exclu. Déclarer les étiquettes dans `dart_test.yaml`, sinon Flutter avertit.
- cucumber-rs : filtre sur les étiquettes du scénario, de sa règle et de la feature, avec `filter_run_and_exit`.

### 8.3 Dialecte Gherkin commun

Les deux exécutants ne lisent pas le même Gherkin. Règles pour les fichiers partagés :

| Règle | Raison |
|---|---|
| Un seul bloc `Feature:` par fichier | exigence de `bdd_widget_test` `[P]` |
| Pas de doc strings | parsées mais ignorées par `bdd_widget_test` `[P]` |
| Pas d'étiquette contenant un espace (`@testMethodName: …`) | invalide en Gherkin standard ; mettre ces options dans `build.yaml` `[P]` |
| Pas du mot-clé `After` | hors standard, propre à `bdd_widget_test` `[P]` |
| Paramètres écrits en clair : `"texte"` et nombres nus | par défaut `bdd_widget_test` attend la valeur entre accolades dans le `.feature` (`{42}`), ce qui est incompatible avec cucumber-rs où `{int}` figure dans la définition d'étape. Activer `literalParameters: true` (3.0.5+) pour converger `[P]`. Établi par S4 `[E]` : entiers et flottants deviennent des paramètres typés `num` côté Dart ; un mot isolé n'est **pas** un paramètre, l'écrire entre guillemets ; la chaîne vide `""` passe des deux côtés. |
| `# language: fr` autorisé | supporté des deux côtés `[P]` |
| `Rule`, `Background`, `Scenario Outline`, tables de données autorisés | supportés des deux côtés depuis `bdd_widget_test` 3.0.0 (parseur officiel `cucumber_gherkin`) `[P]` ; confirmé en français par S4 `[E]`, sauf le `Contexte` propre à une règle, non essayé `[NV]` |
| Cellules de table de données : chaînes entre guillemets (`\| "Courses" \| 1 \|`), en-têtes compris | `bdd_widget_test` insère chaque cellule comme du code Dart ; côté Rust, retirer les guillemets. Les tables `Exemples` restent du texte brut `[E]` |
| `<paramètre>` de plan de scénario : entre guillemets pour une chaîne | hors guillemets, l'étape générée est typée `dynamic`, à retyper à la main `[E]` |
| Pas de forme optionnelle ni de pluriel (`note(s)`) | n'existe pas côté Dart : deux phrases donnent deux étapes distinctes `[E]` |
| Étapes déclaratives, au niveau métier | la même phrase doit avoir un sens pour le cœur, un écran et un terminal `[J]` |

Les définitions d'étapes sont écrites séparément par langage.

**Défaut connu du générateur** `[E]` : la première génération des fichiers d'étapes a corrompu un fichier partagé par plusieurs scénarios deux fois sur trois (octets résiduels en fin de fichier), cause non diagnostiquée. Après toute création de fichiers d'étapes, lance `dart analyze` et relis les fichiers créés avant de continuer.

Alternative Dart à connaître : `gherkart` 0.2.1 lit des `.feature` à l'exécution depuis un chemin arbitraire, ce qui supprimerait la copie, mais il est très jeune (340 téléchargements) `[P]`. Les paquets `gherkin` et `flutter_gherkin` sont à l'arrêt de fait `[P]`.

### 8.4 Double boucle

```
1. Écrire le scénario dans features/ (@core, puis @gui/@tui selon le périmètre).
2. Boucle extérieure ROUGE : cucumber-rs @core échoue sur une étape non implémentée
   (fail_on_skipped activé, lanceur terminé par run_and_exit ou filter_run_and_exit [E]).
3. Boucle intérieure dans domain/ puis application/ :
     test unitaire rouge → code minimal vert → refactor → linters vierges.   (répéter)
4. Boucle extérieure VERTE côté cœur.
5. Pour chaque coque visée, même cycle :
     GUI : sync_features → build_runner → scénario bdd_widget_test rouge
           → bloc_test (rouge/vert/refactor) → tests de widgets → scénario vert.
     TUI : scénario cucumber-rs @tui rouge → tests de update() → snapshots insta → vert.
6. Si @e2e : scénario d'intégration avec le vrai cœur.
7. Gates locales complètes (§11.2) avant commit.
```

Le lanceur cucumber-rs DOIT se terminer par `run_and_exit` ou `filter_run_and_exit` `[E]` : avec `.run()` ou `.filter_run()`, le processus rend le code 0 même quand des scénarios échouent, et la CI ne voit rien.

Règles pour l'agent :
- NE JAMAIS écrire de code de production sans test rouge préalable de la bonne boucle. Périmètre : tout fichier non généré sous `crates/*/src` ou `packages/*/lib`. Exclus : squelette de l'étape 1 du §15, code généré, configuration de build et de CI.
- NE JAMAIS laisser `@wip`, `skip`, `todo!()`, `unimplemented!()` ou une étape vide dans un travail déclaré terminé.
- Les linters sont exécutés **à chaque étape verte**, pas seulement à la fin.

### 8.5 Contre quoi tournent les tests Flutter ?

| Niveau | Dépendances | Vitesse |
|---|---|---|
| `bloc_test` | passerelles factices | très rapide |
| Tests de widgets et scénarios `@gui` | passerelles factices en Dart (environnement `test` d'injectable) | rapide |
| Scénarios `@e2e` (`integration_test`, `patrol` 4.11.0 sur mobile `[P]`) | **vrai cœur Rust** | lent |

Risque : un faux Dart qui diverge du cœur. Parades `[J]` : (1) le comportement est spécifié une fois et prouvé au niveau `@core` ; (2) les faux ne contiennent aucune branche métier : une collection en mémoire, renvoyée telle quelle ; (3) un sous-ensemble `@e2e` traverse le pont à chaque PR sur Linux, et sur chaque cible dans le job `release`.

Pour faire tourner `integration_test` depuis des `.feature`, ajouter `integration_test/**` aux `sources` de `build.yaml` `[P]`.

### 8.6 Pyramide et autres tests

| Cible | Unitaires | Comportement | Visuel | Bout en bout |
|---|---|---|---|---|
| Cœur | `cargo nextest` 0.9.152 `[P]` | cucumber-rs `@core` | — | tests d'intégration des adaptateurs (SQLite réel, serveur factice) |
| Flutter | `bloc_test`, tests de widgets | `bdd_widget_test` `@gui` | goldens : `alchemist` 0.14.1 (actif) ; `golden_toolkit` est abandonné `[P]` | `integration_test`, `patrol` |
| TUI | `update()` | cucumber-rs `@tui` | `insta` | pseudo-terminal |
| Web applicatif | — | — | — | `flutter drive` avec en-têtes (`--web-header`, Flutter ≥ 3.17 `[P]`) |
| Web public | tests Rust | cucumber-rs `@web-public` | — | navigateur piloté (Playwright) `[J]` |

Rapports cucumber-rs : JUnit XML et Cucumber JSON par features Cargo `[P]`. Les scénarios s'exécutent en parallèle par défaut (64), `@serial` pour isoler ; un `World` neuf par scénario `[P]`.

---

## 9. Design system et tokens

### 9.1 Chaîne

```
design/tokens/*.tokens.json   (DTCG 2025.10, source unique)
        │  Style Dictionary 5.8.0
        ├─► packages/design_system/lib/src/tokens/   (Dart : ThemeExtension)
        ├─► crates/web-public/style/tokens.css       (variables CSS)
        └─► crates/design-tokens/src/generated.rs    (palette ratatui)
```

Faits :
- Le format DTCG a atteint sa première version stable, 2025.10, le 2025-10-28 ; l'annonce cite Flutter parmi les cibles `[P]`.
- Style Dictionary 5.8.0 (Node ≥ 22) : support de 2025.10 **incrémental**, la doc dit qu'il n'est pas complet `[P]`. Format Flutter intégré, limité à des constantes plates `[S]` ; **aucun format Rust intégré** `[S]`.
- Terrazzo 2.10.0 annonce le support complet de 2025.10 mais n'a ni plugin Dart ni plugin Rust `[P]`.
- `dtcg_theme` 0.7.1 : génère un `ThemeExtension` par mode, mode `--check` pour la CI ; paquet très jeune `[P]`. Alternative : `design_builder` 1.0.1 `[S]`.
- Aucun générateur DTCG → ratatui trouvé `[NV]`.

Établi par l'essai S6 `[E]` (Style Dictionary 5.8.0, Node 26.13.1, ratatui 0.30.2) :
- Un format personnalisé enregistré par `hooks.formats` tient en une fonction d'environ 25 lignes et suffit à émettre la palette Rust ; le repli par générateur maison est inutile.
- Fonctionnent : couleurs en objet `srgb`, alias `{a.b}`, `$extensions` transmis au format, plusieurs formats sur une même source. `oklch` et `display-p3` sont convertis en hex sRGB avec écrêtage.
- **Ne fonctionnent pas : `$extends` et `$ref`**, non résolus. Les dimensions en objet passent sans transformation. Un `components` contenant `none` est converti en silence.
- DTCG 2025.10 n'a pas de modes natifs. Convention retenue : groupes `mode.<nom>.*` dans le fichier source.
- Sans transformation de nom, les jetons de même nom entre modes entrent en collision.
- Non essayés `[NV]` : plusieurs fichiers de jetons, jetons composites (`shadow`, `typography`, `border`), cycle d'alias.

**Décision**, confirmée pour la cible Rust `[E]` : Style Dictionary avec formats personnalisés (Rust `ratatui::style::Color`, Dart `ThemeExtension`, CSS). Règles :
- Les fichiers de jetons NE DOIVENT PAS utiliser `$extends` ni `$ref`.
- La plateforme DOIT déclarer `name/kebab` (ou équivalent) et `log.warnings: 'error'`.
- Le format Rust DOIT lever une erreur sur une couleur avec transparence.
- Contrôle de dérive : régénérer dans un dossier temporaire et comparer octet à octet aux fichiers versionnés (modèle : `spikes/s6-jetons/check.mjs`), ce qui ne salit pas l'arbre de travail.
- **Reste à faire** : l'essai n'a produit côté Dart que des constantes, non compilées. La génération du `ThemeExtension` et sa compilation ne sont pas établies `[NV]`.

### 9.2 Tokens dans le terminal

Implémenté et testé par S6 `[E]` :
- Couleur sRGB → `Color::Rgb` en truecolor ; repli sur l'index xterm-256 le plus proche, puis sur les 16 couleurs ANSI. Ce dernier repli est approximatif par nature : il dépend de la palette du terminal.
- Précédence de détection : `NO_COLOR` non vide, puis `COLORTERM` (`truecolor`, `24bit`), puis `TERM` (`*256color*`), sinon 16 couleurs. La détection reçoit l'environnement en paramètre pour rester testable.
- Sous `NO_COLOR`, conserver les modificateurs et ajouter l'inversion pour que les atomes colorés restent lisibles.
- `Modifier` est `const` : la palette peut être une constante.

`[J]` :
- Espacement, rayon, typographie n'ont pas d'équivalent : les tokens sémantiques (`surface`, `accent`, `danger`, `emphasis`) se traduisent en couleurs et en modificateurs (gras, inversé, souligné).
- L'atomic design s'applique à la TUI au niveau des atomes et molécules ratatui dans `crates/tui/src/widgets/`.

### 9.3 Atomic design et Widgetbook

- `widgetbook` 3.25.0 stable ; 4.0.0-beta.15 en préversion, contenu non vérifié `[NV]`. Rester en 3.x `[J]`.
- Thèmes : `MaterialThemeAddon`, `ThemeAddon<T>` ; placer `ViewportAddon` avant `ThemeAddon` `[P]`. Les `ThemeExtension` passent par `ThemeData.extensions` (inférence, `[NV]`).
- `packages/design_system/lib/src/` : `tokens/`, `atoms/`, `molecules/`, `organisms/`. Les gabarits et pages vivent dans l'app (`widgets/`, `pages/`).
- Chaque atome, molécule et organisme DOIT avoir un cas d'usage Widgetbook et un golden.
- Un composant du design system NE DOIT PAS dépendre d'un BLoC ni du cœur.

---

## 10. Arborescence du monorepo

```
/
├─ Cargo.toml                 workspace
├─ rust-toolchain.toml
├─ pubspec.yaml               pub workspace + configuration Melos
├─ features/                  Gherkin, source unique
├─ design/tokens/             DTCG
├─ crates/
│  ├─ domain/
│  ├─ application/
│  ├─ sync-protocol/
│  ├─ adapters/
│  │  ├─ storage-sqlite/
│  │  ├─ sync-http/
│  │  └─ platform-native/
│  ├─ core/
│  ├─ bridge/
│  ├─ tui/
│  ├─ web-public/
│  ├─ design-tokens/
│  └─ test-support/
├─ packages/
│  ├─ core_bridge/
│  ├─ design_system/
│  ├─ widgetbook/
│  ├─ bdd_steps/
│  └─ app/
├─ tool/                      sync_features, génération des tokens, contrôle d'imports
├─ docs/architecture/         ce dossier, ADR
└─ .github/workflows/
```

Tous les paquets du monorepo qui utilisent FRB doivent partager la même version `[S]`.

---

## 11. Qualité et intégration continue

### 11.1 Linters

| Outil | Version | Usage |
|---|---|---|
| `rustfmt`, `clippy` | toolchain épinglée | `cargo fmt --check` ; `cargo clippy --workspace --all-targets -- -D warnings` |
| `cargo-deny` | 0.20.2 `[P]` | licences, avis de sécurité, doublons |
| `very_good_analysis` | 11.0.0 (SDK ^3.13) `[P]` | `dart analyze --fatal-infos` |
| `bloc_lint` | 0.4.3 `[P]` | règle `prefer_bloc` |
| `dart format` | SDK | `--output=none --set-exit-if-changed` |
| contrôle d'imports | `import_rules` 0.0.8 ou script | couches FSD, isolement de `core_bridge` |
| contrôle des dépendances Rust | script `cargo metadata` dans `tool/`, ou `[bans]` de cargo-deny | `bridge` référencé par aucun crate ; pas de `tokio` dans `application` ; `tui` et `web-public` sans `bridge` ; `test-support` en `dev-dependencies` seulement `[J]` |

Le code généré (FRB, `build_runner`, tokens) est exclu de l'analyse et du formatage, et jamais édité à la main.

### 11.2 Gates locales

À exécuter avant chaque commit. Fournis-les sous forme d'une commande unique (`just check` ou script Melos).

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run --workspace
cargo test --workspace --test bdd          # cucumber-rs, harness = false
cargo deny check
dart format --output=none --set-exit-if-changed .
dart analyze --fatal-infos
bloc lint .
dart run tool/sync_features.dart --check
flutter test
```

`insta` n'écrit pas de snapshot quand il détecte une CI ; un snapshot manquant ou différent fait échouer le test `[J]` (comportement à confirmer sur la version épinglée).

### 11.3 Contrôles de dérive

Chaque générateur est rejoué en CI, puis `git diff --exit-code` :
- `flutter_rust_bridge_codegen generate`
- `dart run build_runner build --delete-conflicting-outputs`
- génération des tokens
- `sync_features --check`

### 11.4 Pipeline GitHub Actions

| Job | Runner | Contenu |
|---|---|---|
| `rust` | `ubuntu-latest` | fmt, clippy, nextest, cucumber `@core` et `@tui`, `cargo-llvm-cov` 0.9.1, cargo-deny |
| `wasm` | `ubuntu-latest` | cœur compilé pour `wasm32-unknown-unknown` ; `build-web` avec nightly |
| `flutter` | `ubuntu-latest` | format, analyze, bloc lint, dérive, tests unitaires, de widgets, BDD `@gui`, goldens |
| `e2e-linux` | `ubuntu-latest` | `integration_test` desktop Linux avec le vrai cœur |
| `build-desktop` | `ubuntu-latest`, `windows-latest`, `macos-latest` | build Flutter desktop + TUI |
| `build-android` | `ubuntu-latest` | NDK, build Android |
| `build-ios` | `macos-latest` | build iOS sans signature |
| `web-public` | `ubuntu-latest` | build et tests du site |
| `matrice` | `ubuntu-latest` | agrège les rapports JUnit des exécutants et publie le tableau scénario × cible |
| `release` | selon cible | scénarios `@e2e` sur la cible, puis empaquetage et signature, sur étiquette |

Runners hébergés disponibles `[P]` : Linux x64 (`ubuntu-latest` = 24.04) et arm64 (`ubuntu-24.04-arm`) ; Windows x64 (`windows-latest` = 2025) et arm64 (`windows-11-arm`) ; macOS arm64 (`macos-latest` = macOS 26) et Intel (`macos-15-intel`, `macos-26-intel`).

Actions `[P]` : `subosito/flutter-action` v2.23.0, `dtolnay/rust-toolchain`, `Swatinem/rust-cache` v2.9.3, `Mozilla-Actions/sccache-action` v0.0.11, `taiki-e/install-action` v2.90.5, `nttld/setup-ndk` v1.6.0, `bluefireteam/melos-action` v3.10.0. Épingler par SHA, comme le fait RustDesk `[P]`.

Références à lire, sans copier : la CI de FRB (`flutter-actions/setup-flutter`, émulateur Android, runners macOS Intel et arm) et celle de RustDesk (matrice complète, cargo-ndk, workflow séparé pour le codegen) `[P]`. Celle d'AppFlowy est datée `[P]`. La page CI de la doc FRB porte le bandeau V1 : obsolète `[P]`.

Compilation croisée : `cargo-ndk` 4.1.2 pour Android ; le crate pour XCFramework s'appelle `xcframework` 0.2.1 (**`cargo-xcframework` n'existe pas** sur crates.io) ; `cargo-zigbuild` 0.24.1 pour la TUI ; `cross` n'a pas publié depuis 2023 (installer depuis git ou préférer zigbuild) `[P]`. Avec le backend native-assets, Android et iOS sont compilés par le hook : `cargo-ndk` et `xcframework` ne servent qu'en repli `[J]`.

### 11.5 Gates de fusion

Une PR ne fusionne que si : tous les jobs sont verts ; aucun scénario `@wip` ; aucun avertissement de linter ; couverture de `domain` et `application` (cargo-llvm-cov) au moins égale à celle de la branche principale, seuil plancher à fixer par l'utilisateur ; contrôles de dérive vides ; matrice scénario × cible publiée.

---

## 12. Empaquetage et distribution

| Cible | Outils | Remarques |
|---|---|---|
| Windows | `msix` 3.18.0 ; Fastforge 0.6.14 (nouveau nom de Flutter Distributor) `[P]` | Liste exacte des formats Fastforge non confirmée `[NV]` |
| macOS | Fastforge, signature et notarisation Apple | Procédure de notarisation non recherchée `[NV]` |
| Linux | Fastforge (AppImage, deb, rpm) ; Flatpak et Snap | Support Flatpak/Snap par Fastforge `[NV]` |
| iOS, Android | `fastlane` 2.241.2 `[P]` | — |
| Web applicatif | hébergeur statique avec en-têtes (§3.5, §3.6) | réécriture vers `index.html` |
| Web public | hébergement du serveur Leptos, ou SSG | — |
| TUI | `dist` (ex cargo-dist) v0.34.0, actif `[P]` ; `cargo-binstall` 1.27.0 `[P]` | Liste des installeurs de `dist` non relue `[NV]` ; winget et Homebrew non recherchés `[NV]` |

---

## 13. Alternatives écartées

| Alternative | État `[P]` | Pourquoi non |
|---|---|---|
| Tauri 2.12.5 (3.0 alpha) | Desktop mûr ; sur mobile l'équipe reconnaît avoir trop promis. Un retour d'expérience a abandonné Tauri pour sa coque mobile en gardant le cœur Rust `[S]`. | UI web : les compétences Flutter/BLoC sont perdues |
| Dioxus 0.7.10 | Mobile par WebView ; rendu natif « experimental » | Pré-1.0 ; candidat pour le web public seulement |
| Slint 1.18.1 | Android et iOS annoncés (statut iOS actuel `[NV]`) ; web = démonstrations ; triple licence | Web insuffisant |
| egui 0.37.0 | Maturité mobile `[NV]` | Outils internes |
| Compose Multiplatform 1.12.2 | Android, iOS, desktop stables ; web en bêta | Seul concurrent mûr, mais impose Kotlin et ne résout ni la TUI ni le cœur Rust |
| React Native + Rust | `uniffi-bindgen-react-native` 0.31.0-7 | Pertinent pour une équipe React Native |
| Makepad, GPUI | Pré-1.0 ou sans release récente | Trop risqués hors desktop |
| Flutter seul, sans Rust | — | Viable sans TUI ; ici la TUI Rust et le site public imposent un cœur hors Dart `[J]` |

---

## 14. Risques et essais à mener avant de s'engager

Chaque essai est un prototype jetable, borné dans le temps, avec un critère de sortie. La numérotation regroupe les essais par sujet ; l'ordre d'exécution est celui du §15 : S1 et S4 d'abord, S6 avec le design system, S2, S3 et S5 avant tout engagement sur le web applicatif.

| # | Essai | Critère de réussite | Si échec |
|---|---|---|---|
| **S1** | FRB 2.13.0, backend `native-assets`, crate `bridge` dans le Cargo workspace, Flutter 3.50 | Une fonction async et un flux appelés depuis Flutter sur Linux, Android et un appareil Apple | Backend `cargokit` ; ou attendre FRB 2.14. La CI de FRB cible Flutter 3.47 : compatibilité 3.50 `[NV]` |
| **S2** | Cœur en WASM : `build-web`, `rusqlite` + VFS `sahpool`, écriture, rechargement de page, relecture ; second onglet | Donnée relue après rechargement sur Chrome, Firefox et Safari ; second onglet détecté proprement | Plan B ou C (ADR-07) |
| **S3** | En-têtes `COOP: same-origin` + `COEP: require-corp` sur l'hébergeur visé ; connexion par redirection | `crossOriginIsolated` vrai sur Safari ; connexion fonctionnelle | Mode sans en-têtes (`default_dart_async: false`), avec perte de réactivité |
| **S4** ✅ | **Réussi le 2026-11-18** (`adr/S4-gherkin-partage.md`), dialecte plus étroit que prévu : voir §8. Énoncé d'origine : un `.feature` partagé : cucumber-rs + `bdd_widget_test` avec `literalParameters`, `# language: fr`, une table, un `Scenario Outline`, une étiquette d'exclusion | Les deux exécutants passent sur le même fichier ; `sync_features --check` détecte un écart | Dialecte plus restreint, ou `gherkart` |
| **S5** | Flux FRB sur le web : 1 000 émissions, annulation d'abonnement, rechargement à chaud | Aucune perte, aucun blocage | Repli sur requêtes ponctuelles + notification de changement |
| **S6** ✅ | **Réussi le 2026-11-18** (`adr/S6-jetons-ratatui.md`) ; reste la sortie Dart `ThemeExtension`. Énoncé d'origine : format Style Dictionary personnalisé vers Rust | Palette ratatui générée et utilisée par un atome | Générateur `build.rs` maison |

Autres risques suivis :

| Risque | Parade |
|---|---|
| Mainteneur unique de FRB | Crate `bridge` mince et remplaçable (ADR-03) |
| `native_toolchain_rust` : éditeur tiers, mainteneur unique | Repli cargokit documenté |
| Toolchain nightly exigée par le build web | Épingler une date de nightly ; job CI dédié |
| Temps de build et taille des binaires | Cache (`rust-cache`, `sccache`) ; mesurer dès S1 ; aucun chiffre publié trouvé `[NV]` |
| Friction de régénération à chaque changement d'API Rust `[S]` | API à gros grain ; contrôle de dérive |
| Faux Dart divergents | §8.5 |
| `crossterm` peu actif | Surveiller ; ratatui supporte d'autres backends |
| Jaspr, Leptos, Dioxus pré-1.0 | Site public isolé derrière le cœur : remplaçable |

---

## 15. Feuille de route d'amorçage

Chaque étape se termine par les gates vertes et un commit.

1. **Squelette** : workspace Cargo, pub workspace, toolchains épinglées, linters, CI minimale (jobs `rust` et `flutter` vides mais verts).
2. **S1** puis **S4** : sans pont et sans spécification partagée, rien d'autre n'a de sens.
3. **Tranche verticale 1, cœur seul** : un scénario `@core`, un agrégat, un cas d'usage, adaptateur en mémoire.
4. **Persistance native** : adaptateur SQLite, tests d'intégration.
5. **TUI** : même scénario en `@tui`, premier snapshot.
6. **Flutter** : `core_bridge`, première tranche FSD, BLoC, scénario `@gui`, puis `@e2e` sur Linux.
7. **Design system** : tokens, trois atomes, Widgetbook, goldens ; **S6**.
8. **Synchronisation** : `sync-protocol`, boîte d'envoi, serveur factice, scénarios de conflit.
9. **Web applicatif** : **S2**, **S3**, **S5**, choix du plan de persistance, build et déploiement.
10. **Web public** : site Leptos, premier scénario `@web-public`.
11. **Distribution** : empaquetage par cible, release.

---

## 16. Consignes de travail pour l'agent

- **Avant d'écrire du code d'API** (FRB, cucumber-rs, ratatui, bloc, bdd_widget_test, Leptos) : consulte la documentation de la version épinglée. Les versions de ce document datent du 2026-11-16 et plusieurs bibliothèques évoluent vite.
- **Respecte le sens des dépendances** (§5.1, §6.2). Un import qui le viole est un défaut, même si tout compile.
- **Ne place aucune règle métier en Dart ni dans la TUI.** En cas de doute, applique le critère de l'ADR-02.
- **Ne modifie jamais un fichier généré.** Change la source et régénère.
- **Un `[NV]` n'est pas un fait.** Si ton travail en dépend, fais l'essai, consigne le résultat dans une ADR et mets ce document à jour.
- **Aucun achèvement factice** : pas d'étape vide, de `todo!()`, de test désactivé, de `@wip` dans un travail déclaré terminé.
- **Signale toute divergence** entre ce document et la réalité observée plutôt que de la contourner en silence.
- Les décisions marquées `[J]` appartiennent à l'utilisateur : propose, n'impose pas, quand tu veux t'en écarter.

---

## 17. Inventaire des versions (2026-11-16)

| Composant | Version | Preuve |
|---|---|---|
| Flutter stable / Dart | 3.50.0 / 3.14.0 (2026-11-11) | `[P]` |
| flutter_rust_bridge | 2.13.0 stable ; 2.14.0-beta.2 | `[V]` |
| hooks / native_toolchain_rust / flutter_rust_bridge_hooks | 2.2.0 / 1.1.1 / 2.13.0 | `[P]` |
| Melos | 8.10.1 | `[P]` |
| bloc / flutter_bloc / bloc_lint | 9.2.1 / 9.1.1 / 0.4.3 | `[P]` |
| injectable / get_it | 3.0.0 / 9.3.0 | `[P]` |
| bdd_widget_test | 3.1.0 (Dart ≥ 3.11) | `[P]` |
| very_good_analysis | 11.0.0 | `[P]` |
| widgetbook | 3.25.0 | `[P]` |
| alchemist / patrol | 0.14.1 / 4.11.0 | `[P]` |
| ratatui / crossterm | 0.30.2 / 0.29.0 | `[P]` |
| cucumber (cucumber-rs) / insta | 0.23.0 / 1.49.0 | `[P]` |
| cargo-nextest / cargo-deny / cargo-llvm-cov | 0.9.152 / 0.20.2 / 0.9.1 | `[P]` |
| rusqlite / sqlite-wasm-rs / sqlite-wasm-vfs | 0.43.0 / 0.6.3 / 0.3.0 | `[P]` |
| wasm-pack | 0.15.1 | `[P]` |
| Leptos / cargo-leptos | 0.8.23 / 0.3.11 | `[P]` |
| Jaspr | 0.23.5 | `[P]` |
| Style Dictionary / Node | 5.8.0 / 26.13.1 | `[E]` |
| uhlc | 0.9.0 | `[P]` |
| cargo-ndk / xcframework / cargo-zigbuild | 4.1.2 / 0.2.1 / 0.24.1 | `[P]` |
| Fastforge / msix / fastlane / dist | 0.6.14 / 3.18.0 / 2.241.2 / 0.34.0 | `[P]` |

---

## 18. Limites de l'étude

- La recherche principale a extrait 120 affirmations de 27 sources et en a vérifié 25 par vote contradictoire (25 confirmées, 0 réfutée). Elles couvrent FRB, Crux, UniFFI-Dart, PowerSync, Loro et Flutter Web. **Tout le reste** (TUI, monorepo, Gherkin partagé, tokens, CI, empaquetage, retours d'expérience, alternatives, persistance web) vient de cinq recherches complémentaires lues sur sources primaires par un seul agent chacune, sans contre-vérification.
- Les sources sont surtout des documentations d'éditeurs : fiables sur ce que fait le produit, auto-déclaratives sur sa maturité.
- À l'origine, aucun assemblage n'avait été compilé. Depuis, les essais S4 et S6 ont été menés (2026-11-18, Linux, prototypes jetables dans `spikes/`) ; leurs ADR font foi pour le détail. S1, S2, S3 et S5 restent à faire : le pont, le web applicatif et le stockage dans le navigateur ne sont toujours pas validés par l'expérience.
- Les arbitrages (a)/(b), Leptos/Jaspr/Astro et moteur de synchronisation maison sont des jugements, étayés mais non démontrés.

---

## 19. Sources

**Pont Rust↔Dart**
- https://docs.rs/crate/flutter_rust_bridge/latest
- https://crates.io/api/v1/crates/flutter_rust_bridge
- https://cjycode.com/flutter_rust_bridge/manual/miscellaneous/web-cross-origin
- https://fzyzcjy.github.io/flutter_rust_bridge/guides/how-to/cargo-workspaces
- https://github.com/fzyzcjy/flutter_rust_bridge (docs `manual/integrate/03-native-assets.md`, `04-cargokit.md`, `08-dart-workspace.md` ; issues #3067, #3326, #2829, #2856, #2560, #2460)
- https://cjycode.com/posts/rust-ui-flutter/
- https://redbadger.github.io/crux/latest_master/part-4/typegen.html
- https://pub.dev/packages/bdk_dart · https://github.com/bitcoindevkit/bdk-dart
- https://pub.dev/packages/native_toolchain_rust
- https://github.com/cunarist/rinf
- https://github.com/patmuk/flutter-UI_rust-BE-example
- https://dart.dev/tools/hooks · https://docs.flutter.dev/platform-integration/bind-native-code

**Web**
- https://docs.flutter.dev/platform-integration/web/faq
- https://docs.flutter.dev/platform-integration/web/wasm
- https://docs.flutter.dev/platform-integration/web/renderers
- https://docs.flutter.dev/ui/accessibility/web-accessibility
- https://docs.flutter.dev/ui/navigation/url-strategies
- https://opensource.googleblog.com/2026/04/jaspr-why-web-development-in-dart-might-just-be-a-good-idea.html
- https://pub.dev/packages/jaspr · https://docs.jaspr.site/dev/modes · https://docs.jaspr.site/going_further/flutter_embedding
- https://github.com/leptos-rs/leptos/releases · https://book.leptos.dev/islands.html
- https://dioxuslabs.com/learn/0.7/essentials/fullstack/static_site_generation
- https://docs.astro.build/en/guides/integrations-guide/
- https://github.com/wasm-bindgen/wasm-pack/releases
- https://caniuse.com/mdn-http_headers_cross-origin-embedder-policy_credentialless
- https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Cross-Origin-Opener-Policy
- https://developers.cloudflare.com/pages/configuration/headers/ · https://docs.netlify.com/manage/routing/headers/ · https://vercel.com/docs/project-configuration/vercel-json · https://firebase.google.com/docs/hosting/full-config
- https://startdebugging.net/2026/09/canvaskit-vs-skwasm-for-flutter-web-in-2026/ (secondaire)

**Persistance et synchronisation**
- https://crates.io/crates/rusqlite · https://github.com/Spxg/sqlite-wasm-rs · https://docs.rs/crate/sqlite-wasm-vfs/latest
- https://sqlite.org/wasm/doc/trunk/persistence.md
- https://docs.diesel.rs/main/diesel/sqlite/struct.SqliteConnection.html
- https://docs.powersync.com/client-sdks/reference/rust
- https://docs.powersync.com/client-sdks/frameworks/flutter-web-support
- https://docs.powersync.com/resources/supported-platforms
- https://github.com/powersync-ja/powersync-native/issues/6
- https://powersync.com/blog/electricsql-vs-powersync (éditeur)
- https://www.powersync.com/blog/why-cinapse-moved-away-from-crdts-for-sync (éditeur)
- https://electric.ax/docs/intro
- https://docs.turso.tech/sdk/rust/quickstart · https://docs.turso.tech/sync/usage · https://supabase.com/blog/supabase-is-acquiring-turso
- https://github.com/vlcn-io/cr-sqlite
- https://automerge.org/blog/automerge-3/ · https://pub.dev/packages/automerge
- https://loro.dev/docs · https://github.com/loro-dev/loro
- https://doc.replicache.dev/concepts/how-it-works
- https://www.figma.com/blog/how-figmas-multiplayer-technology-works/

**Tests, TUI, design system**
- https://pub.dev/packages/bdd_widget_test · https://pub.dev/packages/bdd_widget_test/changelog
- https://cucumber-rs.github.io/cucumber/main/print.html · https://cucumber-rs.github.io/cucumber/main/writing/tags.html · https://cucumber-rs.github.io/cucumber/main/writing/capturing.html
- https://github.com/dart-lang/build/blob/master/build_config/README.md · https://github.com/dart-lang/build/issues/1445
- https://github.com/typedb/typedb-behaviour · https://github.com/open-feature/spec · https://github.com/cucumber/compatibility-kit
- https://ratatui.rs/recipes/testing/snapshots/
- https://pub.dev/packages/widgetbook · https://docs.widgetbook.io/addons/theme-addon
- https://pub.dev/packages/alchemist · https://pub.dev/packages/golden_toolkit
- https://w3.org/community/design-tokens/2025/10/28/design-tokens-specification-reaches-first-stable-version/
- https://styledictionary.com/info/dtcg/ · https://terrazzo.app/docs/guides/resolvers/
- https://pub.dev/packages/dtcg_theme · https://pub.dev/packages/import_rules · https://pub.dev/packages/dart_boundaries

**Retours d'expérience et alternatives**
- https://appflowy.com/blog/tech-design-flutter-rust
- https://verygood.ventures/blog/rust-as-flutter-data-layer/
- https://github.com/rustdesk/rustdesk/blob/master/flutter/pubspec.yaml
- https://github.com/bitwarden/sdk-internal · https://contributing.bitwarden.com/architecture/sdk/
- https://rustfoundation.org/media/secure-app-development-with-rusts-memory-model/
- https://element.io/blog/element-x-ignition/
- https://github.com/signalapp/libsignal · https://github.com/mozilla/application-services
- https://content.red-badger.com/resources/rust-and-crux-in-the-enterprise-cross-platform-correct-by-design (éditeur)
- https://ac-opensource.github.io/blog/2026-04-07-one-rust-core-across-android-and-ios.html (secondaire)
- https://dev.to/yexiyue/one-rust-core-two-app-shells-tauri-react-native-in-swarmnote-2aam (secondaire)
- https://kotlinlang.org/docs/multiplatform/supported-platforms.html

**CI**
- https://docs.github.com/en/actions/reference/runners/github-hosted-runners
- https://invertase.docs.page/melos/guides/migrations
