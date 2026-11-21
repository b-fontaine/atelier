import 'package:flutter_test/flutter_test.dart';
import 'world.dart';

/// Usage: la note {'<titre>'} a la priorité <priorite>
Future<void> laNoteALaPriorite(
    WidgetTester tester, String param1, num param2) async {
  expect(notes.contains((param1, param2)), isTrue);
}
