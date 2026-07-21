/// Dart application-layer SDK for ADRIAN OS.
///
/// Provides the typed API applications call -- process/thread/
/// channel/event lifecycle -- backed by [HostSimulationBackend] for
/// now. See `src/backend.dart` for exactly what stands between this
/// and a real kernel bridge.
library adrian_os;

export 'src/backend.dart';
export 'src/kernel_object_id.dart';
export 'src/objects.dart';
