/**
 * **Geometría de la mesa en pantalla.** Dado el tamaño del tablero, dónde
 * va cada zona (campo, mano, biblioteca, cementerio…). Es solo de pantalla:
 * dónde está cada carta en el campo lo guarda el motor (`Pos`, 0–10.000) y
 * aquí solo se convierte a píxeles.
 *
 * `repartir` (Fase G) reparte varios tableros en la pantalla; `disponer`
 * reparte las zonas dentro de uno.
 */

export type Rect = { x: number; y: number; w: number; h: number };

/** Proporción de una carta (63 × 88 mm). */
export const PROPORCION = 63 / 88;
/** Lo más lejos que llega una posición del motor en cada eje. */
export const POS_MAX = 10_000;

export type ZonaPantalla = "biblioteca" | "cementerio" | "exilio" | "mando" | "banquillo";

export type Disposicion = {
	tablero: Rect;
	campo: Rect;
	mano: Rect;
	zonas: Record<ZonaPantalla, Rect>;
	/** Medida de una carta del campo (sin girar). */
	carta: { w: number; h: number };
	/** Medida de una carta de la mano y de las zonas. */
	cartaPequena: { w: number; h: number };
};

/**
 * Reparte el tablero: el campo arriba; abajo, la mano a la izquierda y las
 * zonas a la derecha. `tamano` es el de Ajustes > Mesa (25–200 %).
 */
export function disponer(w: number, h: number, tamano = 100, conBanquillo = false): Disposicion {
	const escala = Math.max(0.25, Math.min(2, tamano / 100));
	// La franja de abajo: un 22 % del alto, entre 90 y 220 px.
	const franja = Math.round(Math.max(90, Math.min(220, h * 0.22)));
	const ch = franja - 26;
	const cw = Math.round(ch * PROPORCION);
	const campo: Rect = { x: 0, y: 0, w, h: Math.max(0, h - franja) };
	const nZonas = conBanquillo ? 5 : 4;
	const hueco = 8;
	const anchoZonas = nZonas * (cw + hueco) + hueco;
	const yZonas = h - franja + 8;
	const nombres: ZonaPantalla[] = conBanquillo
		? ["mando", "banquillo", "biblioteca", "cementerio", "exilio"]
		: ["mando", "biblioteca", "cementerio", "exilio"];
	const zonas = {} as Record<ZonaPantalla, Rect>;
	nombres.forEach((n, i) => {
		zonas[n] = { x: w - anchoZonas + hueco + i * (cw + hueco), y: yZonas, w: cw, h: ch };
	});
	if (!conBanquillo) zonas.banquillo = { x: -1000, y: -1000, w: 0, h: 0 };
	const mano: Rect = { x: 8, y: h - franja, w: Math.max(0, w - anchoZonas - 16), h: franja };
	// En el campo, la carta se ve algo más grande que en la franja.
	const fh = Math.round(Math.min(campo.h * 0.24, 240) * escala);
	return {
		tablero: { x: 0, y: 0, w, h },
		campo,
		mano,
		zonas,
		carta: { w: Math.round(fh * PROPORCION), h: fh },
		cartaPequena: { w: cw, h: ch },
	};
}

/** El centro en píxeles de una posición del motor dentro del campo. */
export function posAPx(campo: Rect, p: { x: number; y: number }): { x: number; y: number } {
	return {
		x: campo.x + (p.x / POS_MAX) * campo.w,
		y: campo.y + (p.y / POS_MAX) * campo.h,
	};
}

/** La posición del motor de un punto del campo (recortada al campo). */
export function pxAPos(campo: Rect, x: number, y: number): { x: number; y: number } {
	const rx = campo.w > 0 ? (x - campo.x) / campo.w : 0;
	const ry = campo.h > 0 ? (y - campo.y) / campo.h : 0;
	const recorta = (v: number) => Math.round(Math.max(0, Math.min(1, v)) * POS_MAX);
	return { x: recorta(rx), y: recorta(ry) };
}

/** ¿Está el punto dentro del rectángulo? */
export function dentro(r: Rect, x: number, y: number): boolean {
	return x >= r.x && x <= r.x + r.w && y >= r.y && y <= r.y + r.h;
}

/**
 * Las posiciones de las cartas de la mano en abanico: el centro de cada una
 * y su giro. Si no caben, se solapan.
 */
export function abanico(
	mano: Rect,
	n: number,
	carta: { w: number; h: number },
): { x: number; y: number; giro: number }[] {
	if (n === 0) return [];
	const paso = n > 1 ? Math.min(carta.w * 0.92, (mano.w - carta.w) / (n - 1)) : 0;
	const total = paso * (n - 1) + carta.w;
	const x0 = mano.x + (mano.w - total) / 2 + carta.w / 2;
	const mitad = (n - 1) / 2;
	return Array.from({ length: n }, (_, i) => {
		const d = n > 1 ? (i - mitad) / Math.max(mitad, 1) : 0;
		return {
			x: x0 + i * paso,
			y: mano.y + mano.h / 2 + Math.abs(d) * 6,
			giro: d * Math.min(8, n),
		};
	});
}

// ── Varios tableros (Fase G) ──────────────────────────────────────────────

/**
 * Cómo se reparten los tableros:
 * - `unico`: se ve uno y los chips de jugador cambian de tablero.
 * - `dividido`: dos tableros, uno al lado del otro o uno encima del otro.
 * - `foco`: uno grande y los demás en miniatura en una tira.
 * - `rejilla`: todos iguales.
 * - `equipos`: rejilla con los compañeros juntos.
 */
export type ModoDisposicion = "unico" | "dividido" | "foco" | "rejilla" | "equipos";

export type LadoTira = "arriba" | "derecha" | "abajo";

export type ConfigDisposicion = {
	modo: ModoDisposicion;
	/** Dónde va la tira de miniaturas en «foco». */
	tira: LadoTira;
	/** En «dividido», la parte que se lleva el primero (0,2–0,8). */
	proporcion: number;
	/** En «dividido», uno al lado del otro (vertical) o uno encima del otro. */
	vertical: boolean;
};

export const DISPOSICION_DE_FABRICA: ConfigDisposicion = {
	modo: "unico",
	tira: "abajo",
	proporcion: 0.7,
	vertical: true,
};

/** Los modos que tienen sentido con `n` tableros (y, el de equipos, solo si los hay). */
export function modosPara(n: number, conEquipos = false): ModoDisposicion[] {
	if (n <= 1) return ["unico"];
	if (n === 2) return ["unico", "dividido", "foco", "rejilla"];
	return conEquipos ? ["unico", "foco", "rejilla", "equipos"] : ["unico", "foco", "rejilla"];
}

/** Hueco entre tableros, en píxeles. */
export const HUECO = 4;
/** La parte más pequeña que se le deja a un lado del «dividido». */
export const PROPORCION_MIN = 0.2;
export const PROPORCION_MAX = 0.8;

export function limitarProporcion(p: number): number {
	return Math.max(PROPORCION_MIN, Math.min(PROPORCION_MAX, p));
}

/** Parte `total` píxeles en `n` trozos separados por `HUECO`, sin pasarse ni solapar. */
function trozos(total: number, n: number): { inicio: number; largo: number }[] {
	const libre = Math.max(0, total - HUECO * (n - 1));
	const base = Math.floor(libre / n);
	return Array.from({ length: n }, (_, i) => ({ inicio: i * (base + HUECO), largo: base }));
}

/** La rejilla de `n` celdas dentro de `r`: la que deja las celdas más grandes. */
function rejilla(n: number, r: Rect): Rect[] {
	let mejor = { cols: 1, area: -1 };
	for (let cols = 1; cols <= n; cols++) {
		const filas = Math.ceil(n / cols);
		const w = (r.w - HUECO * (cols - 1)) / cols;
		const h = (r.h - HUECO * (filas - 1)) / filas;
		const area = Math.max(0, w) * Math.max(0, h);
		if (area > mejor.area) mejor = { cols, area };
	}
	const cols = mejor.cols;
	const filas = Math.ceil(n / cols);
	const xs = trozos(r.w, cols);
	const ys = trozos(r.h, filas);
	const celdas: Rect[] = [];
	for (let i = 0; i < n; i++) {
		const fila = Math.floor(i / cols);
		const enFila = fila === filas - 1 ? n - fila * cols : cols;
		// La última fila, si no está llena, se centra.
		const desvio = Math.floor(((cols - enFila) * (xs[0].largo + HUECO)) / 2);
		const x = xs[i % cols];
		const y = ys[fila];
		celdas.push({
			x: r.x + x.inicio + (fila === filas - 1 ? desvio : 0),
			y: r.y + y.inicio,
			w: x.largo,
			h: y.largo,
		});
	}
	return celdas;
}

/**
 * Reparte `n` tableros en `r`. El resultado tiene un rectángulo por tablero,
 * en el mismo orden (el 0 es el propio). `grande` es el tablero que se ve
 * grande en «foco» (por defecto, el propio); `equipos`, el equipo de cada
 * tablero (`null`: sin equipo) para el modo «equipos».
 *
 * Es solo geometría de pantalla: nunca se solapan ni se salen de `r`.
 */
export function repartir(
	n: number,
	r: Rect,
	c: ConfigDisposicion,
	grande = 0,
	equipos: (number | null)[] = [],
): Rect[] {
	if (n <= 0) return [];
	if (n === 1 || c.modo === "unico") return [r, ...Array.from({ length: n - 1 }, () => VACIO)];
	const modo = c.modo === "dividido" && n !== 2 ? "rejilla" : c.modo;
	switch (modo) {
		case "dividido": {
			const p = limitarProporcion(c.proporcion);
			if (c.vertical) {
				const libre = r.w - HUECO;
				const a = Math.floor(libre * p);
				return [
					{ x: r.x, y: r.y, w: a, h: r.h },
					{ x: r.x + a + HUECO, y: r.y, w: libre - a, h: r.h },
				];
			}
			const libre = r.h - HUECO;
			const a = Math.floor(libre * p);
			return [
				{ x: r.x, y: r.y, w: r.w, h: a },
				{ x: r.x, y: r.y + a + HUECO, w: r.w, h: libre - a },
			];
		}
		case "foco": {
			const g = Math.max(0, Math.min(n - 1, grande));
			const otros = Array.from({ length: n }, (_, i) => i).filter((i) => i !== g);
			const salida: Rect[] = new Array(n);
			const horizontal = c.tira === "arriba" || c.tira === "abajo";
			const lado = horizontal ? r.h : r.w;
			// La tira nunca se come más del 40 % de la pantalla.
			const grosor = Math.floor(
				Math.min(
					lado * 0.4,
					horizontal
						? Math.max(60, Math.min(160, r.h * 0.22))
						: Math.max(100, Math.min(280, r.w * 0.22)),
				),
			);
			let cuerpo: Rect;
			let tira: Rect;
			if (c.tira === "arriba") {
				tira = { x: r.x, y: r.y, w: r.w, h: grosor };
				cuerpo = { x: r.x, y: r.y + grosor + HUECO, w: r.w, h: r.h - grosor - HUECO };
			} else if (c.tira === "abajo") {
				cuerpo = { x: r.x, y: r.y, w: r.w, h: r.h - grosor - HUECO };
				tira = { x: r.x, y: r.y + cuerpo.h + HUECO, w: r.w, h: grosor };
			} else {
				cuerpo = { x: r.x, y: r.y, w: r.w - grosor - HUECO, h: r.h };
				tira = { x: r.x + cuerpo.w + HUECO, y: r.y, w: grosor, h: r.h };
			}
			salida[g] = cuerpo;
			const partes = trozos(horizontal ? tira.w : tira.h, otros.length);
			otros.forEach((i, k) => {
				salida[i] = horizontal
					? { x: tira.x + partes[k].inicio, y: tira.y, w: partes[k].largo, h: tira.h }
					: { x: tira.x, y: tira.y + partes[k].inicio, w: tira.w, h: partes[k].largo };
			});
			return salida;
		}
		case "equipos": {
			// Los compañeros juntos: primero el equipo del propio y luego los demás
			// por equipo; los que no tienen equipo, al final, cada uno por su orden.
			const mio = equipos[0] ?? null;
			// El propio primero; luego sus compañeros; luego los demás equipos por
			// número; al final, los que no tienen equipo, cada uno por su orden.
			const clave = (i: number): number => {
				if (i === 0) return -1e9;
				const e = equipos[i] ?? null;
				if (e === null) return 1e6 + i;
				return e === mio ? -1e6 + i : e * 100 + i;
			};
			const orden = Array.from({ length: n }, (_, i) => i).sort(
				(a, b) => clave(a) - clave(b) || a - b,
			);
			const celdas = rejilla(n, r);
			const salida: Rect[] = new Array(n);
			orden.forEach((tablero, k) => {
				salida[tablero] = celdas[k];
			});
			return salida;
		}
		default:
			return rejilla(n, r);
	}
}

/** Un rectángulo sin área, para los tableros que no se ven. */
export const VACIO: Rect = { x: 0, y: 0, w: 0, h: 0 };

/** La disposición con que empieza una partida de `n` jugadores. */
export function disposicionPorDefecto(n: number): ConfigDisposicion {
	return { ...DISPOSICION_DE_FABRICA, modo: n <= 1 ? "unico" : n === 2 ? "dividido" : "foco" };
}

/** La que vale para `n` jugadores: la guardada si sirve, y si no, la de fábrica. */
export function disposicionDe(
	guardadas: Record<number, ConfigDisposicion> | undefined,
	n: number,
	conEquipos = false,
): ConfigDisposicion {
	const g = guardadas?.[n];
	if (!g || !modosPara(n, conEquipos).includes(g.modo)) return disposicionPorDefecto(n);
	return { ...DISPOSICION_DE_FABRICA, ...g, proporcion: limitarProporcion(g.proporcion) };
}

/**
 * El orden en que se reparten los tableros. En red, el propio primero y los
 * demás por asiento a partir de él; en una partida local de varios, el de los
 * asientos tal cual, para que al cambiar de jugador los tableros no salten.
 */
export function ordenTableros(asientos: number[], yo: number | null, local: boolean): number[] {
	const i = yo === null ? -1 : asientos.indexOf(yo);
	if (local || i <= 0) return [...asientos];
	return [...asientos.slice(i), ...asientos.slice(0, i)];
}

/** La proporción que corresponde a un puntero en `(x, y)` al arrastrar el separador de «dividido». */
export function proporcionPorPuntero(r: Rect, x: number, y: number, vertical: boolean): number {
	const largo = (vertical ? r.w : r.h) - HUECO;
	if (largo <= 0) return DISPOSICION_DE_FABRICA.proporcion;
	return limitarProporcion(((vertical ? x - r.x : y - r.y) - HUECO / 2) / largo);
}
