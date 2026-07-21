import 'package:adrian_os/adrian_os.dart';
import 'package:test/test.dart';

void main() {
  group('HostSimulationBackend', () {
    late HostSimulationBackend backend;

    setUp(() {
      backend = HostSimulationBackend();
    });

    test('createProcess returns a valid nonzero id', () {
      final id = backend.createProcess();
      expect(id, isNotNull);
      expect(id!.isNone, isFalse);
    });

    test('successive creates return distinct ids', () {
      final a = backend.createProcess();
      final b = backend.createProcess();
      expect(a, isNot(equals(b)));
    });

    test('createThread fails for an unknown process id', () {
      final threadId = backend.createThread(const KernelObjectId(999));
      expect(threadId, isNull);
    });

    test('createThread succeeds for a real process id', () {
      final processId = backend.createProcess()!;
      final threadId = backend.createThread(processId);
      expect(threadId, isNotNull);
    });

    test('createThread fails once the owning process is destroyed', () {
      final processId = backend.createProcess()!;
      backend.destroyHandle(processId);
      expect(backend.createThread(processId), isNull);
    });

    test('destroyHandle removes a real id and rejects destroying it twice', () {
      final id = backend.createChannel()!;
      expect(backend.destroyHandle(id), isTrue);
      expect(backend.destroyHandle(id), isFalse);
    });

    test('destroyHandle on an id nobody created fails', () {
      expect(backend.destroyHandle(const KernelObjectId(12345)), isFalse);
    });

    test('signalEvent fails for a non-event id', () {
      final processId = backend.createProcess()!;
      expect(backend.signalEvent(processId), isFalse);
    });

    test('isEventSignaled is null for a non-event id', () {
      final processId = backend.createProcess()!;
      expect(backend.isEventSignaled(processId), isNull);
    });

    test('event starts unsignaled, signal flips it, destroy clears it', () {
      final eventId = backend.createEvent()!;
      expect(backend.isEventSignaled(eventId), isFalse);

      expect(backend.signalEvent(eventId), isTrue);
      expect(backend.isEventSignaled(eventId), isTrue);

      backend.destroyHandle(eventId);
      expect(backend.isEventSignaled(eventId), isNull);
    });
  });
}
