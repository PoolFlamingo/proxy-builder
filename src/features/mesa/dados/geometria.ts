/**
 * **La geometría de los dados**, pura (sin three ni el DOM): vértices,
 * caras, normales y el número de cada cara. La usan los tres niveles de
 * dados (three.js con física, CSS 3D y 2D) y la física.
 *
 * El resultado lo decide el motor (Rust) antes de lanzar. La física solo dice
 * qué cara quedó arriba; luego se **cambian los números** de las caras para
 * que esa lleve el del motor (`conResultado`). Así todas las pantallas
 * enseñan el mismo número aunque la física rebote algo distinto.
 *
 * Sin `Math.sin`/`Math.cos` en lo que decide la tirada (las semillas y los
 * empujes, en `azar.ts`): las geometrías sí los usan, pero dan lo mismo en
 * todas partes porque solo dependen de constantes.
 */

export type V3 = [number, number, number];

/** Los dados que se pintan (el d100 son dos d10: decenas y unidades). */
export type TipoDado = 4 | 6 | 8 | 10 | 12 | 20;

export type Geometria = {
	tipo: TipoDado;
	vertices: V3[];
	/** Índices de los vértices de cada cara, en sentido antihorario visto desde fuera. */
	caras: number[][];
	/** La normal de cada cara, hacia fuera y unitaria. */
	normales: V3[];
	/** El número de cada cara (en el d4, el de la cara que queda **abajo**). */
	valores: number[];
};

const PHI = (1 + Math.sqrt(5)) / 2;

export const resta = (a: V3, b: V3): V3 => [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
export const suma = (a: V3, b: V3): V3 => [a[0] + b[0], a[1] + b[1], a[2] + b[2]];
export const por = (a: V3, k: number): V3 => [a[0] * k, a[1] * k, a[2] * k];
export const punto = (a: V3, b: V3) => a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
export const cruz = (a: V3, b: V3): V3 => [
	a[1] * b[2] - a[2] * b[1],
	a[2] * b[0] - a[0] * b[2],
	a[0] * b[1] - a[1] * b[0],
];
export const largo = (a: V3) => Math.sqrt(punto(a, a));
export const unidad = (a: V3): V3 => por(a, 1 / (largo(a) || 1));

/** El centro de una cara. */
export function centro(g: Geometria, cara: number): V3 {
	const vs = g.caras[cara].map((i) => g.vertices[i]);
	return por(vs.reduce(suma, [0, 0, 0] as V3), 1 / vs.length);
}

/** Ordena los vértices de una cara en sentido antihorario alrededor de su normal. */
function ordenar(vertices: V3[], idx: number[], n: V3): number[] {
	const c = por(idx.map((i) => vertices[i]).reduce(suma, [0, 0, 0] as V3), 1 / idx.length);
	const u = unidad(resta(vertices[idx[0]], c));
	const v = cruz(n, u);
	return [...idx].sort((a, b) => {
		const pa = resta(vertices[a], c);
		const pb = resta(vertices[b], c);
		return Math.atan2(punto(pa, v), punto(pa, u)) - Math.atan2(punto(pb, v), punto(pb, u));
	});
}

/** Un poliedro de un dado regular: cada cara, los vértices más lejanos en su dirección. */
function poliedro(tipo: TipoDado, vertices: V3[], direcciones: V3[]): Geometria {
	const normales = direcciones.map(unidad);
	const caras = normales.map((n) => {
		const d = vertices.map((v) => punto(v, n));
		const max = Math.max(...d);
		const idx = d.flatMap((x, i) => (Math.abs(x - max) < 1e-6 ? [i] : []));
		return ordenar(vertices, idx, n);
	});
	return { tipo, vertices: escalar(vertices), caras, normales, valores: numerar(normales) };
}

/** Todos los dados del mismo tamaño: el vértice más lejano, a 0,62 del centro. */
function escalar(vertices: V3[]): V3[] {
	const r = Math.max(...vertices.map(largo));
	return vertices.map((v) => por(v, 0.62 / r));
}

/**
 * Los números, como en los dados de verdad: caras opuestas suman `n + 1`.
 * Sin cara opuesta (el d4), en orden.
 */
function numerar(normales: V3[]): number[] {
	const n = normales.length;
	const valores = new Array<number>(n).fill(0);
	let siguiente = 1;
	for (let i = 0; i < n; i++) {
		if (valores[i]) continue;
		const op = normales.findIndex(
			(m, j) => j !== i && !valores[j] && punto(m, normales[i]) < -0.999,
		);
		valores[i] = siguiente;
		if (op >= 0) valores[op] = n + 1 - siguiente;
		siguiente++;
	}
	return valores;
}

function d4(): Geometria {
	const v: V3[] = [
		[1, 1, 1],
		[-1, -1, 1],
		[-1, 1, -1],
		[1, -1, -1],
	];
	// Cada cara mira al lado contrario de su vértice opuesto.
	return poliedro(
		4,
		v,
		v.map((x) => por(x, -1)),
	);
}

function d6(): Geometria {
	const v: V3[] = [];
	for (const x of [-1, 1]) for (const y of [-1, 1]) for (const z of [-1, 1]) v.push([x, y, z]);
	return poliedro(6, v, [
		[1, 0, 0],
		[-1, 0, 0],
		[0, 1, 0],
		[0, -1, 0],
		[0, 0, 1],
		[0, 0, -1],
	]);
}

function d8(): Geometria {
	const v: V3[] = [
		[1, 0, 0],
		[-1, 0, 0],
		[0, 1, 0],
		[0, -1, 0],
		[0, 0, 1],
		[0, 0, -1],
	];
	const dirs: V3[] = [];
	for (const x of [-1, 1]) for (const y of [-1, 1]) for (const z of [-1, 1]) dirs.push([x, y, z]);
	return poliedro(8, v, dirs);
}

function icosaedro(): V3[] {
	const v: V3[] = [];
	for (const a of [-1, 1])
		for (const b of [-PHI, PHI]) {
			v.push([0, a, b]);
			v.push([a, b, 0]);
			v.push([b, 0, a]);
		}
	return v;
}

function dodecaedro(): V3[] {
	const v: V3[] = [];
	for (const x of [-1, 1]) for (const y of [-1, 1]) for (const z of [-1, 1]) v.push([x, y, z]);
	for (const a of [-1 / PHI, 1 / PHI])
		for (const b of [-PHI, PHI]) {
			v.push([0, a, b]);
			v.push([a, b, 0]);
			v.push([b, 0, a]);
		}
	return v;
}

/**
 * El d12 y el d20 son duales: las caras de uno miran a los vértices del
 * otro, con las coordenadas en el orden que toque (se prueban los seis).
 */
function dual(tipo: TipoDado, vertices: V3[], otro: V3[], lados: number): Geometria {
	const giros = [
		(v: V3): V3 => v,
		(v: V3): V3 => [v[1], v[2], v[0]],
		(v: V3): V3 => [v[2], v[0], v[1]],
		(v: V3): V3 => [v[0], v[2], v[1]],
		(v: V3): V3 => [v[2], v[1], v[0]],
		(v: V3): V3 => [v[1], v[0], v[2]],
	];
	for (const girar of giros) {
		const g = poliedro(tipo, vertices, otro.map(girar));
		if (g.caras.every((c) => c.length === lados)) return g;
	}
	return poliedro(tipo, vertices, otro);
}
const d12 = (): Geometria => dual(12, dodecaedro(), icosaedro(), 5);
const d20 = (): Geometria => dual(20, icosaedro(), dodecaedro(), 3);

/**
 * El d10: trapezoedro pentagonal. Diez vértices en un anillo, alternando
 * arriba y abajo (`b`), y dos puntas a altura `h`. Para que cada cometa sea
 * plana, `h = b (1 + cos 36°) / (1 − cos 36°)`.
 */
function d10(): Geometria {
	const c36 = Math.cos(Math.PI / 5);
	const b = 0.105;
	const h = (b * (1 + c36)) / (1 - c36);
	const anillo: V3[] = Array.from({ length: 10 }, (_, k) => {
		const a = (k * Math.PI) / 5;
		return [Math.cos(a), k % 2 === 0 ? b : -b, Math.sin(a)];
	});
	const vertices: V3[] = [...anillo, [0, h, 0], [0, -h, 0]];
	const arriba = 10;
	const abajo = 11;
	const caras: number[][] = [];
	for (let i = 0; i < 5; i++) {
		caras.push([arriba, (2 * i) % 10, (2 * i + 1) % 10, (2 * i + 2) % 10]);
		caras.push([abajo, (2 * i + 1) % 10, (2 * i + 2) % 10, (2 * i + 3) % 10]);
	}
	const normales = caras.map((cara) => {
		const [p0, p1, p2] = cara.map((i) => vertices[i]);
		let n = unidad(cruz(resta(p1, p0), resta(p2, p0)));
		const cen = por(cara.map((i) => vertices[i]).reduce(suma, [0, 0, 0] as V3), 1 / cara.length);
		if (punto(n, cen) < 0) n = por(n, -1);
		return n;
	});
	const ordenadas = caras.map((cara, i) => ordenar(vertices, cara, normales[i]));
	// 0–9: caras opuestas suman 9 (como los de verdad).
	const valores = numerar(normales).map((v) => v - 1);
	return { tipo: 10, vertices: escalar(vertices), caras: ordenadas, normales, valores };
}

const HECHAS = new Map<TipoDado, Geometria>();

/** La geometría de un dado (la misma cada vez: no se toca). */
export function geometria(tipo: TipoDado): Geometria {
	let g = HECHAS.get(tipo);
	if (!g) {
		g = { 4: d4, 6: d6, 8: d8, 10: d10, 12: d12, 20: d20 }[tipo]();
		HECHAS.set(tipo, g);
	}
	return g;
}

/** El d4 se lee por la cara de abajo; los demás, por la de arriba. */
export const seLeeAbajo = (tipo: TipoDado) => tipo === 4;

/**
 * La cara que se lee con el dado girado por `rotar` (de local a mundo):
 * la que más mira hacia arriba (+Y), o hacia abajo en el d4. Devuelve
 * también cuánto mira (1 = plana; menos de 0,9, el dado se quedó de canto).
 */
export function caraLeida(g: Geometria, rotar: (v: V3) => V3): { cara: number; plana: number } {
	let mejor = 0;
	let y = Number.NEGATIVE_INFINITY;
	const signo = seLeeAbajo(g.tipo) ? -1 : 1;
	g.normales.forEach((n, i) => {
		const yy = rotar(n)[1] * signo;
		if (yy > y) {
			y = yy;
			mejor = i;
		}
	});
	return { cara: mejor, plana: y };
}

/**
 * Los números con los que hay que pintar el dado para que `cara` (la que
 * quedó arriba) muestre `valor`: se intercambia con la cara que lo tenía.
 */
export function conResultado(g: Geometria, cara: number, valor: number): number[] {
	const valores = [...g.valores];
	const tenia = valores.indexOf(valor);
	if (tenia < 0 || tenia === cara) return valores;
	[valores[cara], valores[tenia]] = [valores[tenia], valores[cara]];
	return valores;
}

/**
 * En el d4, el número de cada esquina de una cara: el de la cara opuesta a
 * ese vértice (la que queda abajo cuando el vértice apunta arriba).
 */
export function esquinasD4(g: Geometria, valores: number[], cara: number): number[] {
	return g.caras[cara].map((v) => {
		const opuesta = g.caras.findIndex((c) => !c.includes(v));
		return valores[opuesta] ?? 0;
	});
}

/** Lo que se escribe en una cara (el d10 de decenas lleva «00», «10»…). */
export function textoCara(tipo: TipoDado, valor: number, decenas = false): string {
	if (decenas) return `${valor}0`;
	// El 6 y el 9 llevan punto, como en los dados de verdad, para no confundirlos.
	return (valor === 6 || valor === 9) && tipo >= 8 ? `${valor}.` : String(valor);
}

/** Los dados que se pintan para una tirada de `caras` con `valor` (el d100, dos d10). */
export function dadosDeTirada(
	caras: number,
	valor: number,
): { tipo: TipoDado; valor: number; decenas?: boolean }[] {
	if (caras === 100) {
		// 100 = «00» y «0»; si no, decenas y unidades.
		const v = valor % 100;
		return [
			{ tipo: 10, valor: Math.floor(v / 10), decenas: true },
			{ tipo: 10, valor: v % 10 },
		];
	}
	if (caras === 10) return [{ tipo: 10, valor: valor % 10 }];
	const tipo = ([4, 6, 8, 12, 20] as const).find((t) => t === caras);
	// Un dado sin modelo (d3, d2…): se pinta un d6 con el número del motor.
	return [{ tipo: tipo ?? 6, valor: tipo ? valor : Math.min(6, Math.max(1, valor)) }];
}

/**
 * Una cara en su propio plano: el centro `c`, los ejes `u` (hacia el medio de
 * la primera arista: «arriba» del número) y `w` (`u × n`, «derecha») y sus
 * vértices en esas coordenadas, con `radio` el más lejano.
 */
export function planoCara(g: Geometria, i: number) {
	const c = centro(g, i);
	const n = g.normales[i];
	const [a, b] = g.caras[i].map((k) => g.vertices[k]);
	const u = unidad(resta(por(suma(a, b), 0.5), c));
	// w = u × n: con u «arriba» y n hacia quien mira, w queda a la derecha.
	const w = cruz(u, n);
	const puntos = g.caras[i].map((k) => {
		const p = resta(g.vertices[k], c);
		return [punto(p, w), punto(p, u)] as [number, number];
	});
	const radio = Math.max(...puntos.map(([x, y]) => Math.hypot(x, y)));
	return { c, n, u, w, puntos, radio };
}
