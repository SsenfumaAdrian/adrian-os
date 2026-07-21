/// Opaque identifier for a kernel object (process, thread, channel,
/// event, ...). Mirrors `adrian_kernel::object::KernelObjectId` on the
/// Rust side deliberately -- same shape, same reserved-zero-as-
/// sentinel convention -- so ids round-trip cleanly once a real
/// backend bridges into the actual kernel instead of simulating one.
class KernelObjectId {
  final int value;

  const KernelObjectId(this.value);

  /// The reserved sentinel meaning "no object". A real backend never
  /// hands this out as an allocated id, same convention as the
  /// kernel-side allocator starting its counter at 1, not 0.
  static const KernelObjectId none = KernelObjectId(0);

  bool get isNone => value == 0;

  @override
  bool operator ==(Object other) => other is KernelObjectId && other.value == value;

  @override
  int get hashCode => value.hashCode;

  @override
  String toString() => 'KernelObjectId($value)';
}
