import { useLang } from "@/components/language-provider";
import { useMesa } from "@/stores/mesa-store";
import { useUi } from "@/stores/ui-store";
import { caraVisible, ImagenCarta } from "./imagen-carta";
import { usePantalla } from "./pantalla";
import { ptDe } from "./pt";

/**
 * **La vista grande** al pasar el ratón por una carta (`capturas/pablo/13`),
 * a un lado de la mesa, con el **panel de estado**: contadores, fuerza y
 * resistencia con de dónde sale cada bono, palabras clave, etiquetas y
 * quién la controla. Las de dos caras enseñan las dos.
 */
export function VistaGrande() {
	const { t } = useLang();
	const id = useMesa((s) => s.sobre);
	const c = useMesa((s) => (s.sobre !== null ? s.cartas[s.sobre] : undefined));
	const jugadores = useMesa((s) => s.jugadores);
	const arrastrando = usePantalla((s) => s.arrastre !== null);
	const ancho = useUi((s) => s.mesa.vistaGrande);
	const panel = useUi((s) => s.mesa.panelEstado);
	if (id === null || !c || arrastrando) return null;
	// Boca abajo y mía: la veo (es mía).
	const caras = c.origen ? c.origen.caras : [];
	const pt = ptDe(c);
	const contadores = Object.entries(c.contadores);
	const dosCaras = caras.length > 1 && !c.bocaAbajo;
	const alto = (ancho * 88) / 63;
	return (
		<div
			className="pointer-events-none absolute top-14 right-3 z-[8000] flex flex-col gap-2"
			style={{ width: dosCaras ? ancho * 2 + 8 : ancho }}
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
					c.habilidad) && (
					<div className="flex flex-col gap-1 rounded-lg border border-white/10 bg-black/75 p-2 text-[11px] text-white backdrop-blur">
						{c.habilidad && <p className="italic">{c.habilidad}</p>}
						{contadores.map(([tipo, n]) => (
							<div key={tipo} className="flex justify-between">
								<span>{tipo}</span>
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
						{c.palabrasClave.length > 0 && <span>{c.palabrasClave.join(" · ")}</span>}
						{c.etiquetas.length > 0 && <span>{c.etiquetas.map((e) => e.texto).join(" · ")}</span>}
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
