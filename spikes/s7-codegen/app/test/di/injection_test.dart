import 'package:flutter_test/flutter_test.dart';
import 'package:s7_app/src/injection.dart';
import 'package:s7_app/src/note.dart';
import 'package:s7_app/src/notes_gateway.dart';
import 'package:s7_app/src/reglage.dart';

void main() {
  test('injectable enregistre la passerelle, freezed et json fonctionnent', () {
    configureDependencies();
    final gateway = getIt<NotesGateway>()
      ..ajouter(const Note(titre: 'Lire', priorite: 2));
    expect(identical(gateway, getIt<NotesGateway>()), isTrue);
    expect(gateway.lister().single.toJson(), {'titre': 'Lire', 'priorite': 2});
    expect(Note.fromJson(const {'titre': 'Lire', 'priorite': 2}),
        const Note(titre: 'Lire', priorite: 2));
    expect(const Reglage(cle: 'a', valeur: 'b').toJson(), {'cle': 'a', 'valeur': 'b'});
  });
}
