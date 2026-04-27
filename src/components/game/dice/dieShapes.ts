/**
 * Per-die-type 2D silhouettes for `DieFaceStatic`.
 *
 * Polygons are expressed in a 100×100 SVG viewBox and are designed to
 * be visually balanced around (50, 50) so the always-centered numeral
 * sits at the optical centre of every silhouette.
 */

export function getDiePoints(sides: number): string {
  switch (sides) {
    case 4:
      // Equilateral triangle balanced around y=50.
      return "50,15 91,75 9,75";
    case 6:
      return "14,14 86,14 86,86 14,86";
    case 8:
      return "50,6 94,50 50,94 6,50";
    case 10:
      // Symmetric kite balanced around y=50.
      return "50,8 86,40 50,92 14,40";
    case 12:
      // Regular pentagon balanced around y=50.
      return "50,10 92,40 76,88 24,88 8,40";
    case 20:
      // Regular hexagon (point-up).
      return "50,8 89,30 89,70 50,92 11,70 11,30";
    default:
      return "12,12 88,12 88,88 12,88";
  }
}
