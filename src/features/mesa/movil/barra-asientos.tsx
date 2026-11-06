import { useState } from "react";
import { useShallow } from "zustand/react/shallow";
import { Icono } from "@/components/icono";
import { useLang } from "@/components/language-provider";
import { ScrollArea } from "@/components/ui/scroll-area";
import { cn } from "@/lib/utils";
import { NINGUNA, tableroDe, useMesa } from "@/stores/mesa-store";
import { useRed } from "@/stores/red-store";
import { useUi } from "@/stores/ui-store";
import { atacantesDe } from "../cuentas-flechas";
import { mirarJugador } from "../mirar";
import { elegirFantasma, elegirJugador } from "../objetivo";
import { usePantalla } from "../pantalla";
import { nombreFantasma, vidaDe } from "./cuentas-vida";

/** La barra la recuerda plegada o abierta mientras dure la app (no el reinicio). */
let plegadaGuardada = false;

/**
 * **La barra de asientos del móvil** (plan de la mesa, B5.10 e inventario §22):
 * el color y la vida de cada jugador (y del fantasma, en solitario); tocar uno
 * enseña su tablero. Se pliega a un solo botón. Con «Enfocar al jugador
 * activo», la mesa pasa sola a mirar al del turno en cada turno. Es solo de
 * pantalla: qué tablero se mira no cambia la partida.
 *
 * El móvil no tiene miniaturas: con una flecha en marcha, esta barra es donde se
 * elige al jugador objetivo. Tocar a uno lo marca y el ojo de al lado enseña su
 * tablero, para apuntar a una de sus cartas. Por eso se abre sola aunque esté
 * plegada. Adjuntar y atar una nota solo admiten cartas: tocar sigue cambiando
 * de tablero.
 */
export function BarraAsientos() {
	const { t } = useLang();
	const asientos = useMesa(useShallow((s) => s.asientos));
	const jugadores = useMesa((s) => s.jugadores);
	const viendo = useMesa(tableroDe);
	const yo = useMesa((s) => s.yo);
	const activo = useMesa((s) => s.general?.turno.activo ?? null);
	const primero = useMesa((s) => s.general?.turno.primero ?? null);
	const fantasma = useMesa((s) => s.general?.fantasma ?? null);
	const otros = useMesa((s) => s.general?.fantasmasExtra ?? NINGUNA);
	const verFantasma = useUi((s) => s.mesa.fantasma);
	const seguir = useUi((s) => s.mesa.seguirTurnoLocal);
	const equipos = useMesa((s) => s.general?.equipos);
	const enRed = useRed((s) => s.papel !== null);
	const flechas = useMesa((s) => s.general?.flechas ?? NINGUNA);
	const cartas = useMesa((s) => s.cartas);
	const apuntando = usePantalla((s) => s.modo?.tipo === "flecha");
	const [plegada, setPlegada] = useState(plegadaGuardada);
	const hayFantasma = fantasma?.visible === true && verFantasma;
	if (asientos.length < 2 && !hayFantasma) return null;

	const plegar = (si: boolean) => {
		plegadaGuardada = si;
		setPlegada(si);
	};
	const colorDe = (id: number) => {
		const j = jugadores[id];
		const e = j?.equipo != null ? equipos?.find((x) => x.id === j.equipo) : undefined;
		return e?.color || j?.color || "transparent";
	};

	if (plegada && !apuntando) {
		const colorActivo = activo === null ? "transparent" : colorDe(activo);
		return (
			<button
				type="button"
				aria-label={t("Enseñar los jugadores")}
				onClick={() => plegar(false)}
				className="absolute top-12 left-1 z-[7000] flex size-10 items-center justify-center rounded-xl border border-white/10 bg-black/60 text-white shadow-lg backdrop-blur-md"
				data-tutorial="mesa-asientos"
			>
				<Icono icon="lucide:users" className="size-5" />
				<span
					className="absolute -top-1 -right-1 size-3 rounded-full ring-2 ring-black/60"
					style={{ background: colorActivo }}
				/>
			</button>
		);
	}

	return (
		<div
			// Hasta la franja de la mano (la misma cuenta que `disponer`): no la tapa.
			style={{ bottom: "calc(min(220px, max(90px, 22%)) + 4px)" }}
			className={cn(
				"absolute top-12 left-1 z-[7000] flex flex-col gap-1 rounded-xl border border-white/10 bg-black/60 p-1 text-white shadow-lg backdrop-blur-md",
				apuntando ? "w-[9.5rem]" : "w-[6.5rem]",
			)}
			data-tutorial="mesa-asientos"
		>
			<ScrollArea className="min-h-0 flex-1" viewportClassName="h-full">
				<div className="flex flex-col gap-1 p-0.5">
					{asientos.map((id) => {
						const j = jugadores[id];
						if (!j) return null;
						const atacantes = atacantesDe(flechas, cartas, id);
						return (
							<div key={id} className="flex items-stretch gap-1">
								<button
									type="button"
									onClick={() => {
										if (!elegirJugador(id, t)) void mirarJugador(id);
									}}
									aria-pressed={viendo === id}
									className={cn(
										"flex min-h-11 min-w-0 flex-1 items-center gap-1.5 rounded-lg px-1.5 py-1 text-left text-xs",
										viendo === id && "bg-white/20",
										j.eliminado && "opacity-50",
										activo === id && "ring-2 ring-primary",
										apuntando && "bg-primary/30",
									)}
								>
									<span
										className="size-3 shrink-0 rounded-full"
										style={{ background: j.color, boxShadow: `0 0 0 2px ${colorDe(id)}` }}
									/>
									<span className="flex min-w-0 flex-1 flex-col leading-tight">
										<span className="truncate font-medium">
											{id === yo && enRed ? t("{nombre} (tú)", { nombre: j.nombre }) : j.nombre}
										</span>
										{/* Debajo del nombre, para no comerle sitio en esta barra tan estrecha. */}
										<span className="flex items-center gap-0.5 text-white/70 tabular-nums">
											{activo === id && (
												<Icono
													icon="lucide:play"
													role="img"
													aria-label={t("Tiene el turno")}
													className="size-3 shrink-0 text-primary"
												/>
											)}
											{asientos.length > 1 && primero === id && (
												<Icono
													icon="lucide:flag"
													role="img"
													aria-label={t("Primer jugador")}
													className="mr-0.5 size-3 shrink-0"
												/>
											)}
											<Icono icon="game-icons:hearts" className="size-3" />
											{vidaDe(j, equipos)}
											{atacantes > 0 && (
												<span
													role="img"
													className="ml-auto flex items-center gap-0.5 rounded-sm bg-rose-600 px-1 font-semibold text-white"
													aria-label={t("Ataques contra {nombre}: {cuantos}", {
														nombre: j.nombre,
														cuantos: atacantes,
													})}
												>
													<Icono icon="game-icons:crossed-swords" className="size-3" />
													{atacantes}
												</span>
											)}
										</span>
									</span>
								</button>
								{apuntando && viendo !== id && (
									<button
										type="button"
										aria-label={t("Ver el tablero de {nombre}", { nombre: j.nombre })}
										onClick={() => void mirarJugador(id)}
										className="flex w-9 shrink-0 items-center justify-center rounded-lg bg-white/10"
									>
										<Icono icon="lucide:eye" className="size-4" />
									</button>
								)}
							</div>
						);
					})}
					{hayFantasma &&
						fantasma &&
						[fantasma, ...otros].map((f, i) => {
							const otro = i === 0 ? null : i - 1;
							return (
								<button
									key={otro ?? "primero"}
									type="button"
									aria-label={nombreFantasma(t, otro, otros.length)}
									onClick={() => {
										if (!elegirFantasma(t, otro)) usePantalla.getState().setCapa("vida");
									}}
									className="flex min-h-11 items-center gap-1.5 rounded-lg px-1.5 py-1 text-left text-xs"
								>
									<Icono icon="game-icons:ghost" className="size-3.5 shrink-0 opacity-80" />
									<span className="flex min-w-0 flex-1 flex-col leading-tight">
										{/* Con varios, el número: «Fantasma 2» no cabe en esta barra y se cortaría. */}
										<span className="truncate font-medium">
											{otros.length > 0 ? i + 1 : t("Fantasma")}
										</span>
										<span className="flex items-center gap-0.5 text-white/70 tabular-nums">
											<Icono icon="game-icons:hearts" className="size-3" />
											{f.vida}
										</span>
									</span>
								</button>
							);
						})}
				</div>
			</ScrollArea>
			<div className="flex shrink-0 items-center justify-between gap-1">
				{asientos.length > 1 && (
					<button
						type="button"
						aria-label={t("Enfocar al jugador activo")}
						aria-pressed={seguir}
						onClick={() => useUi.getState().setMesa({ seguirTurnoLocal: !seguir })}
						className={cn(
							"flex size-11 items-center justify-center rounded-lg",
							seguir ? "bg-primary text-primary-foreground" : "bg-white/10",
						)}
					>
						<Icono icon="lucide:crosshair" className="size-5" />
					</button>
				)}
				<button
					type="button"
					aria-label={t("Plegar la barra de jugadores")}
					onClick={() => plegar(true)}
					className="flex size-11 items-center justify-center rounded-lg bg-white/10"
				>
					<Icono icon="lucide:chevron-left" className="size-5" />
				</button>
			</div>
		</div>
	);
}
