import { useRef, useState } from "react";
import { useShallow } from "zustand/react/shallow";
import type { IdJugador } from "@/bindings/IdJugador";
import { ContenidoContextual } from "@/components/contenido-contextual";
import { Icono } from "@/components/icono";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import {
	ContextMenu,
	ContextMenuItem,
	ContextMenuSeparator,
	ContextMenuSub,
	ContextMenuSubContent,
	ContextMenuSubTrigger,
	ContextMenuTrigger,
} from "@/components/ui/context-menu";
import { cn } from "@/lib/utils";
import { useMesa } from "@/stores/mesa-store";
import { useOriginales } from "@/stores/originales-store";
import { useRed } from "@/stores/red-store";
import { useUi } from "@/stores/ui-store";
import { hacer } from "./acciones";
import { CONTADORES_JUGADOR } from "./colores";
import { deltaSeguro } from "./dano-comandante";
import { MarcasJugador } from "./marcas-jugador";
import * as op from "./operaciones";
import { alternarOriginales } from "./originales";
import { pedirNumero, usePantalla } from "./pantalla";

/** Lo que se espera a que paren los clics de vida para mandarlos juntos, en ms. */
const JUNTAR_MS = 700;

/**
 * **La vida de un jugador** (`capturas/pablo/05`): clic en − y + (o la
 * rueda) suma; los clics seguidos se juntan en un solo evento y se enseña
 * la suma («40 + 2 = 42»). Con su menú: daño, veneno, comandante,
 * contadores, designaciones, revelar la mano y conceder. Junto a ella, el
 * botón de pasar el turno y los contadores que se hayan elegido.
 *
 * `ajena`: la de otro jugador, sobre su tablero cuando se le mira (en red);
 * sin el botón de pasar el turno ni lo que solo hace uno mismo (drenar,
 * revelar la mano, terminar). `conNombre`: con su nombre delante, para que
 * se sepa de quién es.
 */
export function VidaJugador({
	jugador,
	ajena = false,
	conNombre = false,
}: {
	jugador: IdJugador;
	ajena?: boolean;
	conNombre?: boolean;
}) {
	const { t } = useLang();
	const j = useMesa((s) => s.jugadores[jugador]);
	const activo = useMesa((s) => s.general?.turno.activo === jugador);
	const mulligan = useMesa((s) => s.general?.mulligan != null);
	const equipo = useMesa((s) =>
		j?.equipo != null ? s.general?.equipos.find((e) => e.id === j.equipo) : undefined,
	);
	const trackers = useUi((s) => s.mesa.trackers);
	const [pendiente, setPendiente] = useState(0);
	const temporizador = useRef<number | null>(null);
	const acumulado = useRef(0);
	if (!j) return null;
	const vida = equipo?.vidaCompartida ?? j.vida;

	const sumar = (d: number) => {
		acumulado.current += d;
		setPendiente(acumulado.current);
		if (temporizador.current) window.clearTimeout(temporizador.current);
		temporizador.current = window.setTimeout(() => {
			const n = acumulado.current;
			acumulado.current = 0;
			temporizador.current = null;
			setPendiente(0);
			if (n !== 0) void hacer({ que: "vida", jugador, delta: n });
		}, JUNTAR_MS);
	};

	const visibles = CONTADORES_JUGADOR.filter(
		(c) => trackers.includes(c.tipo) || (j.contadores[c.tipo] ?? 0) !== 0,
	);
	return (
		<div
			className="flex flex-col items-end gap-1.5"
			data-tutorial={ajena ? undefined : "mesa-vida"}
		>
			<MarcasJugador jugador={jugador} />
			{visibles.length > 0 && (
				<div className="flex flex-wrap justify-end gap-1">
					{visibles.map((c) => (
						<span
							key={c.tipo}
							className="flex items-center gap-1 rounded-full bg-black/60 px-1.5 py-0.5 text-[10px] text-white"
						>
							{t(c.nombre)}
							<button
								type="button"
								className="px-0.5"
								onClick={() =>
									void hacer({ que: "contadorJugador", jugador, tipo: c.tipo, delta: -1 })
								}
							>
								−
							</button>
							<b className="tabular-nums">{j.contadores[c.tipo] ?? 0}</b>
							<button
								type="button"
								className="px-0.5"
								onClick={() =>
									void hacer({ que: "contadorJugador", jugador, tipo: c.tipo, delta: 1 })
								}
							>
								+
							</button>
						</span>
					))}
				</div>
			)}
			<div className="flex items-center gap-2">
				{conNombre && (
					<span
						className="max-w-32 truncate rounded-full px-2 py-0.5 text-[11px] font-medium text-white shadow"
						style={{ background: equipo?.color || j.color }}
					>
						{j.nombre}
					</span>
				)}
				{activo && !mulligan && !ajena && (
					<Button
						size="sm"
						onClick={() => void op.pasarTurno()}
						className="rounded-full shadow-lg"
						data-tutorial="mesa-pasar"
					>
						{t("Pasar turno")}
					</Button>
				)}
				<MenuJugador jugador={jugador} ajena={ajena}>
					<div
						// Ahí apuntan las flechas que me atacan (`flechas.tsx`).
						data-vida-de={jugador}
						className={cn(
							"flex items-center gap-1 rounded-full border border-white/15 bg-black/60 px-2 py-1 text-white shadow-lg backdrop-blur-md",
							vida <= 0 && "ring-2 ring-rose-500",
						)}
						onWheel={(e) => sumar(e.deltaY < 0 ? 1 : -1)}
					>
						<button type="button" className="px-1.5 text-lg leading-none" onClick={() => sumar(-1)}>
							−
						</button>
						<div className="flex min-w-12 flex-col items-center leading-none">
							<span className="text-2xl font-bold tabular-nums">{vida + pendiente}</span>
							{pendiente !== 0 && (
								<span className="text-[10px] text-white/70 tabular-nums">
									{vida} {pendiente > 0 ? "+" : "−"} {Math.abs(pendiente)}
								</span>
							)}
						</div>
						<button type="button" className="px-1.5 text-lg leading-none" onClick={() => sumar(1)}>
							+
						</button>
					</div>
				</MenuJugador>
			</div>
		</div>
	);
}

function MenuJugador({
	jugador,
	ajena,
	children,
}: {
	jugador: IdJugador;
	ajena: boolean;
	children: React.ReactNode;
}) {
	const { t } = useLang();
	const j = useMesa((s) => s.jugadores[jugador]);
	const designaciones = useMesa((s) => s.general?.designaciones);
	const comandantes = useMesa(
		useShallow((s) => Object.values(s.cartas).filter((c) => c.origen?.info.esComandante)),
	);
	const trackers = useUi((s) => s.mesa.trackers);
	const originales = useOriginales((s) => s.modos[jugador] === true);
	// Terminar la partida de todos es del anfitrión (como en la barra).
	const manda = useRed((s) => s.papel === null || s.papel === "anfitrion");
	if (!j) return <>{children}</>;
	const alternarTracker = (tipo: string) =>
		useUi.getState().setMesa({
			trackers: trackers.includes(tipo) ? trackers.filter((x) => x !== tipo) : [...trackers, tipo],
		});
	return (
		<ContextMenu>
			<ContextMenuTrigger asChild>{children}</ContextMenuTrigger>
			<ContenidoContextual className="w-64 text-xs">
				<OpcionesDeVida jugador={jugador}>
					{!ajena && (
						<>
							<ContextMenuItem
								onSelect={() =>
									pedirNumero(t("Drenar (cada oponente pierde y tú ganas)"), 1, (n) =>
										hacer({ que: "drenar", n: Math.abs(n) }),
									)
								}
							>
								<Icono icon="game-icons:flame" />
								{t("Drenar…")}
							</ContextMenuItem>
							<ContextMenuItem
								onSelect={() =>
									pedirNumero(t("Vidas a todos (negativo: daño)"), -1, (n) =>
										hacer({ que: "vidaATodos", delta: n, soloOponentes: false }),
									)
								}
							>
								<Icono icon="game-icons:hearts" />
								{t("Vidas a todos…")}
							</ContextMenuItem>
						</>
					)}
				</OpcionesDeVida>
				{comandantes.length > 0 && (
					<ContextMenuSub>
						<ContextMenuSubTrigger>
							<Icono icon="game-icons:crown" />
							{t("Daño de comandante")}
						</ContextMenuSubTrigger>
						<ContextMenuSubContent className="text-xs">
							{comandantes.map((c) => (
								<ContextMenuItem
									key={c.id}
									onSelect={() =>
										pedirNumero(t("Daño de {nombre}", { nombre: c.origen?.nombre ?? "" }), 1, (n) =>
											hacer({
												que: "danoComandante",
												jugador,
												comandante: c.id,
												delta: deltaSeguro(j.danoComandante[c.id] ?? 0, n),
											}),
										)
									}
								>
									{c.origen?.nombre} · {j.danoComandante[c.id] ?? 0}
								</ContextMenuItem>
							))}
							<ContextMenuSeparator />
							<ContextMenuItem
								onSelect={() => usePantalla.getState().abrir({ tipo: "danoComandante" })}
							>
								{t("Tabla de daño de comandante…")}
							</ContextMenuItem>
						</ContextMenuSubContent>
					</ContextMenuSub>
				)}
				<ContextMenuSub>
					<ContextMenuSubTrigger>
						<Icono icon="lucide:hash" />
						{t("Contadores")}
					</ContextMenuSubTrigger>
					<ContextMenuSubContent className="w-52 text-xs">
						{CONTADORES_JUGADOR.map((c) => (
							<ContextMenuItem key={c.tipo} onSelect={() => alternarTracker(c.tipo)}>
								{trackers.includes(c.tipo) ? "✓ " : ""}
								{t(c.nombre)}
							</ContextMenuItem>
						))}
					</ContextMenuSubContent>
				</ContextMenuSub>
				<ContextMenuSeparator />
				<ContextMenuItem
					onSelect={() =>
						hacer({
							que: "designar",
							tipo: "monarca",
							a: designaciones?.monarca === jugador ? null : jugador,
						})
					}
				>
					<Icono icon="game-icons:crown" />
					{designaciones?.monarca === jugador ? t("Dejar de ser el monarca") : t("Ser el monarca")}
				</ContextMenuItem>
				<ContextMenuItem
					onSelect={() =>
						hacer({
							que: "designar",
							tipo: "iniciativa",
							a: designaciones?.iniciativa === jugador ? null : jugador,
						})
					}
				>
					<Icono icon="game-icons:dungeon-gate" />
					{designaciones?.iniciativa === jugador
						? t("Perder la iniciativa")
						: t("Tomar la iniciativa")}
				</ContextMenuItem>
				<ContextMenuItem onSelect={() => hacer({ que: "designar", tipo: "bendicion", a: jugador })}>
					<Icono icon="game-icons:castle" />
					{designaciones?.bendicion.includes(jugador)
						? t("Quitar la bendición de la ciudad")
						: t("Bendición de la ciudad")}
				</ContextMenuItem>
				<ContextMenuItem
					onSelect={() =>
						hacer({
							que: "diaNoche",
							valor: designaciones?.diaNoche === "dia" ? "noche" : "dia",
						})
					}
				>
					{designaciones?.diaNoche === "dia" ? (
						<Icono icon="game-icons:moon" />
					) : (
						<Icono icon="game-icons:sun" />
					)}
					{designaciones?.diaNoche === "dia" ? t("Hacer de noche") : t("Hacer de día")}
				</ContextMenuItem>
				<ContextMenuSeparator />
				{!ajena && (
					<ContextMenuItem onSelect={() => hacer({ que: "revelarMano", si: !j.manoRevelada })}>
						{j.manoRevelada ? <Icono icon="lucide:eye-off" /> : <Icono icon="lucide:eye" />}
						{j.manoRevelada ? t("Ocultar la mano") : t("Revelar la mano")}
					</ContextMenuItem>
				)}
				<ContextMenuItem onSelect={() => alternarOriginales(jugador)}>
					<Icono icon="lucide:languages" />
					{originales ? "✓ " : ""}
					{t("Cartas originales en mi idioma")}
				</ContextMenuItem>
				{!ajena && manda && (
					<ContextMenuItem onSelect={() => usePantalla.getState().abrir({ tipo: "terminar" })}>
						<Icono icon="game-icons:checkered-flag" />
						{t("Terminar la partida…")}
					</ContextMenuItem>
				)}
			</ContenidoContextual>
		</ContextMenu>
	);
}

/**
 * Lo de la vida de un jugador en un menú contextual: daño, ganar vidas (y lo
 * que se pase entre medias, `children`) y veneno. Lo usan su marcador de vida
 * y su miniatura (`tablero-mini.tsx`), así el daño va siempre a quien se
 * señala y no a quien mira.
 */
export function OpcionesDeVida({
	jugador,
	children,
}: {
	jugador: IdJugador;
	children?: React.ReactNode;
}) {
	const { t } = useLang();
	const veneno = useMesa((s) => s.jugadores[jugador]?.veneno ?? 0);
	return (
		<>
			<ContextMenuItem
				onSelect={() =>
					pedirNumero(t("Daño"), 1, (n) => hacer({ que: "vida", jugador, delta: -Math.abs(n) }))
				}
			>
				<Icono icon="game-icons:flame" />
				{t("Daño…")}
			</ContextMenuItem>
			<ContextMenuItem
				onSelect={() =>
					pedirNumero(t("Ganar vidas"), 1, (n) =>
						hacer({ que: "vida", jugador, delta: Math.abs(n) }),
					)
				}
			>
				<Icono icon="game-icons:hearts" />
				{t("Ganar vidas…")}
			</ContextMenuItem>
			{children}
			<ContextMenuSeparator />
			<ContextMenuItem onSelect={() => hacer({ que: "veneno", jugador, delta: 1 })}>
				<Icono icon="game-icons:poison-bottle" />
				{t("Un veneno más")}
			</ContextMenuItem>
			<ContextMenuItem
				disabled={veneno === 0}
				onSelect={() => hacer({ que: "veneno", jugador, delta: -1 })}
			>
				<Icono icon="game-icons:poison-bottle" />
				{t("Un veneno menos")}
			</ContextMenuItem>
		</>
	);
}
