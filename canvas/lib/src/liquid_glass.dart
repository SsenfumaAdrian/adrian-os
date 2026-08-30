/// Liquid Glass visual language design tokens and glassmorphism UI tree components.
library canvas.liquid_glass;

/// Color representation in ARGB format.
class CanvasColor {
  final int value;

  const CanvasColor(this.value);

  const CanvasColor.fromARGB(int a, int r, int g, int b)
      : value = (((a & 0xff) << 24) |
                ((r & 0xff) << 16) |
                ((g & 0xff) << 8) |
                (b & 0xff)) &
            0xFFFFFFFF;

  int get alpha => (value >> 24) & 0xFF;
  int get red => (value >> 16) & 0xFF;
  int get green => (value >> 8) & 0xFF;
  int get blue => value & 0xFF;
}

/// Glassmorphism visual properties and refractive tokens.
class LiquidGlassTheme {
  final double blurRadius;
  final double refractiveIndex;
  final double specularHighlight;
  final double borderOpacity;
  final CanvasColor backgroundColor;
  final CanvasColor borderColor;

  const LiquidGlassTheme({
    this.blurRadius = 24.0,
    this.refractiveIndex = 1.15,
    this.specularHighlight = 0.35,
    this.borderOpacity = 0.20,
    this.backgroundColor = const CanvasColor.fromARGB(40, 255, 255, 255),
    this.borderColor = const CanvasColor.fromARGB(60, 255, 255, 255),
  });

  /// Preset: Ultra-clear high-refraction glass panel.
  static const LiquidGlassTheme crystal = LiquidGlassTheme(
    blurRadius: 30.0,
    refractiveIndex: 1.25,
    specularHighlight: 0.50,
    borderOpacity: 0.30,
    backgroundColor: CanvasColor.fromARGB(25, 255, 255, 255),
    borderColor: CanvasColor.fromARGB(80, 255, 255, 255),
  );

  /// Preset: Deep frosted dark mode glass panel.
  static const LiquidGlassTheme obsidian = LiquidGlassTheme(
    blurRadius: 40.0,
    refractiveIndex: 1.10,
    specularHighlight: 0.20,
    borderOpacity: 0.15,
    backgroundColor: CanvasColor.fromARGB(140, 18, 20, 28),
    borderColor: CanvasColor.fromARGB(40, 255, 255, 255),
  );
}

/// A node in the Canvas UI tree representing a rendered element.
class GlassNode {
  final String id;
  final LiquidGlassTheme style;
  final double width;
  final double height;
  final List<GlassNode> children;

  GlassNode({
    required this.id,
    this.style = const LiquidGlassTheme(),
    this.width = 0.0,
    this.height = 0.0,
    List<GlassNode>? children,
  }) : children = children ?? [];

  void addChild(GlassNode node) {
    children.add(node);
  }

  @override
  String toString() => 'GlassNode($id, children: ${children.length})';
}
