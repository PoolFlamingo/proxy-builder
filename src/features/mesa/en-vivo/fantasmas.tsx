import { useEffect } from "react";
import { useShallow } from "zustand/react/shallow";
import { useLang } from "@/components/language-provider";
import { tableroDe, useMesa } from "@/stores/mesa-store";
import { useUi } from "@/stores/ui-store";
import { useVivo } from "@/stores/vivo-store";
import { type Disposicion, posAPx } from "../disposicion";
import { caraVisible, DorsoGenerico, ImagenCarta } from "../imagen-carta";

/**
 * **Lo que mueven los demás** (E7), sobre el campo que se está mirando: el
 * cursor de cada jugador con su color y su nombre, la carta que arrastra
 * (si no es visible para mí, su dorso) y los aros de aviso. Se mueve con
 * una transición de 100 ms: los datos llegan a 10–20 por segundo y así se
 * ve fluido. No recibe clics (`pointer-events-none`).
 */
export function FantasmasVivos({ disp }: { disp: Disposicion }) {
	const { t } = useLang();
	const activo = useUi((s) => s.mesa.movimientoEnVivo);
	const viendo = useMesa(tableroDe);
	const fantasmas = useVivo(useShallow((s) => Object.entries(s.fantasmas)));
	const senales = useVivo((s) => s.senales);
	const jugadores = useMesa((s) => s.jugadores);
	const cartas = useMesa((s) => s.cartas);
	const reducir = useUi((s) => s.mesa.reducirAnimaciones);

	// Lo que ya caducó se quita (un cursor quieto se queda 5 s).
	useEffect(() => {
		const i = window.setInterval(() => useVivo.getState().purgar(performance.now()), 500);
		return () => window.clearInterval(i);
	}, []);

	if (!activo) return null;
	const transicion = reducir ? undefined : "transform 100ms linear";
	return (
		<div className="pointer-events-none absolute inset-0 overflow-hidden" style={{ zIndex: 8500 }}>
			{senales.map((s) => {
				const c = s.carta === null ? undefined : cartas[s.carta];
				if (
					!c?.pos ||
					c.zona.tipo !== "de" ||
					c.zona.zona !== "campo" ||
					c.zona.jugador !== viendo
				) {
					return null;
				}
				const p = posAPx(disp.campo, c.pos);
				const color = jugadores[s.jugador]?.color ?? "#e11d48";
				return (
					<div
						key={s.id}
						className="absolute animate-ping rounded-xl border-4"
						style={{
							left: p.x - disp.carta.w * 0.7,
							top: p.y - disp.carta.h * 0.6,
							width: disp.carta.w * 1.4,
							height: disp.carta.h * 1.2,
							borderColor: color,
						}}
					/>
				);
			})}
			{fantasmas.map(([clave, f]) => {
				const jugador = Number(clave);
				if (f.tablero !== viendo) return null;
				const p = posAPx(disp.campo, f);
				const color = jugadores[jugador]?.color ?? "#e11d48";
				const nombre = jugadores[jugador]?.nombre ?? t("Jugador");
				const carta = f.carta === null ? undefined : cartas[f.carta];
				return (
					<div
						key={clave}
						className="absolute top-0 left-0 will-change-transform"
						style={{ transform: `translate(${p.x}px, ${p.y}px)`, transition: transicion }}
					>
						{f.carta !== null && (
							<div
								className="absolute opacity-70 shadow-xl"
								style={{
									width: disp.carta.w,
									height: disp.carta.h,
									left: -disp.carta.w / 2,
									top: -disp.carta.h / 2,
									transform: carta?.girada ? "rotate(90deg)" : undefined,
								}}
							>
								{carta ? (
									<ImagenCarta imagen={caraVisible(carta)} ancho={244} />
								) : (
									<DorsoGenerico />
								)}
							</div>
						)}
						<span
							className="absolute size-3 -translate-x-1/2 -translate-y-1/2 rounded-full border border-white shadow"
							style={{ background: color }}
						/>
						<span
							className="absolute left-2 top-2 rounded px-1 text-[10px] whitespace-nowrap text-white"
							style={{ background: color }}
						>
							{nombre}
						</span>
					</div>
				);
			})}
		</div>
	);
}
