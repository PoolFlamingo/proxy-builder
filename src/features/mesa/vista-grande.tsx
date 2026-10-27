import { Clock } from "lucide-react";
import { useEffect, useMemo, useRef } from "react";
import { useLang } from "@/components/language-provider";
import { useMesa } from "@/stores/mesa-store";
import { useUi } from "@/stores/ui-store";
import { esTemporal, nombreContador } from "./contadores";
import { caraVisible, ImagenCarta } from "./imagen-carta";
import { ANCHO_PANEL, usePantalla } from "./pantalla";
import { ptDe } from "./pt";

/**
 * **La vista grande** al pasar el ratón por una carta (`capturas/pablo/13`),
 * a un lado de la mesa, con el **panel de estado**: contadores, fuerza y
 * resistencia con de dónde sale cada bono, palabras clave, etiquetas y
 * quién la controla. Las de dos caras enseñan las dos. Si la carta señalada
 * está en la mitad derecha de la pantalla, la vista grande sale a la
 * izquierda para no taparla; el lado se decide al empezar a señalarla.
 */
export function VistaGrande() {
	const { t } = useLang();
	const id = useMesa((s) => s.sobre);
	const c = useMesa((s) => (s.sobre !== null ? s.cartas[s.sobre] : undefined));
	const jugadores = useMesa((s) => s.jugadores);
	const arrastrando = usePantalla((s) => s.arrastre !== null);
	const ancho = useUi((s) => s.mesa.vistaGrande);
	const panel = useUi((s) => s.mesa.panelEstado);
	const panelLateral = usePantalla((s) => s.panel !== null);
	// Dónde está el puntero: el lado se decide con la última posición conocida
	// al señalar una carta nueva (no en cada movimiento, para que no salte).
	const ultimaX = useRef(0);
	useEffect(() => {
		const mover = (e: PointerEvent) => {
			ultimaX.current = e.clientX;
		};
		window.addEventListener("pointermove", mover, { passive: true });
		return () => window.removeEventListener("pointermove", mover);
	}, []);
	const aLaIzquierda = useMemo(() => id !== null && ultimaX.current > window.innerWidth / 2, [id]);
	if (id === null || !c || arrastrando) return null;
	// Boca abajo y mía: la veo (es mía).
	const caras = c.origen ? c.origen.caras : [];
	const pt = ptDe(c);
	const contadores = Object.entries(c.contadores);
	const dosCaras = caras.length > 1 && !c.bocaAbajo;
	const alto = (ancho * 88) / 63;
	return (
		<div
			className={`pointer-events-none absolute top-14 z-[8000] flex flex-col gap-2 ${aLaIzquierda ? "left-3" : "right-3"}`}
			style={{
				width: dosCaras ? ancho * 2 + 8 : ancho,
				// Con el panel lateral abierto, la carta se pone a su izquierda.
				...(panelLateral && !aLaIzquierda ? { right: ANCHO_PANEL + 20 } : {}),
			}}
		>
			<div className="flex gap-2">
				{(dosCaras ? caras : [caraVisible(c, true)]).map((imagen, i) => (
					<div
						key={i}
						className="overflow-hidden rounded-[4.5%] shadow-2xl"
						style={{ width: ancho, height: alto }}
					>
						<ImagenCarta imagen={imagen} ancho={488} nombre={c.origen?.nombre} />
					</div>
				))}
			</div>
			{panel &&
				(contadores.length > 0 ||
					pt?.cambiada ||
					c.palabrasClave.length > 0 ||
					c.etiquetas.length > 0 ||
					c.controlador !== c.dueno ||
					c.devolverA !== null ||
					c.habilidad) && (
					<div className="flex flex-col gap-1 rounded-lg border border-white/10 bg-black/75 p-2 text-[11px] text-white backdrop-blur">
						{c.habilidad && <p className="italic">{c.habilidad}</p>}
						{contadores.map(([tipo, n]) => (
							<div key={tipo} className="flex justify-between">
								<span className="flex items-center gap-1">
									{esTemporal(tipo) && <Clock className="size-3 text-amber-300" />}
									{nombreContador(tipo, t)}
								</span>
								<b className="tabular-nums">{n}</b>
							</div>
						))}
						{pt?.cambiada && (
							<div className="flex flex-col gap-0.5 border-t border-white/10 pt-1">
								<div className="flex justify-between">
									<span>{t("Fuerza / resistencia")}</span>
									<b className="tabular-nums">
										{pt.f}/{pt.r}
									</b>
								</div>
								{c.pt.fijo && (
									<span className="text-white/70">
										{t("Fijada en {f}/{r}", { f: c.pt.fijo[0], r: c.pt.fijo[1] })}
									</span>
								)}
								{(c.pt.otros[0] !== 0 || c.pt.otros[1] !== 0) && (
									<span className="text-white/70">
										{t("Otros bonos: {f}/{r}", { f: c.pt.otros[0], r: c.pt.otros[1] })}
									</span>
								)}
								{(c.pt.hastaFinDeTurno[0] !== 0 || c.pt.hastaFinDeTurno[1] !== 0) && (
									<span className="text-white/70">
										{t("Hasta el final del turno: {f}/{r}", {
											f: c.pt.hastaFinDeTurno[0],
											r: c.pt.hastaFinDeTurno[1],
										})}
									</span>
								)}
							</div>
						)}
						{c.palabrasClave.length > 0 && (
							<span className="flex flex-wrap items-center gap-x-1.5">
								{c.palabrasClave.map((p) => (
									<span key={p} className="flex items-center gap-0.5">
										{esTemporal(p) && <Clock className="size-3 text-amber-300" />}
										{nombreContador(p, t)}
									</span>
								))}
							</span>
						)}
						{c.etiquetas.length > 0 && <span>{c.etiquetas.map((e) => e.texto).join(" · ")}</span>}
						{c.devolverA !== null && (
							<span className="flex items-center gap-1 text-amber-300">
								<Clock className="size-3" />
								{t("Vuelve a {quien} al final del turno", {
									quien: jugadores[c.devolverA]?.nombre ?? "",
								})}
							</span>
						)}
						{c.controlador !== c.dueno && (
							<span className="text-rose-300">
								{t("La controla {quien}; es de {dueno}", {
									quien: jugadores[c.controlador]?.nombre ?? "",
									dueno: jugadores[c.dueno]?.nombre ?? "",
								})}
							</span>
						)}
					</div>
				)}
		</div>
	);
}
