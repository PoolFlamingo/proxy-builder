import { Eye, Heart, Languages, Loader2, Wifi, WifiOff } from "lucide-react";
import { useShallow } from "zustand/react/shallow";
import { ContenidoContextual } from "@/components/contenido-contextual";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import { ContextMenu, ContextMenuItem, ContextMenuTrigger } from "@/components/ui/context-menu";
import { WithTooltip } from "@/components/ui/with-tooltip";
import { cn } from "@/lib/utils";
import { NINGUNA, tableroDe, useMesa } from "@/stores/mesa-store";
import { useOriginales } from "@/stores/originales-store";
import { useRed } from "@/stores/red-store";
import { useVivo } from "@/stores/vivo-store";
import { salir } from "../conexion";
import { senalar } from "../en-vivo/emitir";
import { mirarJugador } from "../mirar";
import { alternarOriginales } from "../originales";
import { useDisposicion } from "../selector-disposicion";

/** Verde, ámbar o rojo según lo que tarda el ida y vuelta. */
export function colorDeLatencia(rttMs: number): string {
	if (rttMs < 80) return "text-emerald-400";
	if (rttMs < 200) return "text-amber-400";
	return "text-red-400";
}

/** La latencia de un jugador con el anfitrión y si el camino es directo o por el relé. */
export function Latencia({ id }: { id: number }) {
	const { t } = useLang();
	const e = useRed((s) => s.enlaces.find((f) => f.jugador === id));
	if (!e) return null;
	const ayuda = e.directo
		? t("{ms} ms · directo", { ms: e.rttMs })
		: t("{ms} ms · por relé", { ms: e.rttMs });
	return (
		<WithTooltip content={ayuda}>
			<span
				className={cn(
					"flex items-center gap-0.5 text-[10px] tabular-nums",
					colorDeLatencia(e.rttMs),
				)}
			>
				<Wifi className={cn("size-3", !e.directo && "opacity-60")} />
				{e.rttMs}
			</span>
		</WithTooltip>
	);
}

/**
 * **Los jugadores de la mesa** (con más de uno): su color, su nombre y su
 * vida; pulsar uno enseña su tablero. Con todos a la vez (Fase G) las
 * miniaturas ocupan su lugar.
 */
export function SelectorTablero() {
	const { t } = useLang();
	const asientos = useMesa(useShallow((s) => s.asientos));
	const jugadores = useMesa((s) => s.jugadores);
	const yo = useMesa((s) => s.yo);
	const viendo = useMesa(tableroDe);
	const papel = useRed((s) => s.papel);
	const espectador = papel === "espectador";
	const senales = useVivo((s) => s.senales);
	const originales = useOriginales((s) => s.modos);
	const equipos = useMesa((s) => s.general?.equipos ?? NINGUNA);
	const { config } = useDisposicion();
	// Con varios tableros a la vez, cada miniatura ya lleva su nombre y su vida.
	if (asientos.length < 2 || config.modo !== "unico") return null;
	return (
		<div className="absolute top-2 left-2 z-[7000] flex flex-col gap-1 rounded-xl border border-white/10 bg-black/60 p-1 text-white shadow-lg backdrop-blur-md">
			{espectador && (
				<span className="flex items-center gap-1 px-1.5 text-[10px] text-white/60">
					<Eye className="size-3" />
					{t("Mirando la partida")}
				</span>
			)}
			{asientos.map((id) => {
				const j = jugadores[id];
				if (!j) return null;
				return (
					<ContextMenu key={id}>
						<ContextMenuTrigger asChild>
							<button
								type="button"
								onClick={(e) => {
									// Alt + clic: un aro de aviso sobre el jugador, en todas las pantallas.
									if (e.altKey && papel !== null) senalar(null, id);
									else void mirarJugador(id);
								}}
								className={cn(
									"flex items-center gap-1.5 rounded-lg px-1.5 py-1 text-left text-xs transition hover:bg-white/10",
									viendo === id && "bg-white/15",
									j.eliminado && "opacity-50",
									senales.some((s) => s.a === id) && "animate-pulse ring-2 ring-primary",
								)}
							>
								<span
									className="size-2.5 shrink-0 rounded-full"
									style={{
										background: j.color,
										// El aro es el color de su equipo, si juega por equipos.
										boxShadow:
											j.equipo !== null
												? `0 0 0 2px ${equipos.find((e) => e.id === j.equipo)?.color ?? "transparent"}`
												: undefined,
									}}
								/>
								<span className="max-w-32 truncate font-medium">
									{id === yo ? t("{nombre} (tú)", { nombre: j.nombre }) : j.nombre}
								</span>
								{papel !== null && <Latencia id={id} />}
								{originales[id] && <Languages className="size-3 text-white/60" />}
								<span className="ml-auto flex items-center gap-0.5 pl-1.5 text-white/70 tabular-nums">
									<Heart className="size-3" />
									{j.vida}
								</span>
							</button>
						</ContextMenuTrigger>
						<ContenidoContextual className="w-64 text-xs">
							<ContextMenuItem onSelect={() => alternarOriginales(id)}>
								<Languages />
								{originales[id] ? "✓ " : ""}
								{t("Cartas originales en mi idioma")}
							</ContextMenuItem>
						</ContenidoContextual>
					</ContextMenu>
				);
			})}
		</div>
	);
}

/** Si la conexión con el anfitrión se corta: «Reconectando…» o perdida. */
export function AvisoConexion() {
	const { t } = useLang();
	const conexion = useRed((s) => s.conexion);
	const intento = useRed((s) => s.intento);
	if (conexion === "conectada") return null;
	return (
		<div className="absolute top-14 left-1/2 z-[7500] flex -translate-x-1/2 items-center gap-2.5 rounded-xl border border-amber-500/40 bg-black/80 px-2.5 py-1.5 text-xs text-white shadow-lg backdrop-blur-md">
			{conexion === "reconectando" ? (
				<>
					<Loader2 className="size-3.5 animate-spin text-amber-400" />
					{t("Reconectando con el anfitrión… (intento {intento} de 5)", { intento })}
				</>
			) : (
				<>
					<WifiOff className="size-3.5 text-red-400" />
					{t("Se ha perdido la conexión con la partida.")}
					<Button size="sm" variant="secondary" onClick={() => void salir(false)}>
						{t("Salir")}
					</Button>
				</>
			)}
		</div>
	);
}
