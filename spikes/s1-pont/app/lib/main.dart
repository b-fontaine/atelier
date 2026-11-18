import 'dart:async';

import 'package:material_ui/material_ui.dart';
import 'package:s1_app/src/rust/api/demo.dart';
import 'package:s1_app/src/rust/frb_generated.dart';

Future<void> main() async {
  await RustLib.init();
  runApp(const S1App());
}

class S1App extends StatelessWidget {
  const S1App({super.key});

  @override
  Widget build(BuildContext context) =>
      const MaterialApp(home: S1Page());
}

class S1Page extends StatefulWidget {
  const S1Page({super.key});

  @override
  State<S1Page> createState() => _S1PageState();
}

class _S1PageState extends State<S1Page> {
  String _sum = 'somme : en attente';
  final List<int> _ticks = [];
  bool _done = false;
  StreamSubscription<int>? _sub;

  @override
  void initState() {
    super.initState();
    // Appel async : Future renvoyé par Rust.
    delayedSum(a: 40, b: 2, delayMs: BigInt.from(100)).then(
      (v) => setState(() => _sum = 'somme : $v'),
      onError: (Object e) => setState(() => _sum = 'erreur : $e'),
    );
    // Flux : StreamSink côté Rust.
    _sub = tickStream(count: 5, periodMs: BigInt.from(50)).listen(
      (t) => setState(() => _ticks.add(t)),
      onDone: () => setState(() => _done = true),
    );
  }

  @override
  void dispose() {
    _sub?.cancel();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => Scaffold(
        appBar: AppBar(title: const Text('Essai S1 — pont FRB')),
        body: Center(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Text(_sum, key: const Key('sum')),
              Text('ticks : ${_ticks.join(',')}', key: const Key('ticks')),
              if (_done) const Text('flux terminé', key: Key('done')),
            ],
          ),
        ),
      );
}
