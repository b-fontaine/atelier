import 'package:flutter_test/flutter_test.dart';
import 'world.dart';

/// Usage: j'ajoute la note {''} avec la priorité {1}
Future<void> jajouteLaNoteAvecLaPriorite(
    WidgetTester tester, String param1, num param2) async {
  if (param1.isNotEmpty) notes.add((param1, param2));
}
