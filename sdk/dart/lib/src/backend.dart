import 'kernel_object_id.dart';

/// What a real backend needs to provide: create/destroy for each
/// object kind, plus event signal/check. A real implementation would
/// route these through FFI into the actual kernel's syscall dispatch
/// (`adrian_kernel::syscall::dispatch_syscall`) -- that bridge doesn't
/// exist yet, since there is no bootable OS and no Dart runtime port
/// to run on top of it once one exists.
///
/// [HostSimulationBackend] below is what exists instead: an honest,
/// in-memory stand-in with the same create/destroy/signal semantics,
/// so application code written against this interface doesn't need
/// to change when a real backend eventually replaces it -- only which
/// backend gets wired up does.
abstract class AdrianBackend {
  KernelObjectId? createProcess();
  KernelObjectId? createThread(KernelObjectId processId);
  KernelObjectId? createChannel();
  KernelObjectId? createEvent();

  /// Destroy any handle by id, regardless of kind -- mirrors the
  /// kernel side's generic HandleClose dispatch (object.rs's
  /// HandleRegistry resolving which table actually owns an id).
  bool destroyHandle(KernelObjectId id);

  /// Not reachable via any syscall number on the kernel side yet
  /// either (SyscallNumber only defines EventCreate, no
  /// EventSignal/EventWait) -- a direct backend call on both sides
  /// for the same reason.
  bool signalEvent(KernelObjectId id);

  /// `null` if `id` isn't a currently-live event.
  bool? isEventSignaled(KernelObjectId id);
}

enum _ObjectKind { process, thread, channel, event }

/// In-memory stand-in for the real kernel bridge. Mirrors the Rust
/// side's own `object::HandleRegistry` shape: one map from id to
/// kind, with a monotonic counter for allocation. Not a
/// reimplementation of the kernel's actual logic -- just enough to
/// let application code exercise realistic create/destroy/signal
/// sequences, and see realistic failure modes (destroying an unknown
/// id, checking a destroyed event), before any real backend exists.
class HostSimulationBackend implements AdrianBackend {
  int _nextId = 1;
  final Map<int, _ObjectKind> _registry = {};
  final Set<int> _signaledEvents = {};

  KernelObjectId _allocate(_ObjectKind kind) {
    final id = _nextId;
    _nextId += 1;
    _registry[id] = kind;
    return KernelObjectId(id);
  }

  @override
  KernelObjectId? createProcess() => _allocate(_ObjectKind.process);

  @override
  KernelObjectId? createThread(KernelObjectId processId) {
    if (_registry[processId.value] != _ObjectKind.process) {
      return null;
    }
    return _allocate(_ObjectKind.thread);
  }

  @override
  KernelObjectId? createChannel() => _allocate(_ObjectKind.channel);

  @override
  KernelObjectId? createEvent() => _allocate(_ObjectKind.event);

  @override
  bool destroyHandle(KernelObjectId id) {
    if (!_registry.containsKey(id.value)) {
      return false;
    }
    _registry.remove(id.value);
    _signaledEvents.remove(id.value);
    return true;
  }

  @override
  bool signalEvent(KernelObjectId id) {
    if (_registry[id.value] != _ObjectKind.event) {
      return false;
    }
    _signaledEvents.add(id.value);
    return true;
  }

  @override
  bool? isEventSignaled(KernelObjectId id) {
    if (_registry[id.value] != _ObjectKind.event) {
      return null;
    }
    return _signaledEvents.contains(id.value);
  }
}
