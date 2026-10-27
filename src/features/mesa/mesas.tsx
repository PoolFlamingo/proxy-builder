import { useEffect, useMemo, useRef, useState } from "react";
import { useShallow } from "zustand/react/shallow";
import { useLang } from "@/components/language-provider";
import { tableroDe, useMesa } from "@/stores/mesa-store";
import { useRed } from "@/stores/red-store";
import { AyudasTurno } from "./ayudas-turno";
import { Bocadillo, InsigniaSenal } from "./chat";
import {
	HUECO,
	limitarProporcion,
	ordenTableros,
	PROPORCION_MAX,
	PROPORCION_MIN,
	proporcionPorPuntero,
	type Rect,
	repartir,
} from "./disposicion";
import { PanelPlanes } from "./planes";
import { PanelPlanar } from "./plano-activo";
import { deckDelTablero } from "./privacidad";
import { useDisposicion } from "./selector-disposicion";
import { Tablero } from "./tablero";
import { TableroMini } from "./tablero-mini";
import { VidaJugador } from "./vida-jugador";

/**
 * **Los tableros de la mesa.** Con un solo jugador (o en «un solo tablero»)
 * es el tablero de siempre. Con varios, `repartir` los coloca: el tablero de
 * quien se mira es el interactivo y los demás son miniaturas que, al pulsarlas,
 * pasan a ser el grande. En «dos tableros» el separador se arrastra.
 */
export function Mesas({ deckIds }: { deckIds: string[] }) {
	const { n, config } = useDisposicion();
	// Cada tablero con el tapete de su jugador (no el del primero).
	const asientos = useMesa(useShallow((s) => s.asientos));
	const viendo = useMesa(tableroDe);
	const enRed = useRed((s) => s.papel !== null);
	const esAnfitrion = useRed((s) => s.papel === "anfitrion");
	const deckId = deckDelTablero(deckIds, asientos, viendo, enRed, esAnfitrion);
	if (n < 2 || config.modo === "unico") {
		return (
			<div className="relative size-full">
				<Tablero deckId={deckId} />
				<Controles />
			</div>
		);
	}
	return <VariosTableros deckId={deckId} />;
}

/** Lo que el que juega tiene a mano sobre su tablero: las ayudas del turno y su vida. */
function Controles() {
	const yo = useMesa((s) => s.yo ?? 0);
	return (
		<div
			className="absolute right-3 z-[7000] flex flex-col items-end gap-1.5"
			style={{ bottom: "calc(min(220px, max(90px, 22%)) + 24px)" }}
		>
			<Bocadillo jugador={yo} />
			<PanelPlanar />
			<PanelPlanes />
			<AyudasTurno />
			<InsigniaSenal jugador={yo} />
			<VidaJugador jugador={yo} />
		</div>
	);
}

/** Lo que dejan libre la barra de la mesa por arriba, en px. */
const MARGEN_BARRA = 52;

function VariosTableros({ deckId }: { deckId: string | null }) {
	const { t } = useLang();
	const { config, cambiar } = useDisposicion();
	const caja = useRef<HTMLDivElement>(null);
	const [tam, setTam] = useState({ w: 0, h: 0 });
	const asientos = useMesa(useShallow((s) => s.asientos));
	const yoReal = useMesa((s) => s.yo);
	const grande = useMesa(tableroDe);
	const equiposPorJugador = useMesa(
		useShallow((s) => asientos.map((id) => s.jugadores[id]?.equipo ?? null)),
	);
	const enRed = useRed((s) => s.papel !== null);
	const colorGrande = useMesa((s) => {
		const j = s.jugadores[grande];
		const e = j?.equipo != null ? s.general?.equipos.find((x) => x.id === j.equipo) : undefined;
		return e?.color ?? j?.color ?? "transparent";
	});

	useEffect(() => {
		const el = caja.current;
		if (!el) return;
		const ro = new ResizeObserver(([e]) =>
			setTam({ w: e.contentRect.width, h: e.contentRect.height }),
		);
		ro.observe(el);
		return () => ro.disconnect();
	}, []);

	const orden = useMemo(() => ordenTableros(asientos, yoReal, !enRed), [asientos, yoReal, enRed]);
	const rects = useMemo(() => {
		const equipos = orden.map((id) => equiposPorJugador[asientos.indexOf(id)] ?? null);
		const pantalla: Rect = { x: 0, y: 0, w: tam.w, h: tam.h };
		return repartir(orden.length, pantalla, config, Math.max(0, orden.indexOf(grande)), equipos);
	}, [orden, equiposPorJugador, asientos, tam, config, grande]);

	const rectGrande = rects[Math.max(0, orden.indexOf(grande))];
	const arrastrar = (e: React.PointerEvent<HTMLDivElement>) => {
		e.preventDefault();
		const el = e.currentTarget;
		el.setPointerCapture(e.pointerId);
		const mueve = (ev: PointerEvent) => {
			const r = caja.current?.getBoundingClientRect();
			if (!r) return;
			const pantalla: Rect = { x: 0, y: 0, w: r.width, h: r.height };
			cambiar({
				proporcion: proporcionPorPuntero(
					pantalla,
					ev.clientX - r.left,
					ev.clientY - r.top,
					config.vertical,
				),
			});
		};
		const suelta = () => {
			el.removeEventListener("pointermove", mueve);
			el.removeEventListener("pointerup", suelta);
			el.removeEventListener("pointercancel", suelta);
		};
		el.addEventListener("pointermove", mueve);
		el.addEventListener("pointerup", suelta);
		el.addEventListener("pointercancel", suelta);
	};

	return (
		<div className="relative size-full">
			<div
				ref={caja}
				className="absolute inset-x-0 bottom-0 overflow-hidden"
				style={{ top: MARGEN_BARRA }}
			>
				{tam.w > 0 && rectGrande && (
					<div
						className="absolute"
						style={{
							left: rectGrande.x,
							top: rectGrande.y,
							width: rectGrande.w,
							height: rectGrande.h,
						}}
					>
						<Tablero deckId={deckId} />
						<Controles />
						<div
							className="pointer-events-none absolute inset-0 z-[6600] rounded-md border-2"
							style={{ borderColor: colorGrande }}
						/>
					</div>
				)}
				{tam.w > 0 &&
					orden.map((id, i) => {
						const r = rects[i];
						if (id === grande || !r || r.w <= 0 || r.h <= 0) return null;
						return (
							<div
								key={id}
								className="absolute"
								style={{ left: r.x, top: r.y, width: r.w, height: r.h }}
							>
								<TableroMini jugador={id} w={r.w} h={r.h} />
							</div>
						);
					})}
				{tam.w > 0 && config.modo === "dividido" && rects.length === 2 && (
					<div
						role="separator"
						tabIndex={0}
						aria-label={t("Separador entre los tableros")}
						aria-orientation={config.vertical ? "vertical" : "horizontal"}
						aria-valuemin={Math.round(PROPORCION_MIN * 100)}
						aria-valuemax={Math.round(PROPORCION_MAX * 100)}
						aria-valuenow={Math.round(limitarProporcion(config.proporcion) * 100)}
						onPointerDown={arrastrar}
						onKeyDown={(e) => {
							const paso =
								e.key === "ArrowLeft" || e.key === "ArrowUp"
									? -0.05
									: e.key === "ArrowRight" || e.key === "ArrowDown"
										? 0.05
										: 0;
							if (paso === 0) return;
							e.preventDefault();
							cambiar({ proporcion: limitarProporcion(config.proporcion + paso) });
						}}
						className="absolute z-[6500] rounded bg-white/0 transition hover:bg-primary/60 focus-visible:bg-primary/60 focus-visible:outline-none"
						style={
							config.vertical
								? {
										left: rects[0].x + rects[0].w - 4,
										top: 0,
										width: HUECO + 8,
										height: "100%",
										cursor: "col-resize",
									}
								: {
										top: rects[0].y + rects[0].h - 4,
										left: 0,
										height: HUECO + 8,
										width: "100%",
										cursor: "row-resize",
									}
						}
					/>
				)}
			</div>
		</div>
	);
}
