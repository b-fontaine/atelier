// Copie les .feature étiquetés @gui de ../features vers test/features.
// Usage : dart run tool/sync_features.dart [--check] [--tag gui]
// --check : ne modifie rien, code de sortie 1 en cas d'écart.
import 'dart:io';

const source = '../features';
const dest = 'test/features';

Future<void> main(List<String> args) async {
  final check = args.contains('--check');
  final src = Directory(source);
  final expected = <String, String>{}; // chemin relatif -> contenu
  for (final f in src.listSync(recursive: true).whereType<File>()) {
    if (!f.path.endsWith('.feature')) continue;
    final content = f.readAsStringSync();
    if (!RegExp(r'@gui\b').hasMatch(content)) continue;
    expected[f.path.substring(src.path.length + 1)] = content;
  }
  final d = Directory(dest)..createSync(recursive: true);
  final actual = <String, String>{
    for (final f in d.listSync(recursive: true).whereType<File>())
      if (f.path.endsWith('.feature'))
        f.path.substring(d.path.length + 1): f.readAsStringSync(),
  };
  final diffs = <String>[];
  for (final e in expected.entries) {
    if (!actual.containsKey(e.key)) {
      diffs.add('manquant : ${e.key}');
    } else if (actual[e.key] != e.value) {
      diffs.add('différent : ${e.key}');
    }
  }
  for (final k in actual.keys) {
    if (!expected.containsKey(k)) diffs.add('orphelin : $k');
  }
  if (check) {
    if (diffs.isEmpty) {
      stdout.writeln('sync_features : OK');
      return;
    }
    stderr.writeln('sync_features : écart détecté\n${diffs.join('\n')}');
    exit(1);
  }
  for (final e in expected.entries) {
    final out = File('$dest/${e.key}')..createSync(recursive: true);
    out.writeAsStringSync(e.value);
  }
  for (final k in actual.keys) {
    if (!expected.containsKey(k)) File('$dest/$k').deleteSync();
  }
  stdout.writeln('sync_features : ${expected.length} fichier(s) copié(s)');
}
