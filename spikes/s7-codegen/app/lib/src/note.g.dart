// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'note.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

_Note _$NoteFromJson(Map<String, dynamic> json) => _Note(
  titre: json['titre'] as String,
  priorite: (json['priorite'] as num).toInt(),
);

Map<String, dynamic> _$NoteToJson(_Note instance) => <String, dynamic>{
  'titre': instance.titre,
  'priorite': instance.priorite,
};
