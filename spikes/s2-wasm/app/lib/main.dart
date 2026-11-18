import 'dart:convert';

import 'package:material_ui/material_ui.dart';
import 'package:s2_app/report_stub.dart'
    if (dart.library.js_interop) 'package:s2_app/report_web.dart';
import 'package:s2_app/src/rust/api/demo.dart';
import 'package:s2_app/src/rust/api/store.dart';
import 'package:s2_app/src/rust/frb_generated.dart';

/// Scénario piloté par l'URL :
/// `?mode=memory|opfs` (défaut : memory), `&lock=0|1` (défaut : 1), `&add=<texte>`,
/// `&stress=<n>` (n insertions pendant que d'autres Workers travaillent).
Future<Map<String, Object?>> runScenario(Map<String, String> params) async {
  final result = <String, Object?>{};
  try {
    result['sum'] = await delayedSum(a: 40, b: 2, delayMs: 50);
    final mode = params['mode'] == 'opfs'
        ? StoreMode.persistent
        : StoreMode.memory;
    final info = await storeOpen(
      mode: mode,
      dbName: params['db'] ?? 's2-notes.db',
      tabLock: params['lock'] != '0',
    );
    result['bridgeContext'] = info.bridgeContext;
    result['storeContext'] = info.storeContext;
    result['before'] = await storeList();
    final add = params['add'];
    if (add != null) {
      result['addedId'] = await storeAdd(text: add);
    }
    final stress = int.tryParse(params['stress'] ?? '');
    if (stress != null) {
      final watch = Stopwatch()..start();
      // Référence sans concurrence, puis la même charge avec le pool occupé.
      await storeAddMany(prefix: 'seul', count: stress);
      result['aloneMs'] = watch.elapsedMilliseconds;
      watch.reset();
      final done = await Future.wait<Object>([
        storeAddMany(prefix: 'charge', count: stress).then((v) {
          result['insertMs'] = watch.elapsedMilliseconds;
          return v;
        }),
        for (var i = 0; i < 16; i++) churnAllocator(rounds: 2000),
        for (var i = 0; i < 16; i++) delayedSum(a: i, b: 1, delayMs: 5),
      ]);
      result['stressTotal'] = done.first;
      result['stressMs'] = watch.elapsedMilliseconds;
      result['count'] = (await storeList()).length;
    } else {
      result['after'] = await storeList();
    }
    result['ok'] = true;
  } on BridgeError catch (e) {
    result['ok'] = false;
    result['errorKind'] = e.kind.name;
    result['errorMessage'] = e.message;
  } catch (e) {
    result['ok'] = false;
    result['errorKind'] = 'untyped';
    result['errorMessage'] = e.toString();
  }
  return result;
}

Future<void> main() async {
  await RustLib.init();
  runApp(const S2App());
}

class S2App extends StatelessWidget {
  const S2App({super.key});

  @override
  Widget build(BuildContext context) => const MaterialApp(home: S2Page());
}

class S2Page extends StatefulWidget {
  const S2Page({super.key});

  @override
  State<S2Page> createState() => _S2PageState();
}

class _S2PageState extends State<S2Page> {
  Map<String, Object?>? _result;

  @override
  void initState() {
    super.initState();
    runScenario(Uri.base.queryParameters).then((r) {
      publishResult(jsonEncode(r));
      setState(() => _result = r);
    });
  }

  @override
  Widget build(BuildContext context) {
    final r = _result;
    final String text;
    if (r == null) {
      text = 'en cours…';
    } else if (r['errorKind'] == 'storeOpenInAnotherTab') {
      text = 'Atelier est déjà ouvert dans un autre onglet.';
    } else {
      text = const JsonEncoder.withIndent('  ').convert(r);
    }
    return Scaffold(
      appBar: AppBar(title: const Text('Essai S2 — WASM + SQLite')),
      body: Center(child: Text(text, key: const Key('result'))),
    );
  }
}
