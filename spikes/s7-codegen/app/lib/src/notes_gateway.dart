import 'package:injectable/injectable.dart';
import 'package:s7_app/src/note.dart';

/// Passerelle dont dépend la présentation.
abstract interface class NotesGateway {
  List<Note> lister();
  void ajouter(Note note);
}

/// Faux en mémoire, enregistré par injectable.
@LazySingleton(as: NotesGateway)
class NotesGatewayMemoire implements NotesGateway {
  final List<Note> _notes = [];

  @override
  List<Note> lister() => List.unmodifiable(_notes);

  @override
  void ajouter(Note note) => _notes.add(note);
}
