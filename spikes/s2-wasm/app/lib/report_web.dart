import 'dart:js_interop';
import 'dart:js_interop_unsafe';

/// Web : expose le résultat à l'automate de test dans `globalThis.s2Result`.
void publishResult(String json) {
  globalContext.setProperty('s2Result'.toJS, json.toJS);
  // ignore: avoid_print
  print('S2:result=$json');
}
