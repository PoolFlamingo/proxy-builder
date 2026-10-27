import { useEffect, useId, useRef, useState } from "react";
import { type Estilo, METALES } from "./aspecto";
import { entre, generador } from "./azar";
import type { PiezaTirada, Tirada } from "./cola";
import { textoCara } from "./geometria";
import { CAOS, CARA, CRUZ, PLANESWALK } from "./simbolos";
import { agitarDados, golpeDado, golpeMoneda, lanzarMoneda } from "./sonidos";

/**
 * **Los dados en 2D**, lo más sencillo: la silueta de cada dado girando un
 * momento mientras cambian los números, y al parar el del motor. Para equipos
 * sin WebGL ni soltura con el CSS 3D, o para quien lo prefiera (Ajustes).
 */

const DURA = 900;

/** La silueta de cada dado (en una caja de 100 × 100). */
const SILUETA: Record<number, string> = {
	4: "50,8 94,86 6,86",
	6: "12,12 88,12 88,88 12,88",
	8: "50,4 94,50 50,96 6,50",
	10: "50,4 92,42 50,96 8,42",
	12: "50,4 95,37 78,92 22,92 5,37",
	20: "50,3 92,27 92,73 50,97 8,73 8,27",
};

function Pieza({
	p,
	estilo,
	fondo,
	valor,
}: {
	p: PiezaTirada;
	estilo: Estilo;
	fondo: string;
	valor: React.ReactNode;
}) {
	const id = useId();
	if (p.forma === "moneda") {
		const metal = METALES[estilo.moneda];
		return (
			<div
				className="flex size-14 items-center justify-center rounded-full shadow-lg"
				style={{
					background: `radial-gradient(circle at 40% 35%, ${metal.claro}, ${metal.oscuro})`,
					boxShadow: `inset 0 0 0 3px ${metal.borde}`,
				}}
			>
				{valor}
			</div>
		);
	}
	const lados = p.forma === "planar" ? 6 : p.tipo;
	return (
		<div className="relative size-16 drop-shadow-lg">
			<svg viewBox="0 0 100 100" className="absolute inset-0" aria-hidden>
				<defs>
					<pattern id={id} patternUnits="userSpaceOnUse" width={100} height={100}>
						<image href={fondo} width={100} height={100} preserveAspectRatio="xMidYMid slice" />
					</pattern>
				</defs>
				<polygon
					points={SILUETA[lados] ?? SILUETA[6]}
					fill={`url(#${CSS.escape(id)})`}
					fillOpacity={estilo.acabado === "translucido" && !estilo.imagen ? 0.82 : 1}
					stroke="rgba(0,0,0,.35)"
					strokeWidth={3}
				/>
			</svg>
			<div
				className="absolute inset-0 flex items-center justify-center pt-1 text-lg font-bold"
				style={{ color: estilo.numeros, textShadow: `0 0 3px ${estilo.contorno}` }}
			>
				{valor}
			</div>
		</div>
	);
}

function Simbolo({ d, relleno, trazo }: { d: string; relleno: string; trazo: string }) {
	return (
		<svg viewBox="0 0 100 100" className="size-8" aria-hidden>
			<path d={d} fill={relleno} stroke={trazo} strokeWidth={5} paintOrder="stroke" />
		</svg>
	);
}

/** Lo que enseña una pieza (al final, el resultado; mientras gira, otro al azar). */
function cara(p: PiezaTirada, r: () => number, final: boolean, e: Estilo): React.ReactNode {
	if (p.forma === "moneda") {
		const metal = METALES[e.moneda];
		return (
			<Simbolo
				d={(final ? p.cara : r() < 0.5) ? CARA : CRUZ}
				relleno={metal.simbolo}
				trazo={metal.trazo}
			/>
		);
	}
	if (p.forma === "planar") {
		const c = final ? p.cara : (["planeswalk", "caos", "nada"] as const)[Math.floor(r() * 3)];
		return c === "planeswalk" ? (
			<Simbolo d={PLANESWALK} relleno={e.numeros} trazo={e.contorno} />
		) : c === "caos" ? (
			<Simbolo d={CAOS} relleno={e.numeros} trazo={e.contorno} />
		) : null;
	}
	const v = final
		? p.valor
		: Math.floor(r() * (p.tipo === 10 ? 10 : p.tipo)) + (p.tipo === 10 ? 0 : 1);
	return textoCara(p.tipo, v, p.decenas);
}

export function DadosPlanos({
	tirada,
	estilo,
	fondo,
	alAcabar,
}: {
	tirada: Tirada;
	estilo: Estilo;
	/** El fondo de las caras (`data:` URL de `texturas.urlFondo`). */
	fondo: string;
	alAcabar: () => void;
}) {
	const caja = useRef<HTMLDivElement>(null);
	const acabar = useRef(alAcabar);
	acabar.current = alAcabar;
	const [parado, setParado] = useState(false);
	const [, setPaso] = useState(0);
	const r = useRef(generador(tirada.semilla));

	useEffect(() => {
		const raiz = caja.current;
		if (!raiz) return;
		const rr = generador(tirada.semilla ^ 0x9e3779b9);
		const animaciones = [...raiz.querySelectorAll<HTMLElement>("[data-pieza]")].map((el, i) =>
			el.animate(
				[
					{ transform: `translate(${entre(rr, -80, 80)}px, 220px) rotate(0deg) scale(.7)` },
					{
						transform: `translate(0, -20px) rotate(${entre(rr, 400, 700)}deg) scale(1.1)`,
						offset: 0.7,
					},
					{ transform: "translate(0, 0) rotate(720deg) scale(1)" },
				],
				{ duration: DURA, delay: i * 40, easing: "cubic-bezier(.2,.7,.3,1)", fill: "forwards" },
			),
		);
		// Mientras gira, los números cambian y suenan en la mano; al parar, cada uno cae.
		const id = window.setInterval(() => setPaso((n) => n + 1), 70);
		const monedas = tirada.piezas.every((p) => p.forma === "moneda");
		if (monedas) lanzarMoneda();
		else agitarDados(tirada.piezas.length, (DURA * 0.6) / 1000);
		animaciones.forEach((a, i) => {
			a.onfinish = () =>
				tirada.piezas[i]?.forma === "moneda" ? golpeMoneda(0.6) : golpeDado(0.55);
		});
		let vivo = true;
		void Promise.all(animaciones.map((a) => a.finished)).then(() => {
			window.clearInterval(id);
			if (!vivo) return;
			setParado(true);
			acabar.current();
		});
		return () => {
			vivo = false;
			window.clearInterval(id);
			for (const a of animaciones) a.cancel();
		};
	}, [tirada.semilla, tirada.piezas]);

	return (
		<div ref={caja} className="absolute inset-0 flex items-center justify-center">
			<div className="flex max-w-[70%] flex-wrap items-center justify-center gap-3">
				{tirada.piezas.map((p, i) => (
					<div key={i} data-pieza={i}>
						<Pieza p={p} estilo={estilo} fondo={fondo} valor={cara(p, r.current, parado, estilo)} />
					</div>
				))}
			</div>
		</div>
	);
}
