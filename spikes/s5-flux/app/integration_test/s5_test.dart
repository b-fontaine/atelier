// ignore_for_file: avoid_print
// Référence native (Linux) : mêmes scénarios que sur le web.
// Les résultats sont imprimés (`S5RESULT {json}`) ; le test ne fait pas échouer la série sur
// un scénario en échec, pour obtenir le tableau complet.
import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:s5_app/scenarios.dart';
import 'package:s5_app/src/rust/frb_generated.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  setUpAll(() async => RustLib.init());

  test('scénarios S5 (natif)', () async {
    final results = await runAll(
        onResult: (r) => print('S5RESULT ${jsonEncode(r.toJson())}'));
    print('S5DONE');
    expect(results, isNotEmpty);
  }, timeout: const Timeout(Duration(minutes: 5)));
}
