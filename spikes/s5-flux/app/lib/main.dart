// App d'essai S5. Deux modes, choisis par l'URL (web) ou par --dart-define (natif) :
// - `?auto=1` : exécute tous les scénarios et écrit `S5RESULT {json}` puis `S5DONE` dans la
//   console (lue par le harnais Playwright) ;
// - `?live=1` : flux infinis affichés en continu, avec `S5LIVE …` toutes les 500 ms, pour les
//   essais de rechargement à chaud et de rechargement de page.
import 'dart:async';
import 'dart:convert';

import 'package:flutter/foundation.dart';
import 'package:material_ui/material_ui.dart';

import 'scenarios.dart';
import 'src/rust/api/flux.dart';
import 'src/rust/frb_generated.dart';

/// Change à chaque modification pour vérifier qu'un rechargement à chaud a pris effet.
String codeVersion() => 'v1';

Future<void> main() async {
  await RustLib.init();
  final params = Uri.base.queryParameters;
  final mode = params['auto'] == '1'
      ? 'auto'
      : params['live'] == '1'
          ? 'live'
          : const String.fromEnvironment('S5_MODE', defaultValue: 'auto');
  runApp(S5App(mode: mode));
}

class S5App extends StatelessWidget {
  const S5App({super.key, required this.mode});

  final String mode;

  @override
  Widget build(BuildContext context) => MaterialApp(
        home: mode == 'live' ? const LivePage() : const AutoPage(),
      );
}

class AutoPage extends StatefulWidget {
  const AutoPage({super.key});

  @override
  State<AutoPage> createState() => _AutoPageState();
}

class _AutoPageState extends State<AutoPage> {
  final _results = <ScenarioResult>[];
  bool _done = false;

  @override
  void initState() {
    super.initState();
    debugPrint('S5START sansEntetes=$sansEntetes web=$kIsWeb');
    runAll(only: Uri.base.queryParameters['only'], onResult: (r) {
      debugPrint('S5RESULT ${jsonEncode(r.toJson())}');
      if (mounted) setState(() => _results.add(r));
    }).then((_) {
      debugPrint('S5DONE');
      if (mounted) setState(() => _done = true);
    });
  }

  @override
  Widget build(BuildContext context) => Scaffold(
        appBar: AppBar(
            title: Text('Essai S5 — flux${sansEntetes ? " (sans en-têtes)" : ""}')),
        body: ListView(children: [
          for (final r in _results)
            ListTile(title: Text(r.name), subtitle: Text('${r.status} — ${r.detail}')),
          if (_done) const ListTile(key: Key('done'), title: Text('terminé')),
        ]),
      );
}

/// Flux infinis pour les essais de rechargement : un émetteur `async` (fil principal) et, si
/// possible, un émetteur worker. Compte les événements reçus et ceux d'un autre flux.
class LivePage extends StatefulWidget {
  const LivePage({super.key});

  @override
  State<LivePage> createState() => _LivePageState();
}

class _LivePageState extends State<LivePage> {
  final _ids = <String, int>{};
  final _recv = <String, int>{};
  final _lastSeq = <String, int>{};
  var _foreign = 0;
  var _disorder = 0;
  final _subs = <StreamSubscription<FluxEvent>>[];
  Timer? _timer;

  void _listen(String name, Stream<FluxEvent> Function(int id) start) {
    final id = newId();
    _ids[name] = id;
    _recv[name] = 0;
    _lastSeq[name] = -1;
    _subs.add(start(id).listen((e) {
      if (e.streamId != id) {
        _foreign++;
        return;
      }
      if (e.seq != _lastSeq[name]! + 1) _disorder++;
      _lastSeq[name] = e.seq;
      _recv[name] = _recv[name]! + 1;
    }));
  }

  @override
  void initState() {
    super.initState();
    _listen('async', (id) => pacedAsync(streamId: id, count: 0, periodMs: 20));
    if (!sansEntetes) {
      _listen('worker', (id) => pacedWorker(streamId: id, count: 0, periodMs: 20));
    }
    _timer = Timer.periodic(const Duration(milliseconds: 500), (_) {
      final rust = {
        for (final e in _ids.entries) e.key: streamInfo(streamId: e.value).emitted
      };
      debugPrint('S5LIVE ${jsonEncode({
            'code': codeVersion(),
            'ids': _ids,
            'recv': _recv,
            'rustEmitted': rust,
            'foreign': _foreign,
            'disorder': _disorder,
            'running': runningStreams(),
          })}');
      if (mounted) setState(() {});
    });
  }

  @override
  void dispose() {
    _timer?.cancel();
    for (final s in _subs) {
      s.cancel();
    }
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => Scaffold(
        appBar: AppBar(title: const Text('Essai S5 — flux en continu')),
        body: Center(
          child: Text(
            'code ${codeVersion()}\nreçus $_recv\nétrangers $_foreign, désordre $_disorder',
            key: const Key('live'),
          ),
        ),
      );
}
