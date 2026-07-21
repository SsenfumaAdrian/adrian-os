import 'package:adrian_os/adrian_os.dart';
import 'package:test/test.dart';

void main() {
  group('KernelObjectId', () {
    test('none is the zero sentinel', () {
      expect(KernelObjectId.none.value, equals(0));
      expect(KernelObjectId.none.isNone, isTrue);
    });

    test('equality is based on value, not identity', () {
      expect(const KernelObjectId(5), equals(const KernelObjectId(5)));
      expect(const KernelObjectId(5), isNot(equals(const KernelObjectId(6))));
    });

    test('equal ids have equal hash codes', () {
      expect(const KernelObjectId(7).hashCode, equals(const KernelObjectId(7).hashCode));
    });

    test('non-zero id is not none', () {
      expect(const KernelObjectId(1).isNone, isFalse);
    });
  });
}
