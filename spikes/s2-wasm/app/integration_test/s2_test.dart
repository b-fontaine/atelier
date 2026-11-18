import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:s2_app/main.dart';
import 'package:s2_app/src/rust/frb_generated.dart';

/// Même scénario que sur le web, exécuté en natif (même code Rust et Dart).
void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  setUpAll(() async => RustLib.init());

  test('stockage en mémoire', () async {
    final r = await runScenario({'add': 'bonjour'});
    expect(r['ok'], true, reason: '$r');
    expect(r['sum'], 42);
    expect(r['before'], isEmpty);
    expect(r['after'], ['bonjour']);
  });

  test('stockage persistant : relu après réouverture', () async {
    final dir = await Directory.systemTemp.createTemp('s2-');
    final db = '${dir.path}/notes.db';
    final first = await runScenario({'mode': 'opfs', 'db': db, 'add': 'un'});
    expect(first['after'], ['un'], reason: '$first');
    final second = await runScenario({'mode': 'opfs', 'db': db, 'add': 'deux'});
    expect(second['before'], ['un'], reason: '$second');
    expect(second['after'], ['un', 'deux']);
    await dir.delete(recursive: true);
  });

  test('charge : insertions pendant que le pool travaille', () async {
    final r = await runScenario({'stress': '2000'});
    expect(r['ok'], true, reason: '$r');
    expect(r['stressTotal'], 4000);
    expect(r['count'], 4000);
  });
}
