import 'backend.dart';
import 'kernel_object_id.dart';

/// A process handle. Create/destroy are the only lifecycle operations
/// modeled so far -- matches exactly what the kernel side's syscall
/// layer supports today (ProcessCreate/HandleClose), nothing claimed
/// beyond that.
class AdrianProcess {
  final KernelObjectId id;
  final AdrianBackend _backend;

  AdrianProcess._(this.id, this._backend);

  /// `null` if the backend couldn't allocate one (e.g. its table is
  /// full) -- mirrors the kernel side's `Option<KernelObjectId>`
  /// return. Not thrown as an exception: a full table is an expected,
  /// handleable condition, not a programming error.
  static AdrianProcess? create(AdrianBackend backend) {
    final id = backend.createProcess();
    if (id == null) return null;
    return AdrianProcess._(id, backend);
  }

  /// Create a thread owned by this process.
  AdrianThread? createThread() {
    final threadId = _backend.createThread(id);
    if (threadId == null) return null;
    return AdrianThread._(threadId, _backend);
  }

  /// `false` if this process was already destroyed or never existed.
  bool destroy() => _backend.destroyHandle(id);

  @override
  String toString() => 'AdrianProcess($id)';
}

/// A thread handle. Always created inside a process --
/// [AdrianProcess.createThread], never constructed on its own, same
/// as the kernel side requiring a process id to spawn a thread.
class AdrianThread {
  final KernelObjectId id;
  final AdrianBackend _backend;

  AdrianThread._(this.id, this._backend);

  bool destroy() => _backend.destroyHandle(id);

  @override
  String toString() => 'AdrianThread($id)';
}

/// A channel handle. Message send/receive aren't modeled on this
/// class yet -- only create/destroy are, matching the kernel side,
/// where Channel's real send/receive logic exists but isn't reachable
/// through any syscall number yet either.
class AdrianChannel {
  final KernelObjectId id;
  final AdrianBackend _backend;

  AdrianChannel._(this.id, this._backend);

  static AdrianChannel? create(AdrianBackend backend) {
    final id = backend.createChannel();
    if (id == null) return null;
    return AdrianChannel._(id, backend);
  }

  bool destroy() => _backend.destroyHandle(id);

  @override
  String toString() => 'AdrianChannel($id)';
}

/// An event handle: a single signal/clear flag, checked by polling
/// ([isSignaled]) rather than blocking -- there's no real thread-
/// blocking mechanism on the kernel side yet for a wait to suspend
/// against, so polling is the honest option, not a placeholder for
/// something more real.
class AdrianEvent {
  final KernelObjectId id;
  final AdrianBackend _backend;

  AdrianEvent._(this.id, this._backend);

  static AdrianEvent? create(AdrianBackend backend) {
    final id = backend.createEvent();
    if (id == null) return null;
    return AdrianEvent._(id, backend);
  }

  bool signal() => _backend.signalEvent(id);

  /// `null` if this event no longer exists (already destroyed).
  bool? get isSignaled => _backend.isEventSignaled(id);

  bool destroy() => _backend.destroyHandle(id);

  @override
  String toString() => 'AdrianEvent($id)';
}
