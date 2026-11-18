import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:material_ui/material_ui.dart';
import 'package:s1_app/main.dart';
import 'package:s1_app/src/rust/api/demo.dart';
import 'package:s1_app/src/rust/frb_generated.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  setUpAll(() async => RustLib.init());

  test('fonction async Rust', () async {
    expect(await delayedSum(a: 40, b: 2, delayMs: BigInt.from(50)), 42);
  });

  test('erreur Rust remontée comme exception Dart', () async {
    await expectLater(
      delayedSum(a: 1, b: 1, delayMs: BigInt.from(60000)),
      throwsA(isA<BridgeError>()
          .having((e) => e.kind, 'kind', BridgeErrorKind.invalidInput)),
    );
  });

  test('flux Rust (StreamSink)', () async {
    final ticks =
        await tickStream(count: 5, periodMs: BigInt.from(20)).toList();
    expect(ticks, [0, 1, 2, 3, 4]);
  });

  testWidgets('UI : somme et flux affichés', (tester) async {
    await tester.pumpWidget(const S1App());
    // Laisse le temps réel s'écouler pour les timers Rust.
    for (var i = 0; i < 40; i++) {
      await tester.runAsync(
          () => Future<void>.delayed(const Duration(milliseconds: 25)));
      await tester.pump();
      if (find.byKey(const Key('done')).evaluate().isNotEmpty &&
          find.text('somme : 42').evaluate().isNotEmpty) {
        break;
      }
    }
    expect(find.text('somme : 42'), findsOneWidget);
    expect(find.text('ticks : 0,1,2,3,4'), findsOneWidget);
    expect(find.text('flux terminé'), findsOneWidget);
  });
}
