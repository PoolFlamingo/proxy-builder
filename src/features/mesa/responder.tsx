import { CornerUpLeft, Reply } from "lucide-react";
import { useMemo } from "react";
import { useShallow } from "zustand/react/shallow";
import type { IdJugador } from "@/bindings/IdJugador";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import {
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuLabel,
	DropdownMenuSeparator,
	DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { WithTooltip } from "@/components/ui/with-tooltip";
import { tableroDe, useMesa } from "@/stores/mesa-store";
import { useRed } from "@/stores/red-store";
import { responderComo, volverAlTurno } from "./mirar";
import { quienResponde } from "./privacidad";

const ESTILO = "h-7 gap-1 px-2 text-xs text-white hover:bg-white/10 hover:text-white";

/**
 * **Responder fuera de tu turno** (una partida local de varios, en la barra):
 * con el turno de otro, «Responder» pasa a mirar como el que responde, con
 * su mano tapada hasta que la destape; mientras tanto, «Volver a …» devuelve
 * al jugador del turno. No cambia nada de la partida: es solo qué jugador
 * mira la pantalla (el motor ya deja hacer con lo propio cuando se quiera).
 */
export function BotonResponder() {
	const { t } = useLang();
	const enRed = useRed((s) => s.papel !== null);
	const asientos = useMesa(useShallow((s) => s.asientos));
	const general = useMesa((s) => s.general);
	const jugadores = useMesa((s) => s.jugadores);
	const viendo = useMesa(tableroDe);
	const activo = general?.turno.activo;
	const quienes = useMemo(
		() => quienResponde({ asientos, general, jugadores }),
		[asientos, general, jugadores],
	);
	if (enRed || asientos.length < 2 || general?.mulligan != null || activo === undefined) {
		return null;
	}
	if (viendo !== activo) {
		const otros = quienes.filter((j) => j !== viendo);
		if (otros.length === 0) {
			return (
				<WithTooltip content={t("Volver al jugador del turno (Mayús+R)")}>
					<Button
						size="sm"
						variant="secondary"
						className="h-7 gap-1 px-2 text-xs"
						onClick={() => void volverAlTurno()}
					>
						<CornerUpLeft className="size-3.5" />
						{t("Volver a {nombre}", { nombre: jugadores[activo]?.nombre ?? "" })}
					</Button>
				</WithTooltip>
			);
		}
		// Con más de un jugador que responde: volver al turno o saltar a otro.
		return (
			<DropdownMenu>
				<WithTooltip content={t("Cambiar de jugador que responde (Mayús+R)")}>
					<DropdownMenuTrigger asChild>
						<Button size="sm" variant="secondary" className="h-7 gap-1 px-2 text-xs">
							<Reply className="size-3.5" />
							{t("Respondiendo como {nombre}", { nombre: jugadores[viendo]?.nombre ?? "" })}
						</Button>
					</DropdownMenuTrigger>
				</WithTooltip>
				<DropdownMenuContent align="center" className="z-[9000] w-56">
					<DropdownMenuItem onSelect={() => void volverAlTurno()}>
						<CornerUpLeft className="size-3.5" />
						{t("Volver a {nombre}", { nombre: jugadores[activo]?.nombre ?? "" })}
					</DropdownMenuItem>
					<DropdownMenuSeparator />
					<DropdownMenuLabel>{t("Responder como…")}</DropdownMenuLabel>
					{otros.map((j) => (
						<ItemJugador key={j} id={j} />
					))}
				</DropdownMenuContent>
			</DropdownMenu>
		);
	}
	if (quienes.length === 0) return null;
	const [primero] = quienes;
	if (quienes.length === 1 && primero !== undefined) {
		return (
			<WithTooltip content={t("Responder desde la mesa de otro jugador (Mayús+R)")}>
				<Button
					size="sm"
					variant="ghost"
					className={ESTILO}
					onClick={() => void responderComo(primero)}
				>
					<Reply className="size-3.5" />
					{t("Responder como {nombre}", { nombre: jugadores[primero]?.nombre ?? "" })}
				</Button>
			</WithTooltip>
		);
	}
	return (
		<DropdownMenu>
			<WithTooltip content={t("Responder desde la mesa de otro jugador (Mayús+R)")}>
				<DropdownMenuTrigger asChild>
					<Button size="sm" variant="ghost" className={ESTILO}>
						<Reply className="size-3.5" />
						{t("Responder")}
					</Button>
				</DropdownMenuTrigger>
			</WithTooltip>
			<DropdownMenuContent align="center" className="z-[9000] w-56">
				<DropdownMenuLabel>{t("Responder como…")}</DropdownMenuLabel>
				{quienes.map((j) => (
					<ItemJugador key={j} id={j} />
				))}
			</DropdownMenuContent>
		</DropdownMenu>
	);
}

/** Una entrada del menú: responder como este jugador, con el punto de su color. */
function ItemJugador({ id }: { id: IdJugador }) {
	const nombre = useMesa((s) => s.jugadores[id]?.nombre ?? "");
	const color = useMesa((s) => s.jugadores[id]?.color);
	return (
		<DropdownMenuItem onSelect={() => void responderComo(id)}>
			<span className="size-2 shrink-0 rounded-full" style={{ background: color }} />
			{nombre}
		</DropdownMenuItem>
	);
}
