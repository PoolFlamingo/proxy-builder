import { useEffect, useRef } from "react";
import { createPortal } from "react-dom";
import { useLang } from "@/components/language-provider";
import { Progress } from "@/components/ui/progress";
import { useTrabajo } from "@/stores/trabajo-store";
import { centroSector, recortarNombre, trazoSector } from "./cuentas";
import { anguloDe, type Giro, useRuleta } from "./estado";

/** El radio de la rueda y hasta dónde llega un nombre, en unidades del dibujo. */
const RADIO = 100;
const FIN_NOMBRE = 90;
const INICIO_NOMBRE = 16;

/**
 * **La ruleta de quién empieza**, a toda la pantalla y por encima de los diálogos
 * (la partida se empieza desde uno y la mesa es un contexto de apilado propio).
 * No recibe clics: con un diálogo debajo, uno fuera lo cerraría.
 */
export function Ruleta() {
	const giro = useRuleta((s) => s.giro);
	if (!giro) return null;
	return createPortal(<Capa giro={giro} />, document.body);
}

function Capa({ giro }: { giro: Giro }) {
	const { t } = useLang();
	const trabajo = useTrabajo((s) => (s.actual?.tipo === "mesa" ? s.actual : null));
	const rueda = useRef<SVGGElement>(null);
	const { parada, parado, sectores } = giro;

	// El ángulo va por su cuenta, sin pasar por React: son 60 cambios por segundo.
	// `parado` vuelve a lanzarlo para pintar una vez el ángulo final.
	useEffect(() => {
		let id = 0;
		const pintar = () => {
			const g = useRuleta.getState().giro;
			if (!g || !rueda.current) return;
			rueda.current.setAttribute("transform", `rotate(${anguloDe(g, performance.now())})`);
			if (!g.parado) id = requestAnimationFrame(pintar);
		};
		pintar();
		return () => cancelAnimationFrame(id);
	}, [parado]);

	const n = sectores.length;
	const letra = n > 6 ? 9 : 11;
	const maxEm = (FIN_NOMBRE - INICIO_NOMBRE) / letra;
	const ganador = parado ? sectores.find((s) => s.id === parada?.ganador) : undefined;
	return (
		<div
			role="status"
			className="pointer-events-none fixed inset-0 z-[9900] flex animate-in flex-col items-center justify-center gap-4 bg-black/85 p-4 text-white backdrop-blur-sm duration-200 fade-in-0 landscape:[[data-formato=movil]_&]:flex-row landscape:[[data-formato=movil]_&]:gap-6"
		>
			<svg
				viewBox="-110 -110 220 220"
				role="img"
				aria-label={t("¿Quién empieza?")}
				className="aspect-square w-[min(68vmin,26rem)] shrink-0 drop-shadow-2xl"
			>
				<g ref={rueda}>
					{sectores.map((s, i) => (
						<g
							key={s.id}
							opacity={ganador && ganador.id !== s.id ? 0.35 : 1}
							className="transition-opacity duration-300"
						>
							<path
								d={trazoSector(i, n, RADIO)}
								fill={s.color}
								stroke="#fff"
								strokeOpacity={0.75}
								strokeWidth={1}
							/>
							<text
								// Del centro hacia fuera, a lo largo del radio de su sector.
								transform={`rotate(${centroSector(i, n) - 90})`}
								x={FIN_NOMBRE}
								textAnchor="end"
								dominantBaseline="central"
								fontSize={letra}
								fontWeight={600}
								fill="#fff"
								stroke="rgba(0,0,0,0.65)"
								strokeWidth={2}
								paintOrder="stroke"
							>
								{recortarNombre(s.nombre, maxEm)}
							</text>
						</g>
					))}
				</g>
				<circle r={RADIO} fill="none" stroke="#fff" strokeOpacity={0.9} strokeWidth={2.5} />
				<circle r={13} fill="#111" stroke="#fff" strokeWidth={2} />
				{/* La aguja, fija arriba. */}
				<path
					d="M0 -88 L-9 -108 H9 Z"
					fill="#fff"
					stroke="#111"
					strokeWidth={1.5}
					strokeLinejoin="round"
				/>
			</svg>
			<div className="flex w-72 max-w-full min-w-0 flex-col items-center gap-2.5 text-center">
				<h2 className="text-lg font-semibold">{t("¿Quién empieza?")}</h2>
				{/* Con hueco fijo: lo de abajo no hace saltar la rueda al cambiar. */}
				<div className="flex min-h-12 w-full min-w-0 flex-col items-center justify-center gap-1">
					{ganador ? (
						<p
							className="max-w-full animate-in truncate rounded-full px-4 py-1.5 text-lg font-semibold text-white shadow-lg duration-300 zoom-in-90 fade-in-0"
							style={{ background: ganador.color }}
						>
							{parada?.mio ? t("Empiezas tú") : t("Empieza {quien}", { quien: ganador.nombre })}
						</p>
					) : (
						trabajo && (
							<>
								<span className="text-xs text-white/70">{trabajo.fase}</span>
								<Progress value={trabajo.total ? (trabajo.hechas / trabajo.total) * 100 : 0} />
							</>
						)
					)}
				</div>
			</div>
		</div>
	);
}
