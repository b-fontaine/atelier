// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint, type=warning

import 'package:bdd_widget_test/data_table.dart' as bdd;
import 'package:material_ui/material_ui.dart';
import 'package:flutter_test/flutter_test.dart';

import './step/une_liste_de_notes_vide.dart';
import './step/jajoute_la_note_avec_la_priorite.dart';
import './step/la_liste_contient_note.dart';
import './step/la_note_a_la_priorite.dart';
import './step/japplique_un_coefficient_de.dart';
import './step/la_priorite_effective_de_est.dart';
import './step/le_statut_est_ouvert.dart';
import './step/jajoute_les_notes_suivantes.dart';
import './step/la_liste_contient_notes.dart';

void main() {
  group('''Gestion de notes''', () {
    Future<void> bddSetUp(WidgetTester tester) async {
      await uneListeDeNotesVide(tester);
    }

    testWidgets('''Ajouter une note''', (tester) async {
      await bddSetUp(tester);
      await jajouteLaNoteAvecLaPriorite(tester, 'Acheter du pain', 3);
      await laListeContientNote(tester, 1);
      await laNoteALaPriorite(tester, 'Acheter du pain', 3);
    }, tags: ['core', 'gui']);
    testWidgets('''Cœur seul, flottant et mot isolé''', (tester) async {
      await bddSetUp(tester);
      await jajouteLaNoteAvecLaPriorite(tester, 'Lire', 2);
      await jappliqueUnCoefficientDe(tester, 1.5);
      await laPrioriteEffectiveDeEst(tester, 'Lire', 3);
      await leStatutEstOuvert(tester);
    }, tags: ['core']);
    testWidgets('''Ajouter plusieurs notes (Courses, 1)''', (tester) async {
      await bddSetUp(tester);
      await jajouteLaNoteAvecLaPriorite(tester, 'Courses', 1);
      await laListeContientNote(tester, 1);
      await laNoteALaPriorite(tester, 'Courses', 1);
    }, tags: ['core', 'gui']);
    testWidgets('''Ajouter plusieurs notes (Sport, 5)''', (tester) async {
      await bddSetUp(tester);
      await jajouteLaNoteAvecLaPriorite(tester, 'Sport', 5);
      await laListeContientNote(tester, 1);
      await laNoteALaPriorite(tester, 'Sport', 5);
    }, tags: ['core', 'gui']);
    testWidgets('''Ajouter plusieurs notes d'un coup''', (tester) async {
      await bddSetUp(tester);
      await jajouteLesNotesSuivantes(
          tester,
          const bdd.DataTable([
            ["titre", "priorite"],
            ["Courses", 1],
            ["Sport", 5]
          ]));
      await laListeContientNotes(tester, 2);
    }, tags: ['core', 'gui']);
    group('''Une note vide est refusée''', () {
      testWidgets('''Titre vide''', (tester) async {
        await bddSetUp(tester);
        await jajouteLaNoteAvecLaPriorite(tester, '', 1);
        await laListeContientNote(tester, 0);
      }, tags: ['core', 'gui']);
    });
  });
}
