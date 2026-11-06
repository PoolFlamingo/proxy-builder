import { useMemo } from "react";
import type { IdJugador } from "@/bindings/IdJugador";
import { AyudaAlTocar } from "@/components/ayuda-al-tocar";
import { Icono } from "@/components/icono";
import { useLang } from "@/components/language-provider";

import { cn } from "@/lib/utils";
import { useMesa } from "@/stores/mesa-store";
import { comandantesDe } from "./dano-comandante";

/**
 * **Lo que lleva encima un jugador**, a la vista: veneno, el daño de cada
 * comandante, monarca, iniciativa, bendición de la ciudad y, con varios
 * jugadores, la bandera de quien empezó (de ahí se cuentan las rondas). Rojo lo
 * que ya llega al límite (el motor solo avisa; quién queda fuera lo confirma la
 * persona). `pequeno`: la cabecera de las miniaturas.
 */
export function MarcasJugador({
	jugador,
	pequeno = false,
	className,
}: {
	jugador: IdJugador;
	pequeno?: boolean;
	className?: string;
}) {
	const { t } = useLang();
	const j = useMesa((s) => s.jugadores[jugador]);
	const designaciones = useMesa((s) => s.general?.designaciones);
	const primero = useMesa((s) => s.asientos.length > 1 && s.general?.turno.primero === jugador);
	const limiteVeneno = useMesa((s) => s.opciones?.limiteVeneno ?? 10);
	const limiteComandante = useMesa((s) => s.opciones?.limiteComandante ?? 21);
	const cartas = useMesa((s) => s.cartas);
	const nombres = useMemo(
		() => Object.fromEntries(comandantesDe(cartas).map((c) => [c.id, c.nombre])),
		[cartas],
	);
	if (!j) return null;
	const comandantes = Object.entries(j.danoComandante).filter(([, n]) => n > 0);
	const monarca = designaciones?.monarca === jugador;
	const iniciativa = designaciones?.iniciativa === jugador;
	const bendicion = designaciones?.bendicion.includes(jugador) ?? false;
	if (!j.veneno && comandantes.length === 0 && !monarca && !iniciativa && !bendicion && !primero) {
		return null;
	}
	const icono = pequeno ? "size-3" : "size-3.5";
	const marca = cn(
		"flex shrink-0 items-center gap-0.5 rounded-full tabular-nums",
		pequeno
			? "text-[11px]"
			: "bg-black/60 px-1.5 py-0.5 text-xs text-white shadow backdrop-blur-sm",
	);
	return (
		<span className={cn("flex items-center gap-1", className)}>
			{primero && (
				<AyudaAlTocar ayuda={t("Primer jugador")}>
					<span className={marca}>
						<Icono icon="lucide:flag" className={icono} />
					</span>
				</AyudaAlTocar>
			)}
			{monarca && (
				<AyudaAlTocar ayuda={t("Monarca")}>
					<span className={cn(marca, "text-amber-300")}>
						<Icono icon="game-icons:crown" className={icono} />
					</span>
				</AyudaAlTocar>
			)}
			{iniciativa && (
				<AyudaAlTocar ayuda={t("Tiene la iniciativa")}>
					<span className={cn(marca, "text-sky-300")}>
						<Icono icon="game-icons:dungeon-gate" className={icono} />
					</span>
				</AyudaAlTocar>
			)}
			{bendicion && (
				<AyudaAlTocar ayuda={t("Bendición de la ciudad")}>
					<span className={cn(marca, "text-yellow-200")}>
						<Icono icon="game-icons:castle" className={icono} />
					</span>
				</AyudaAlTocar>
			)}
			{j.veneno > 0 && (
				<AyudaAlTocar ayuda={t("Veneno")}>
					<span className={cn(marca, j.veneno >= limiteVeneno ? "text-rose-400" : "text-lime-300")}>
						<Icono icon="game-icons:poison-bottle" className={icono} />
						{j.veneno}
					</span>
				</AyudaAlTocar>
			)}
			{comandantes.map(([id, n]) => (
				<AyudaAlTocar key={id} ayuda={t("Daño de {nombre}", { nombre: nombres[Number(id)] ?? "" })}>
					<span className={cn(marca, n >= limiteComandante ? "text-rose-400" : "text-orange-200")}>
						<Icono icon="game-icons:crowned-skull" className={icono} />
						{n}
					</span>
				</AyudaAlTocar>
			))}
		</span>
	);
}
