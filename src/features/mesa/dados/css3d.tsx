import { useEffect, useMemo, useRef } from "react";
import { entre, generador, giroAlAzar } from "./azar";
import type { PiezaTirada, Tirada } from "./cola";
import { esquinasD4, geometria, planoCara, textoCara, type V3 } from "./geometria";
import { caraDelResultado, tipoDe } from "./piezas";
import { CAOS, CARA, CRUZ, PLANESWALK } from "./simbolos";

/**
 * **Los dados en CSS 3D, low poly y planos**: sin WebGL ni física. Las caras
 * son elementos recortados (`clip-path`) y colocados con `matrix3d`; la
 * tirada es una animación (Web Animations API) que da tumbos y acaba con la
 * cara del resultado mirando a quien juega. La luz es fija (plana): cada cara
 * con su tono según cómo queda al final.
 */

type M3 = [number, number, number, number, number, number, number, number, number];

/** Píxeles por unidad de la geometría (los dados miden ~1,2). */
const ESCALA = 66;
const DURA = 1300;

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

function Simbolo({ d }: { d: string }) {
	return (
		<svg viewBox="0 0 100 100" className="size-[62%]" aria-hidden>
			<path d={d} fill="#fff" stroke="rgba(0,0,0,.55)" strokeWidth={5} paintOrder="stroke" />
		</svg>
	);
}

/** Un dado: sus caras como elementos 3D. */
function Dado({ p, color, final }: { p: PiezaTirada; color: string; final: M3 }) {
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
				const poligono = pl.puntos
					.map(
						([x, y]) => `${50 + (x / (2 * pl.radio)) * 100}% ${50 - (y / (2 * pl.radio)) * 100}%`,
					)
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
							<Simbolo d={PLANESWALK} />
						) : valores[i] === 6 ? (
							<Simbolo d={CAOS} />
						) : null;
				} else if (p.forma === "dado" && t === 4) {
					const esquinas = esquinasD4(g, valores, i);
					dentro = pl.puntos.map(([x, y], k) => (
						<span
							key={k}
							className="absolute font-bold text-white"
							style={{
								left: `${50 + (x / (2 * pl.radio)) * 100 * 0.5}%`,
								top: `${50 - (y / (2 * pl.radio)) * 100 * 0.5}%`,
								fontSize: d * 0.17,
								transform: `translate(-50%,-50%) rotate(${Math.atan2(x, y)}rad)`,
								textShadow: "0 0 3px rgba(0,0,0,.6)",
							}}
						>
							{esquinas[k]}
						</span>
					));
				} else if (p.forma === "dado") {
					const texto = textoCara(t, valores[i], p.decenas);
					dentro = (
						<span
							className="font-bold text-white"
							style={{
								fontSize:
									d *
									(t === 10 ? 0.2 : t >= 12 || t === 8 ? 0.26 : 0.34) *
									(texto.length > 2 ? 0.78 : 1),
								textShadow: "0 0 3px rgba(0,0,0,.6)",
							}}
						>
							{texto}
						</span>
					);
				}
				return (
					<div
						key={i}
						className="absolute flex items-center justify-center"
						style={{
							width: d,
							height: d,
							left: -d / 2,
							top: -d / 2,
							transform: m,
							clipPath: `polygon(${poligono})`,
							background: `linear-gradient(rgba(0,0,0,${sombra}),rgba(0,0,0,${sombra})), ${color}`,
							backfaceVisibility: "hidden",
						}}
					>
						{dentro}
					</div>
				);
			})}
		</>
	);
}

/** La moneda: dos caras planas, de frente al final la del resultado. */
function Moneda() {
	const d = 76;
	const cara = (simbolo: string, atras: boolean) => (
		<div
			className="absolute flex items-center justify-center rounded-full"
			style={{
				width: d,
				height: d,
				left: -d / 2,
				top: -d / 2,
				background: "radial-gradient(circle at 40% 35%, #f8d77a, #b8862b)",
				boxShadow: "inset 0 0 0 3px #9a6f1f",
				transform: atras ? "rotateX(180deg)" : undefined,
				backfaceVisibility: "hidden",
			}}
		>
			<Simbolo d={simbolo} />
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
	color,
	alAcabar,
}: {
	tirada: Tirada;
	color: string;
	alAcabar: () => void;
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
			const pieza = raiz.querySelector<HTMLElement>(`[data-pieza="${i}"]`);
			const sombra = raiz.querySelector<HTMLElement>(`[data-sombra="${i}"]`);
			if (!pieza || !sombra) return;
			const paso = (k: number) => ({
				x: x.inicio.x + (x.fin.x - x.inicio.x) * k,
				y: x.inicio.y + (x.fin.y - x.inicio.y) * k,
			});
			const puntos = [0, 0.45, 0.75, 1].map(paso);
			// Botes: alto al entrar, más bajo después.
			const alto = [70, 30, 10, 0];
			const giro =
				x.final === null
					? [0, 0.45, 0.75, 1].map(
							(k) =>
								`rotateX(${Math.round(k * x.vueltas) * 360 + (k === 1 && !(x.p.forma === "moneda" && x.p.cara) ? 180 : 0)}deg)`,
						)
					: [...x.giros.map((q) => css(deCuaternion(q))), css(x.final)];
			const opciones: KeyframeAnimationOptions = {
				duration: DURA,
				delay: i * 45,
				easing: "cubic-bezier(.2,.7,.3,1)",
				fill: "forwards",
			};
			animaciones.push(
				pieza.animate(
					puntos.map((pt, k) => ({
						transform: `translate3d(${pt.x}px,${pt.y - alto[k]}px,${alto[k]}px) ${giro[k]}`,
						offset: [0, 0.45, 0.75, 1][k],
					})),
					opciones,
				),
				sombra.animate(
					puntos.map((pt, k) => ({
						transform: `translate(${pt.x + 8}px,${pt.y + 10}px) scale(${1 + alto[k] / 120})`,
						opacity: 0.45 - alto[k] / 300,
						offset: [0, 0.45, 0.75, 1][k],
					})),
					opciones,
				),
			);
		});
		let vivo = true;
		void Promise.all(animaciones.map((a) => a.finished)).then(() => {
			if (vivo) acabar.current();
		});
		return () => {
			vivo = false;
			for (const a of animaciones) a.cancel();
		};
	}, [piezas]);

	return (
		<div
			ref={caja}
			className="absolute inset-0 flex items-center justify-center"
			style={{ perspective: 1100 }}
		>
			{piezas.map((_, i) => (
				<div key={`s${i}`} data-sombra={i} className="absolute size-0">
					<div className="absolute -top-8 -left-10 h-16 w-20 rounded-full bg-black/60 blur-md" />
				</div>
			))}
			{piezas.map((x, i) => (
				<div
					key={i}
					data-pieza={i}
					className="absolute size-0"
					style={{ transformStyle: "preserve-3d" }}
				>
					{x.p.forma === "moneda" ? (
						<Moneda />
					) : (
						<Dado p={x.p} color={color} final={x.final ?? deCuaternion([0, 0, 0, 1])} />
					)}
				</div>
			))}
		</div>
	);
}
