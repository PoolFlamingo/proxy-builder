import { useEffect, useMemo, useRef } from "react";
import { type Estilo, METALES } from "./aspecto";
import { entre, generador, giroAlAzar } from "./azar";
import type { PiezaTirada, Tirada } from "./cola";
import { esquinasD4, geometria, planoCara, textoCara, type V3 } from "./geometria";
import { caraDelResultado, tipoDe } from "./piezas";
import { CAOS, CARA, CRUZ, PLANESWALK } from "./simbolos";
import { golpeDado, golpeMoneda, lanzarMoneda } from "./sonidos";
import { LADO_FONDO, trozoDeCara } from "./texturas";

/**
 * **Los dados en CSS 3D, low poly y planos**: sin WebGL ni física. Las caras
 * son elementos recortados (`clip-path`) y colocados con `matrix3d`; la
 * tirada es una animación (Web Animations API) que da tumbos y acaba con la
 * cara del resultado mirando a quien juega. La luz es fija (plana): cada cara
 * con su tono según cómo queda al final. El aspecto (`Estilo`) pone el fondo
 * de las caras (`fondo`, del mismo lienzo que el 3D) y el color de los
 * números; el translúcido aquí es solo un poco de transparencia.
 */

type M3 = [number, number, number, number, number, number, number, number, number];

/** Píxeles por unidad de la geometría (los dados miden ~1,2). */
const ESCALA = 66;
const DURA = 1300;
/** Los momentos clave de la animación y la altura en cada uno: cae, bota y se queda. */
const MOMENTOS = [0, 0.4, 0.62, 0.8, 1];
const ALTO = [80, 0, 16, 0, 0];
/** Lo que se ha recorrido del camino en cada momento. */
const CAMINO = [0, 0.6, 0.82, 0.96, 1];
/** Los golpes: en qué momento y con qué fuerza. */
const GOLPES: [number, number][] = [
	[1, 0.85],
	[3, 0.4],
];
const EASING: [number, number, number, number] = [0.2, 0.7, 0.3, 1];

/**
 * El tiempo (0–1) en el que la animación llega a la posición `p` de sus
 * fotogramas con el `EASING`: la inversa de la curva, por bisección.
 */
function tiempoDe(p: number): number {
	const [x1, y1, x2, y2] = EASING;
	const curva = (a: number, b: number, s: number) =>
		3 * a * s * (1 - s) ** 2 + 3 * b * s * s * (1 - s) + s ** 3;
	let lo = 0;
	let hi = 1;
	for (let i = 0; i < 30; i++) {
		const m = (lo + hi) / 2;
		if (curva(y1, y2, m) < p) lo = m;
		else hi = m;
	}
	return curva(x1, x2, (lo + hi) / 2);
}

const deCuaternion = ([x, y, z, w]: number[]): M3 => [
	1 - 2 * (y * y + z * z),
	2 * (x * y - z * w),
	2 * (x * z + y * w),
	2 * (x * y + z * w),
	1 - 2 * (x * x + z * z),
	2 * (y * z - x * w),
	2 * (x * z - y * w),
	2 * (y * z + x * w),
	1 - 2 * (x * x + y * y),
];

const aplicar = (m: M3, v: V3): V3 => [
	m[0] * v[0] + m[1] * v[1] + m[2] * v[2],
	m[3] * v[0] + m[4] * v[1] + m[5] * v[2],
	m[6] * v[0] + m[7] * v[1] + m[8] * v[2],
];

/** Un giro de la geometría (Y arriba) a `matrix3d` de CSS (Y abajo): F·M·F. */
function css(m: M3): string {
	const c = [m[0], -m[1], m[2], -m[3], m[4], -m[5], m[6], -m[7], m[8]];
	// matrix3d va por columnas.
	return `matrix3d(${c[0]},${c[3]},${c[6]},0,${c[1]},${c[4]},${c[7]},0,${c[2]},${c[5]},${c[8]},0,0,0,0,1)`;
}

/**
 * El giro final: la cara del resultado de frente (en el d4, de espaldas, con
 * la punta hacia quien mira) y su «arriba» hacia arriba, con un poco de
 * ladeo. Filas = los ejes de la cara (w, u, n).
 */
function giroFinal(p: PiezaTirada, ladeo: number): M3 {
	const t = tipoDe(p) ?? 6;
	const { w, u, n } = planoCara(geometria(t), caraDelResultado(p));
	const s = t === 4 ? -1 : 1;
	const base: M3 = [s * w[0], s * w[1], s * w[2], u[0], u[1], u[2], s * n[0], s * n[1], s * n[2]];
	const c = Math.cos(ladeo);
	const si = Math.sin(ladeo);
	const rz: M3 = [c, -si, 0, si, c, 0, 0, 0, 1];
	const r: M3 = [0, 0, 0, 0, 0, 0, 0, 0, 0];
	for (let i = 0; i < 3; i++)
		for (let j = 0; j < 3; j++)
			r[i * 3 + j] =
				rz[i * 3] * base[j] + rz[i * 3 + 1] * base[3 + j] + rz[i * 3 + 2] * base[6 + j];
	return r;
}

const LUZ: V3 = (() => {
	const l: V3 = [-0.35, 0.55, 1];
	const n = Math.hypot(...l);
	return [l[0] / n, l[1] / n, l[2] / n];
})();

function Simbolo({ d, relleno, trazo }: { d: string; relleno: string; trazo: string }) {
	return (
		<svg viewBox="0 0 100 100" className="size-[62%]" aria-hidden>
			<path d={d} fill={relleno} stroke={trazo} strokeWidth={5} paintOrder="stroke" />
		</svg>
	);
}

/** Un dado: sus caras como elementos 3D. */
function Dado({
	p,
	estilo,
	fondo,
	final,
}: {
	p: PiezaTirada;
	estilo: Estilo;
	fondo: string;
	final: M3;
}) {
	const tinta = { relleno: estilo.numeros, trazo: estilo.contorno };
	const sombraTexto = `0 0 3px ${estilo.contorno}, 0 0 1px ${estilo.contorno}`;
	const t = tipoDe(p) ?? 6;
	const g = geometria(t);
	const valores = g.valores;
	return (
		<>
			{g.caras.map((_, i) => {
				const pl = planoCara(g, i);
				const d = pl.radio * 2 * ESCALA;
				const col = (v: V3) => [v[0], -v[1], v[2]];
				const [a, b, c] = [col(pl.w), col([-pl.u[0], -pl.u[1], -pl.u[2]]), col(pl.n)];
				const tr = col([pl.c[0] * ESCALA, pl.c[1] * ESCALA, pl.c[2] * ESCALA]);
				const m = `matrix3d(${a[0]},${a[1]},${a[2]},0,${b[0]},${b[1]},${b[2]},0,${c[0]},${c[1]},${c[2]},0,${tr[0]},${tr[1]},${tr[2]},1)`;
				// La caja de la cara es la del polígono (no un cuadrado mayor): las cajas
				// grandes, como las de las cometas del d10, se cruzan con los planos de
				// otras caras y Chromium, al partirlas para ordenarlas, perdía trozos.
				const xs = pl.puntos.map(([x]) => x);
				const ys = pl.puntos.map(([, y]) => y);
				const [x0, x1, y0, y1] = [
					Math.min(...xs),
					Math.max(...xs),
					Math.min(...ys),
					Math.max(...ys),
				];
				const poligono = pl.puntos
					.map(([x, y]) => `${((x - x0) / (x1 - x0)) * 100}% ${((y1 - y) / (y1 - y0)) * 100}%`)
					.join(",");
				const luz = Math.max(
					0,
					aplicar(final, pl.n).reduce((s, x, k) => s + x * LUZ[k], 0),
				);
				const sombra = 0.42 * (1 - luz);
				let dentro: React.ReactNode = null;
				if (p.forma === "planar") {
					dentro =
						valores[i] === 1 ? (
							<Simbolo d={PLANESWALK} {...tinta} />
						) : valores[i] === 6 ? (
							<Simbolo d={CAOS} {...tinta} />
						) : null;
				} else if (p.forma === "dado" && t === 4) {
					const esquinas = esquinasD4(g, valores, i);
					dentro = pl.puntos.map(([x, y], k) => (
						<span
							key={k}
							className="absolute font-bold"
							style={{
								left: `${50 + (x / (2 * pl.radio)) * 100 * 0.5}%`,
								top: `${50 - (y / (2 * pl.radio)) * 100 * 0.5}%`,
								fontSize: d * 0.17,
								transform: `translate(-50%,-50%) rotate(${Math.atan2(x, y)}rad)`,
								color: estilo.numeros,
								textShadow: sombraTexto,
							}}
						>
							{esquinas[k]}
						</span>
					));
				} else if (p.forma === "dado") {
					const texto = textoCara(t, valores[i], p.decenas);
					dentro = (
						<span
							className="font-bold"
							style={{
								fontSize:
									d *
									(t === 10 ? 0.2 : t >= 12 || t === 8 ? 0.26 : 0.34) *
									(texto.length > 2 ? 0.78 : 1),
								color: estilo.numeros,
								textShadow: sombraTexto,
							}}
						>
							{texto}
						</span>
					);
				}
				// El trozo del fondo de esta cara, en % de la imagen de fondo.
				const [fx, fy, fl] = trozoDeCara(estilo, i);
				const escala = (LADO_FONDO / fl) * 100;
				const posicion =
					fl >= LADO_FONDO
						? "center"
						: `${(fx / (LADO_FONDO - fl)) * 100}% ${(fy / (LADO_FONDO - fl)) * 100}%`;
				return (
					<div
						key={i}
						className="absolute"
						style={{
							width: (x1 - x0) * ESCALA,
							height: (y1 - y0) * ESCALA,
							left: x0 * ESCALA,
							top: -y1 * ESCALA,
							transformOrigin: `${-x0 * ESCALA}px ${y1 * ESCALA}px`,
							transform: m,
							clipPath: `polygon(${poligono})`,
							background: `linear-gradient(rgba(0,0,0,${sombra}),rgba(0,0,0,${sombra})), url(${fondo}) ${posicion} / ${escala}% ${escala}% no-repeat, ${estilo.color}`,
							opacity: estilo.acabado === "translucido" && !estilo.imagen ? 0.86 : undefined,
							backfaceVisibility: "hidden",
						}}
					>
						{/* Lo de dentro, centrado en el centro de la cara. */}
						<div
							className="absolute flex items-center justify-center"
							style={{ width: d, height: d, left: -x0 * ESCALA - d / 2, top: y1 * ESCALA - d / 2 }}
						>
							{dentro}
						</div>
					</div>
				);
			})}
		</>
	);
}

/** La moneda: dos caras planas, de frente al final la del resultado. */
function Moneda({ estilo }: { estilo: Estilo }) {
	const metal = METALES[estilo.moneda];
	const d = 76;
	const cara = (simbolo: string, atras: boolean) => (
		<div
			className="absolute flex items-center justify-center rounded-full"
			style={{
				width: d,
				height: d,
				left: -d / 2,
				top: -d / 2,
				background: `radial-gradient(circle at 40% 35%, ${metal.claro}, ${metal.oscuro})`,
				boxShadow: `inset 0 0 0 3px ${metal.borde}`,
				transform: atras ? "rotateX(180deg)" : undefined,
				backfaceVisibility: "hidden",
			}}
		>
			<Simbolo d={simbolo} relleno={metal.simbolo} trazo={metal.trazo} />
		</div>
	);
	return (
		<>
			{cara(CARA, false)}
			{cara(CRUZ, true)}
		</>
	);
}

/**
 * Una tirada en CSS 3D: las piezas entran desde abajo (el lado de quien
 * tira), dan tumbos y se quedan en el centro de la mesa.
 */
export function DadosCss({
	tirada,
	estilo,
	fondo,
	alAcabar,
	sonido = true,
}: {
	tirada: Tirada;
	estilo: Estilo;
	/** El fondo de las caras (`data:` URL de `texturas.urlFondo`). */
	fondo: string;
	alAcabar: () => void;
	sonido?: boolean;
}) {
	const caja = useRef<HTMLDivElement>(null);
	const acabar = useRef(alAcabar);
	acabar.current = alAcabar;
	const r = useMemo(() => generador(tirada.semilla), [tirada.semilla]);
	const piezas = useMemo(
		() =>
			tirada.piezas.map((p, i, todas) => {
				const porFila = Math.min(8, todas.length);
				const fila = Math.floor(i / porFila);
				const col = i % porFila;
				return {
					p,
					fin: {
						x: (col - (porFila - 1) / 2) * 108 + entre(r, -12, 12),
						y: fila * 108 - 40 + entre(r, -12, 12),
					},
					inicio: { x: entre(r, -120, 120), y: 340 },
					giros: [giroAlAzar(r), giroAlAzar(r), giroAlAzar(r)],
					vueltas: 3 + Math.floor(r() * 3),
					azarSonido: r(),
					final: p.forma === "moneda" ? null : giroFinal(p, entre(r, -0.25, 0.25)),
				};
			}),
		[tirada.piezas, r],
	);

	useEffect(() => {
		const raiz = caja.current;
		if (!raiz) return;
		const animaciones: Animation[] = [];
		piezas.forEach((x, i) => {
			const lugar = raiz.querySelector<HTMLElement>(`[data-lugar="${i}"]`);
			const pieza = raiz.querySelector<HTMLElement>(`[data-pieza="${i}"]`);
			const sombra = raiz.querySelector<HTMLElement>(`[data-sombra="${i}"]`);
			if (!lugar || !pieza || !sombra) return;
			const paso = (k: number) => ({
				x: x.inicio.x + (x.fin.x - x.inicio.x) * k,
				y: x.inicio.y + (x.fin.y - x.inicio.y) * k,
			});
			const puntos = CAMINO.map(paso);
			const moneda = x.p.forma === "moneda";
			// La moneda da vueltas sobre su eje y acaba de cara o de cruz.
			const total = x.vueltas * 360 + (x.p.forma === "moneda" && x.p.cara ? 0 : 180);
			const giro =
				x.final === null
					? [0, 0.62, 0.86, 1, 1].map((k) => `rotateX(${Math.round(k * total)}deg)`)
					: [...x.giros.map((q) => css(deCuaternion(q))), css(x.final), css(x.final)];
			const retraso = i * 45;
			const opciones: KeyframeAnimationOptions = {
				duration: DURA,
				delay: retraso,
				easing: `cubic-bezier(${EASING.join(",")})`,
				fill: "forwards",
			};
			animaciones.push(
				// Dónde está (en 2D: la altura, más arriba y más grande)…
				lugar.animate(
					puntos.map((pt, k) => ({
						transform: `translate(${pt.x}px,${pt.y - ALTO[k]}px) scale(${1 + ALTO[k] / 260})`,
						offset: MOMENTOS[k],
					})),
					opciones,
				),
				// …y cómo gira, con su propia perspectiva.
				pieza.animate(
					giro.map((g, k) => ({ transform: g, offset: MOMENTOS[k] })),
					opciones,
				),
				sombra.animate(
					puntos.map((pt, k) => ({
						transform: `translate(${pt.x + 8}px,${pt.y + 10}px) scale(${1 + ALTO[k] / 120})`,
						opacity: 0.45 - ALTO[k] / 300,
						offset: MOMENTOS[k],
					})),
					opciones,
				),
			);
			if (sonido) {
				if (moneda && i === 0) lanzarMoneda();
				for (const [m, fuerza] of GOLPES) {
					const s = (retraso + tiempoDe(MOMENTOS[m]) * DURA) / 1000;
					if (moneda) golpeMoneda(fuerza, s);
					else golpeDado(fuerza * (0.8 + 0.4 * x.azarSonido), s);
				}
			}
		});
		let vivo = true;
		void Promise.all(animaciones.map((a) => a.finished)).then(() => {
			if (vivo) acabar.current();
		});
		return () => {
			vivo = false;
			for (const a of animaciones) a.cancel();
		};
	}, [piezas, sonido]);

	return (
		<div ref={caja} className="absolute inset-0 flex items-center justify-center">
			{piezas.map((_, i) => (
				<div key={`s${i}`} data-sombra={i} className="absolute size-0">
					<div className="absolute -top-8 -left-10 h-16 w-20 rounded-full bg-black/60 blur-md" />
				</div>
			))}
			{piezas.map((x, i) => (
				// Cada pieza con su perspectiva centrada en ella: se ve igual caiga
				// donde caiga (con una sola para toda la mesa, los de los lados se
				// veían torcidos).
				<div
					key={i}
					data-lugar={i}
					className="absolute size-0"
					style={{ perspective: 700, perspectiveOrigin: "0 0" }}
				>
					<div data-pieza={i} className="absolute size-0" style={{ transformStyle: "preserve-3d" }}>
						{x.p.forma === "moneda" ? (
							<Moneda estilo={estilo} />
						) : (
							<Dado
								p={x.p}
								estilo={estilo}
								fondo={fondo}
								final={x.final ?? deCuaternion([0, 0, 0, 1])}
							/>
						)}
					</div>
				</div>
			))}
		</div>
	);
}
