import 'package:adrian_os/adrian_os.dart';
import 'package:test/test.dart';

void main() {
  group('AdrianProcess / AdrianThread / AdrianChannel / AdrianEvent', () {
    late HostSimulationBackend backend;

    setUp(() {
      backend = HostSimulationBackend();
    });

    test('process create and destroy round trip', () {
      final process = AdrianProcess.create(backend);
      expect(process, isNotNull);
      expect(process!.destroy(), isTrue);
      expect(process.destroy(), isFalse); // already gone
    });

    test('process can create a thread inside it', () {
      final process = AdrianProcess.create(backend)!;
      final thread = process.createThread();
      expect(thread, isNotNull);
      expect(thread!.destroy(), isTrue);
    });

    test('two processes produce independently destroyable threads', () {
      final processA = AdrianProcess.create(backend)!;
      final processB = AdrianProcess.create(backend)!;
      final threadA = processA.createThread()!;
      final threadB = processB.createThread()!;

      expect(threadA.id, isNot(equals(threadB.id)));
      expect(threadA.destroy(), isTrue);
      // Destroying one thread doesn't affect the other.
      expect(threadB.destroy(), isTrue);
    });

    test('channel create and destroy round trip', () {
      final channel = AdrianChannel.create(backend);
      expect(channel, isNotNull);
      expect(channel!.destroy(), isTrue);
    });

    test('event signal and check round trip through the real object', () {
      final event = AdrianEvent.create(backend)!;
      expect(event.isSignaled, isFalse);
      expect(event.signal(), isTrue);
      expect(event.isSignaled, isTrue);
    });

    test('destroyed event reports null, not false, for isSignaled', () {
      final event = AdrianEvent.create(backend)!;
      event.destroy();
      // null (unknown/gone) is meaningfully different from false
      // (known to exist, just not signaled) -- the API preserves that
      // distinction rather than collapsing it.
      expect(event.isSignaled, isNull);
    });
  });
}
