/**
 * Los símbolos que no son números, como trazados SVG en una caja de 100 × 100
 * (centrados en 50, 50). Sirven igual para pintar en un `canvas` (con
 * `Path2D`) que en un `<svg>` (CSS 3D y 2D).
 */

/** Planeswalk: el trazo vertical con dos alas, a la manera del símbolo de los planeswalkers. */
export const PLANESWALK =
	"M50 6 L56 60 L50 94 L44 60 Z M20 22 C30 40 36 52 40 74 L34 74 C30 56 26 42 20 22 Z M80 22 C70 40 64 52 60 74 L66 74 C70 56 74 42 80 22 Z";

/** Caos: una espiral de seis brazos (el símbolo del caos del Planechase, simplificado). */
export const CAOS =
	"M50 50 m-6 0 a6 6 0 1 0 12 0 a6 6 0 1 0 -12 0 M50 8 L56 36 L50 44 L44 36 Z M50 92 L44 64 L50 56 L56 64 Z M14 29 L40 41 L41 51 L32 46 Z M86 71 L60 59 L59 49 L68 54 Z M14 71 L32 54 L41 49 L40 59 Z M86 29 L68 46 L59 51 L60 41 Z";

/** Cara de la moneda: una corona. */
export const CARA =
	"M18 70 L22 34 L38 52 L50 26 L62 52 L78 34 L82 70 Z M18 76 L82 76 L82 84 L18 84 Z";

/** Cruz de la moneda: una cruz griega. */
export const CRUZ =
	"M42 16 L58 16 L58 42 L84 42 L84 58 L58 58 L58 84 L42 84 L42 58 L16 58 L16 42 L42 42 Z";
