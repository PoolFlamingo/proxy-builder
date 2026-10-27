import { Hand, Heart, Languages, Library, Skull } from "lucide-react";
import { memo } from "react";
import type { Equipo } from "@/bindings/Equipo";
import type { IdJugador } from "@/bindings/IdJugador";
import type { VistaCarta } from "@/bindings/VistaCarta";
import type { VistaJugador } from "@/bindings/VistaJugador";
import { ContenidoContextual } from "@/components/contenido-contextual";
import { useLang } from "@/components/language-provider";
import { ContextMenu, ContextMenuItem, ContextMenuTrigger } from "@/components/ui/context-menu";
import { cn } from "@/lib/utils";
import { cartasDe, cuantasEn, useMesa } from "@/stores/mesa-store";
import { useOriginales } from "@/stores/originales-store";
import { useRed } from "@/stores/red-store";
import { useVivo } from "@/stores/vivo-store";
import { zonaDe } from "./acciones";
import { Bocadillo, InsigniaSenal } from "./chat";
import { abanico, posAPx } from "./disposicion";
import { senalar } from "./en-vivo/emitir";
import { caraVisible, DorsoGenerico, ImagenCarta } from "./imagen-carta";
import { igualesLista, useMesaLenta } from "./lento";
import { mirarJugador } from "./mirar";
import { alternarOriginales } from "./originales";
import { Latencia } from "./red/estado-red";

/** Alto de la cabecera (nombre y vida) de una miniatura, en px. */
const CABECERA = 20;
/** Por debajo de este alto, la miniatura no pinta el campo, solo los números. */
const ALTO_CON_CAMPO = 64;

type DatosMini = {
	j: VistaJugador | undefined;
	campo: VistaCarta[];
	mano: number;
	/** Las cartas de la mano que este mirador ve (la de un compañero); el resto, reversos. */
	manoVista: VistaCarta[];
	biblioteca: number;
	activo: boolean;
	equipo: Equipo | undefined;
};

/** Lo que enseña la miniatura de `jugador`, sacado de la mesa. */
export function datosMini(
	m: Pick<ReturnType<typeof useMesa.getState>, "jugadores" | "zonas" | "cartas" | "general">,
	jugador: IdJugador,
): DatosMini {
	const j = m.jugadores[jugador];
	const campo: VistaCarta[] = [];
	for (const id of cartasDe(m, zonaDe(jugador, "campo"))) {
		const c = m.cartas[id];
		if (c) campo.push(c);
	}
	const manoVista: VistaCarta[] = [];
	for (const id of cartasDe(m, zonaDe(jugador, "mano"))) {
		const c = m.cartas[id];
		if (c) manoVista.push(c);
	}
	return {
		j,
		campo,
		mano: cuantasEn(m, zonaDe(jugador, "mano")),
		manoVista,
		biblioteca: cuantasEn(m, zonaDe(jugador, "biblioteca")),
		activo: m.general?.turno.activo === jugador,
		equipo: j?.equipo != null ? m.general?.equipos.find((e) => e.id === j.equipo) : undefined,
	};
}

/** Dos lecturas de la misma miniatura que se pintarían igual. */
export function igualMini(a: DatosMini, b: DatosMini): boolean {
	return (
		a.j === b.j &&
		a.mano === b.mano &&
		igualesLista(a.manoVista, b.manoVista) &&
		a.biblioteca === b.biblioteca &&
		a.activo === b.activo &&
		a.equipo === b.equipo &&
		igualesLista(a.campo, b.campo)
	);
}

/**
 * **El tablero de otro jugador, en pequeño**: su nombre y vida, cuántas cartas
 * tiene en la mano y la biblioteca y, debajo, su campo a escala. No se puede
 * tocar nada: pulsarlo pasa a mirar a ese jugador. Se pone al día como mucho
 * 10 veces por segundo y sin animaciones, para que 9 miniaturas no frenen el
 * tablero grande.
 */
export const TableroMini = memo(function TableroMini({
	jugador,
	w,
	h,
}: {
	jugador: IdJugador;
	w: number;
	h: number;
}) {
	const { t } = useLang();
	const d = useMesaLenta((m) => datosMini(m, jugador), igualMini);
	const enRed = useRed((s) => s.papel !== null);
	const senalada = useVivo((s) => s.senales.some((x) => x.a === jugador));
	const originales = useOriginales((s) => s.modos[jugador]);
	const j = d.j;
	if (!j) return null;
	const color = d.equipo?.color ?? j.color;
	const vida = d.equipo?.vidaCompartida ?? j.vida;
	const eliminado = j.eliminado !== null;
	const conCampo = h >= ALTO_CON_CAMPO;
	const campo = { x: 0, y: CABECERA, w, h: Math.max(0, h - CABECERA) };
	const ancho = Math.max(14, Math.min(44, w / 10));
	const alto = (ancho * 88) / 63;
	return (
		<ContextMenu>
			<ContextMenuTrigger asChild>
				<button
					type="button"
					onClick={(e) => {
						// Alt + clic: un aro de aviso sobre el jugador, en todas las pantallas.
						if (e.altKey && enRed) senalar(null, jugador);
						else void mirarJugador(jugador);
					}}
					aria-label={j.nombre}
					className={cn(
						"relative isolate block size-full overflow-hidden rounded-md border-2 bg-black/50 text-left text-white",
						d.activo && "ring-2 ring-primary",
						eliminado && "opacity-50",
						senalada && "animate-pulse ring-2 ring-primary",
					)}
					style={{ borderColor: color }}
				>
					<Bocadillo jugador={jugador} className="absolute bottom-1 left-1" />
					<span
						className="absolute inset-x-0 top-0 flex items-center gap-1.5 px-1.5 text-[11px] leading-none"
						style={{ height: CABECERA, background: `${color}66` }}
					>
						{eliminado && <Skull className="size-3 shrink-0" />}
						<span className="min-w-0 flex-1 truncate font-medium">{j.nombre}</span>
						<InsigniaSenal jugador={jugador} />
						{enRed && <Latencia id={jugador} />}
						{originales && <Languages className="size-3 shrink-0 text-white/60" />}
						<span className="flex shrink-0 items-center gap-0.5 tabular-nums">
							<Heart className="size-3" />
							{vida}
						</span>
						{w >= 120 && (
							<>
								<span
									className="flex shrink-0 items-center gap-0.5 tabular-nums"
									aria-label={t("Mano")}
								>
									<Hand className="size-3" />
									{d.mano}
								</span>
								<span
									className="flex shrink-0 items-center gap-0.5 tabular-nums"
									aria-label={t("Biblioteca")}
								>
									<Library className="size-3" />
									{d.biblioteca}
								</span>
							</>
						)}
					</span>
					{conCampo && d.mano > 0 && <ManoMini d={d} w={w} ancho={ancho} />}
					{conCampo &&
						d.campo.map((c) => {
							if (!c.pos || c.adjuntaA !== null || c.agrupadaBajo !== null) return null;
							const p = posAPx(campo, c.pos);
							return (
								<span
									key={c.id}
									className="absolute block"
									style={{
										left: p.x - ancho / 2,
										top: p.y - alto / 2,
										width: ancho,
										height: alto,
										zIndex: c.z,
										transform: c.girada ? "rotate(90deg)" : undefined,
									}}
									onPointerEnter={() => useMesa.getState().setSobre(c.id)}
									onPointerLeave={() => {
										if (useMesa.getState().sobre === c.id) useMesa.getState().setSobre(null);
									}}
								>
									<ImagenCarta imagen={caraVisible(c)} ancho={146} />
								</span>
							);
						})}
				</button>
			</ContextMenuTrigger>
			<ContenidoContextual className="w-64 text-xs">
				<ContextMenuItem onSelect={() => alternarOriginales(jugador)}>
					<Languages />
					{originales ? "✓ " : ""}
					{t("Cartas originales en mi idioma")}
				</ContextMenuItem>
			</ContenidoContextual>
		</ContextMenu>
	);
});

/** Tope de reversos que se dibujan en la franja de la mano de una miniatura. */
const MAX_MANO_MINI = 20;

/**
 * La mano de un jugador en pequeño, abajo de su miniatura: **siempre se ve**,
 * con el reverso del deck de ese jugador (el elegido o el genérico) aunque la
 * carta sea de doble cara, para que nadie sepa qué tiene. Solo las cartas que
 * este mirador puede ver de verdad (la mano de un compañero de equipo) salen
 * boca arriba; esas las decide el motor.
 */
function ManoMini({ d, w, ancho }: { d: DatosMini; w: number; ancho: number }) {
	const n = Math.min(d.mano, MAX_MANO_MINI);
	const cw = Math.max(12, Math.min(ancho, 30));
	const ch = (cw * 88) / 63;
	const franja = ch * 0.62;
	// El mismo abanico que la mano grande, a escala: el arco y el giro se
	// reducen con el tamaño de la carta para que se note sin torcerla mucho.
	const escala = ch / 90;
	const pos = abanico({ x: 0, y: 0, w: w - 8, h: ch }, n, { w: cw, h: ch }).map((p) => ({
		x: p.x + 4,
		y: ch / 2 + (p.y - ch / 2) * escala,
		giro: p.giro,
	}));
	const dorso = d.j?.dorso ?? null;
	const abiertas = d.manoVista.length > 0;
	return (
		<span
			className="pointer-events-none absolute inset-x-0 bottom-0 z-[9000] block bg-black/55"
			style={{ height: franja + 4 }}
			data-mano={abiertas ? "abierta" : "oculta"}
		>
			{Array.from({ length: n }, (_, i) => {
				// De la mano abierta, las que se ven; si faltan, reversos.
				const c = d.manoVista[i];
				return (
					<span
						key={c?.id ?? `r${i}`}
						className="absolute block overflow-hidden rounded-[2px]"
						style={{
							left: (pos[i]?.x ?? 0) - cw / 2,
							top: (pos[i]?.y ?? ch / 2) - ch / 2 + 2,
							width: cw,
							height: ch,
							transform: `rotate(${pos[i]?.giro ?? 0}deg)`,
						}}
					>
						{c ? (
							<ImagenCarta imagen={caraVisible(c)} ancho={146} />
						) : dorso ? (
							<ImagenCarta imagen={dorso} ancho={146} />
						) : (
							<DorsoGenerico />
						)}
					</span>
				);
			})}
		</span>
	);
}
