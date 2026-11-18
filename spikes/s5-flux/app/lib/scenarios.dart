// Scénarios de l'essai S5, partagés entre le web (harnais Playwright) et le natif
// (integration_test). Chaque scénario renvoie un résultat au lieu de lever : on veut un
// tableau complet même quand un cas échoue.
import 'dart:async';
import 'dart:math';

import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart'
    show AnyhowException;

import 'src/rust/api/flux.dart';
import 'src/rust/api/repli.dart';

/// `true` quand l'app est construite pour le mode web sans en-têtes
/// (`default_dart_async: false`) : les fonctions Rust non `async` y deviennent synchrones
/// sur le fil principal, donc un émetteur bloquant (`pacedWorker`) gèlerait la page.
const bool sansEntetes = bool.fromEnvironment('S5_SANS_ENTETES');

class ScenarioResult {
  ScenarioResult(this.name, this.status, this.detail, this.ms);

  final String name;

  /// `ok`, `echec` ou `nt` (non testable dans ce mode).
  final String status;
  final String detail;
  final int ms;

  Map<String, Object> toJson() =>
      {'name': name, 'status': status, 'detail': detail, 'ms': ms};

  @override
  String toString() => '$name: $status ($ms ms) $detail';
}

class _Fail implements Exception {
  _Fail(this.message);
  final String message;
}

void _check(bool cond, String message) {
  if (!cond) throw _Fail(message);
}

/// Identifiants de flux propres à cette page (deux onglets n'ont pas les mêmes).
final _rng = Random();
int _nextId = (_rng.nextInt(1 << 20) + 1) * 1000;
int newId() => _nextId++;

const _timeout = Duration(seconds: 30);

/// Filtre (expression régulière) sur les noms de scénarios ; `null` : tous.
String? _only;

Future<void> _sleep(int ms) => Future<void>.delayed(Duration(milliseconds: ms));

/// Vérifie qu'une liste d'événements est complète, ordonnée et d'un seul flux.
void _checkSequence(List<FluxEvent> events, int id, int count,
    {String label = ''}) {
  final foreign = events.where((e) => e.streamId != id).length;
  _check(foreign == 0, '$label$foreign événements d\'un autre flux');
  if (events.length != count) {
    final seen = events.map((e) => e.seq).toSet();
    final missing = [for (var i = 0; i < count; i++) if (!seen.contains(i)) i];
    final info = streamInfo(streamId: id);
    throw _Fail('${label}reçu ${events.length}/$count, manquants '
        '${missing.take(10).toList()}${missing.length > 10 ? "…" : ""} ; '
        'Rust a émis ${info.emitted} (${info.stopReason.name})');
  }
  for (var i = 0; i < events.length; i++) {
    _check(events[i].seq == i,
        'désordre ou perte : position $i, seq ${events[i].seq}');
  }
}

Future<ScenarioResult> _run(String name, Future<String> Function() body,
    {bool skip = false, String skipReason = ''}) async {
  if (_only != null && !RegExp(_only!).hasMatch(name)) {
    return ScenarioResult(name, 'filtre', '', 0);
  }
  if (skip) return ScenarioResult(name, 'nt', skipReason, 0);
  final sw = Stopwatch()..start();
  try {
    final detail = await body().timeout(const Duration(seconds: 60));
    return ScenarioResult(name, 'ok', detail, sw.elapsedMilliseconds);
  } on _Fail catch (e) {
    return ScenarioResult(name, 'echec', e.message, sw.elapsedMilliseconds);
  } on TimeoutException {
    return ScenarioResult(
        name, 'echec', 'blocage (délai dépassé)', sw.elapsedMilliseconds);
  } catch (e) {
    return ScenarioResult(name, 'echec', 'exception : $e', sw.elapsedMilliseconds);
  }
}

/// Enveloppe un flux FRB pour que l'annulation côté Dart arrête l'émetteur Rust
/// (`stop_stream`) : contournement de l'absence de signal d'annulation sur le web.
Stream<FluxEvent> cancellable(int id, Stream<FluxEvent> source) {
  late StreamController<FluxEvent> ctrl;
  StreamSubscription<FluxEvent>? sub;
  ctrl = StreamController<FluxEvent>(
    onListen: () {
      sub = source.listen(ctrl.add,
          onError: ctrl.addError, onDone: ctrl.close);
    },
    onPause: () => sub?.pause(),
    onResume: () => sub?.resume(),
    onCancel: () {
      stopStream(streamId: id);
      return sub?.cancel();
    },
  );
  return ctrl.stream;
}

Future<String> _burst(Stream<FluxEvent> Function(int id, int count) f,
    int count) async {
  final id = newId();
  final sw = Stopwatch()..start();
  final events = await f(id, count).toList().timeout(_timeout);
  final ms = sw.elapsedMilliseconds;
  _checkSequence(events, id, count);
  final info = streamInfo(streamId: id);
  _check(!info.running && info.stopReason == StopReason.completed,
      'émetteur Rust : ${info.stopReason}');
  return '$count reçus dans l\'ordre en $ms ms';
}

/// Annulation de l'abonnement Dart en cours d'émission infinie, puis observation de
/// l'émetteur Rust pendant 1 s.
Future<String> _cancel(
    Stream<FluxEvent> Function(int id) start, {required bool explicit}) async {
  final id = newId();
  final received = <FluxEvent>[];
  final enough = Completer<void>();
  var afterCancel = 0;
  var cancelled = false;
  final source = explicit ? cancellable(id, start(id)) : start(id);
  final sub = source.listen((e) {
    if (cancelled) {
      afterCancel++;
      return;
    }
    received.add(e);
    if (received.length == 20 && !enough.isCompleted) enough.complete();
  });
  await enough.future.timeout(_timeout);
  cancelled = true;
  await sub.cancel();
  final atCancel = streamInfo(streamId: id).emitted;
  await _sleep(1000);
  final info = streamInfo(streamId: id);
  _checkSequence(received, id, 20);
  final extra = info.emitted - atCancel;
  if (info.running) {
    // Nettoyage pour ne pas polluer les scénarios suivants.
    stopStream(streamId: id);
    await _sleep(200);
    final after = streamInfo(streamId: id);
    throw _Fail('fuite : l\'émetteur Rust tourne encore 1 s après l\'annulation '
        '($extra émissions de plus ; stop_stream → '
        '${after.running ? "toujours en cours" : after.stopReason.name})');
  }
  _check(afterCancel == 0, '$afterCancel événements reçus après annulation');
  return 'arrêt Rust : ${info.stopReason.name}, $extra émission(s) après '
      'l\'annulation';
}

Future<String> _resubscribe() async {
  // 1) abonnement annulé SANS arrêt explicite (l'émetteur peut fuir), 2) nouvel abonnement.
  final oldId = newId();
  final oldSub = pacedAsync(streamId: oldId, count: 0, periodMs: 2).listen((_) {});
  await _sleep(100);
  await oldSub.cancel();
  final id = newId();
  final events =
      await pacedAsync(streamId: id, count: 100, periodMs: 1).toList().timeout(_timeout);
  _checkSequence(events, id, 100);
  final old = streamInfo(streamId: oldId);
  stopStream(streamId: oldId);
  // 3) même identifiant réutilisé après arrêt explicite.
  final id2 = newId();
  final first = await cancellable(id2, pacedAsync(streamId: id2, count: 0, periodMs: 1))
      .take(30)
      .toList()
      .timeout(_timeout);
  _checkSequence(first, id2, 30);
  await _sleep(100);
  final again = await pacedAsync(streamId: id2, count: 50, periodMs: 1)
      .toList()
      .timeout(_timeout);
  _checkSequence(again, id2, 50);
  return '100/100 puis 50/50 après réabonnement ; ancien émetteur '
      '${old.running ? "encore en cours (fuite)" : "arrêté"} au moment du réabonnement';
}

/// Consommateur lent : `await for` avec une attente dans la boucle, ce qui met en pause
/// l'abonnement entre deux événements (contre-pression Dart ordinaire).
Future<String> _slowConsumer(Stream<FluxEvent> Function(int id, int count) f) async {
  final id = newId();
  final events = <FluxEvent>[];
  Future<void> consume() async {
    await for (final e in f(id, 300)) {
      events.add(e);
      if (e.seq % 10 == 0) await _sleep(1);
    }
  }

  await consume().timeout(_timeout);
  _checkSequence(events, id, 300);
  return '300 reçus dans l\'ordre malgré les pauses';
}

Future<String> _concurrent() async {
  final ids = List.generate(4, (_) => newId());
  final futures = <Future<List<FluxEvent>>>[
    burstAsync(streamId: ids[0], count: 1000).toList(),
    burstSync(streamId: ids[1], count: 1000).toList(),
    pacedAsync(streamId: ids[2], count: 200, periodMs: 1).toList(),
    if (!sansEntetes)
      pacedWorker(streamId: ids[3], count: 200, periodMs: 1).toList()
    else
      pacedAsync(streamId: ids[3], count: 200, periodMs: 1).toList(),
  ];
  final results = await Future.wait(futures).timeout(_timeout);
  _checkSequence(results[0], ids[0], 1000, label: 'rafale async : ');
  _checkSequence(results[1], ids[1], 1000, label: 'rafale worker : ');
  _checkSequence(results[2], ids[2], 200, label: 'cadencé async : ');
  _checkSequence(results[3], ids[3], 200,
      label: sansEntetes ? 'cadencé async (2) : ' : 'cadencé worker : ');
  return '4 flux simultanés complets et ordonnés'
      '${sansEntetes ? " (4e flux async : pas de worker sans en-têtes)" : ""}';
}

Future<String> _errorAddError() async {
  final id = newId();
  final events = <FluxEvent>[];
  final errors = <Object>[];
  final done = Completer<void>();
  final order = <String>[];
  errorStream(streamId: id, mode: ErrorMode.addErrorThenContinue).listen(
    (e) {
      events.add(e);
      order.add('${e.seq}');
    },
    onError: (Object e) {
      errors.add(e);
      order.add('E');
    },
    onDone: done.complete,
    cancelOnError: false,
  );
  await done.future.timeout(_timeout);
  _check(errors.length == 1, '${errors.length} erreur(s) reçue(s)');
  final err = errors.single;
  // FRB 2.13 décode toute erreur de flux en AnyhowException (chaîne), jamais en
  // BridgeError typée : le code est encodé dans la chaîne côté Rust.
  _check(err is AnyhowException && err.message == 'Simulated:erreur simulée',
      'erreur de type ${err.runtimeType} : $err');
  _check(order.join(',') == '0,1,2,E,3,4', 'ordre ${order.join(',')}');
  return 'ordre reçu ${order.join(',')}, AnyhowException("Simulated:…"), flux terminé';
}

Future<String> _errorReturnErr() async {
  final id = newId();
  final events = <FluxEvent>[];
  final streamErrors = <Object>[];
  final zoneErrors = <Object>[];
  final done = Completer<void>();
  runZonedGuarded(() {
    errorStream(streamId: id, mode: ErrorMode.returnErr).listen(
      events.add,
      onError: streamErrors.add,
      onDone: done.complete,
      cancelOnError: false,
    );
  }, (e, _) => zoneErrors.add(e));
  await done.future.timeout(_timeout);
  await _sleep(200);
  final where = streamErrors.isNotEmpty
      ? 'dans le flux (${streamErrors.single.runtimeType})'
      : zoneErrors.isNotEmpty
          ? 'hors du flux : erreur asynchrone non interceptée '
              '(${zoneErrors.single.runtimeType})'
          : 'perdue';
  _check(events.length == 3, 'reçu ${events.length}/3 événements');
  _check(streamErrors.isNotEmpty,
      'Err renvoyé par la fonction : flux fermé normalement, erreur $where');
  return 'erreur $where';
}

/// Repli : requête ponctuelle + flux de notifications de version.
class RepliReader {
  RepliReader(this.watchId);

  final int watchId;
  Snapshot snapshot = const Snapshot(version: 0, items: []);
  int notifications = 0;
  int reads = 0;
  StreamSubscription<int>? _sub;
  bool _pending = false;

  void start() {
    _sub = repliWatch(watchId: watchId).listen((version) {
      notifications++;
      if (version <= snapshot.version || _pending) return;
      // Coalescence : une seule relecture par tour de boucle d'événements.
      _pending = true;
      scheduleMicrotask(() {
        _pending = false;
        reads++;
        snapshot = repliGet();
      });
    });
  }

  Future<void> stop() async {
    repliUnwatch(watchId: watchId);
    await _sub?.cancel();
  }
}

Future<String> _repli() async {
  repliReset();
  final reader = RepliReader(newId())..start();
  for (var i = 0; i < 1000; i++) {
    repliAdd(item: 'note $i');
    if (i % 100 == 99) await _sleep(0);
  }
  final sw = Stopwatch()..start();
  while (reader.snapshot.version < 1000) {
    _check(sw.elapsedMilliseconds < 10000,
        'lecteur bloqué à la version ${reader.snapshot.version}');
    await _sleep(10);
  }
  final items = reader.snapshot.items;
  _check(items.length == 1000, '${items.length}/1000 éléments');
  for (var i = 0; i < items.length; i++) {
    _check(items[i] == 'note $i', 'désordre en $i');
  }
  await reader.stop();
  final left = repliWatchers();
  _check(left == 0, '$left observateur(s) restant(s) côté Rust');
  return '1000/1000 via ${reader.reads} relectures pour '
      '${reader.notifications} notifications ; 0 observateur restant';
}

/// Tous les scénarios automatisables (le rechargement de page et à chaud sont pilotés de
/// l'extérieur).
Future<List<ScenarioResult>> runAll(
    {void Function(ScenarioResult)? onResult, String? only}) async {
  _only = only;
  const noWorker = 'pas de worker sans en-têtes : bloquerait le fil principal';
  final scenarios = <Future<ScenarioResult> Function()>[
    () => _run('ping', () async => 'ping ${await ping(delayMs: 10)} ms'),
    () => _run('rafale_async_1000',
        () => _burst((id, n) => burstAsync(streamId: id, count: n), 1000)),
    () => _run('rafale_worker_1000',
        () => _burst((id, n) => burstSync(streamId: id, count: n), 1000)),
    () => _run('rafale_async_10000',
        () => _burst((id, n) => burstAsync(streamId: id, count: n), 10000)),
    () => _run('rafale_worker_10000',
        () => _burst((id, n) => burstSync(streamId: id, count: n), 10000)),
    () => _run('simultanes_4', _concurrent),
    () => _run('consommateur_lent_async',
        () => _slowConsumer((id, n) => burstAsync(streamId: id, count: n))),
    () => _run('consommateur_lent_worker',
        () => _slowConsumer((id, n) => burstSync(streamId: id, count: n))),
    () => _run(
        'annulation_async',
        () => _cancel((id) => pacedAsync(streamId: id, count: 0, periodMs: 5),
            explicit: false)),
    () => _run(
        'annulation_worker',
        () => _cancel((id) => pacedWorker(streamId: id, count: 0, periodMs: 5),
            explicit: false),
        skip: sansEntetes,
        skipReason: noWorker),
    () => _run(
        'annulation_explicite_async',
        () => _cancel((id) => pacedAsync(streamId: id, count: 0, periodMs: 5),
            explicit: true)),
    () => _run(
        'annulation_explicite_worker',
        () => _cancel((id) => pacedWorker(streamId: id, count: 0, periodMs: 5),
            explicit: true),
        skip: sansEntetes,
        skipReason: noWorker),
    () => _run('reabonnement', _resubscribe),
    () => _run('erreur_add_error', _errorAddError),
    () => _run('erreur_return_err', _errorReturnErr),
    () => _run('repli_requete_notification', _repli),
  ];
  final results = <ScenarioResult>[];
  for (final s in scenarios) {
    final r = await s();
    if (r.status == 'filtre') continue;
    results.add(r);
    onResult?.call(r);
  }
  if (_only != null) return results;
  // Bilan de fuite : après nettoyage explicite, plus aucun émetteur ne doit tourner.
  await _sleep(300);
  results.add(ScenarioResult('emetteurs_restants', 'info',
      '${runningStreams()} émetteur(s) Rust encore en cours en fin de série', 0));
  onResult?.call(results.last);
  return results;
}
