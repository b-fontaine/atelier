import 'package:freezed_annotation/freezed_annotation.dart';

part 'note.freezed.dart';
part 'note.g.dart';

/// Modèle côté interface, généré par freezed et json_serializable.
@freezed
abstract class Note with _$Note {
  const factory Note({required String titre, required int priorite}) = _Note;

  factory Note.fromJson(Map<String, dynamic> json) => _$NoteFromJson(json);
}
