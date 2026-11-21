import 'package:dio/dio.dart';
import 'package:retrofit/retrofit.dart';
import 'package:s7_app/src/note.dart';

part 'notes_api.g.dart';

/// Client HTTP généré par retrofit_generator.
@RestApi()
abstract class NotesApi {
  factory NotesApi(Dio dio, {String? baseUrl}) = _NotesApi;

  @GET('/notes')
  Future<List<Note>> lister();
}
