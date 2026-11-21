import 'package:json_annotation/json_annotation.dart';

part 'reglage.g.dart';

/// Classe sérialisée par json_serializable seul.
@JsonSerializable()
class Reglage {
  const Reglage({required this.cle, required this.valeur});

  factory Reglage.fromJson(Map<String, dynamic> json) =>
      _$ReglageFromJson(json);

  final String cle;
  final String valeur;

  Map<String, dynamic> toJson() => _$ReglageToJson(this);
}
