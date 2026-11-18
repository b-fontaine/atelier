import 'package:flutter_test/flutter_test.dart';
import 'world.dart';

/// Usage: la liste contient {2} notes
Future<void> laListeContientNotes(WidgetTester tester, num param1) async {
  expect(notes.length, param1);
}
