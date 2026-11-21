import 'package:bdd_widget_test/data_table.dart' as bdd;
import 'package:flutter_test/flutter_test.dart';
import 'world.dart';

/// Usage: j'ajoute les notes suivantes:
Future<void> jajouteLesNotesSuivantes(
    WidgetTester tester, bdd.DataTable dataTable) async {
  for (final m in dataTable.asMaps()) {
    notes.add((m['titre'] as String, m['priorite'] as num));
  }
}
