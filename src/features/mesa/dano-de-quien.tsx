import { useEffect, useRef } from "react";
import { create } from "zustand";
import { useShallow } from "zustand/react/shallow";
import type { IdJugador } from "@/bindings/IdJugador";
import { Icono } from "@/components/icono";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { useMesa } from "@/stores/mesa-store";
import { useRed } from "@/stores/red-store";
import { DURA_PREGUNTA_MS, debePreguntarDano, REPOSO_DANO_MS, vidaPerdida } from "./cuentas-muerte";

type Pregunta = { jugador: IdJugador; dano: number };

/**
 * Lo que se sabe de quién hizo el último daño a cada jugador (solo en esta
 * pantalla: sirve para apuntar quién lo eliminó) y la pregunta abierta.
 */
export const useDanoDe = create<{
	ultimo: Record<number, IdJugador>;
	pregunta: Pregunta | null;
	apuntar: (jugador: IdJugador, quien: IdJugador) => void;
	preguntar: (p: Pregunta | null) => void;
	limpiar: () => void;
}>((set) => ({
	ultimo: {},
	pregunta: null,
	apuntar: (jugador, quien) =>
		set((s) => ({ ultimo: { ...s.ultimo, [jugador]: quien }, pregunta: null })),
	preguntar: (pregunta) => set({ pregunta }),
	limpiar: () => set({ ultimo: {}, pregunta: null }),
}));

/** La vida que cuenta de cada asiento (la del equipo si es compartida). */
function vidasDe(s: ReturnType<typeof useMesa.getState>): number[] {
	return s.asientos.map((id) => {
		const j = s.jugadores[id];
		const e = j?.equipo != null ? s.general?.equipos.find((x) => x.id === j.equipo) : undefined;
		return e?.vidaCompartida ?? j?.vida ?? 0;
	});
}

/**
 * **¿De quién fue el daño?** Cuando a alguien le hacen mucho daño de golpe
 * en el turno de otro, se le pregunta de quién vino, con un clic para el del
 * turno. Si no se contesta, no se apunta nada (como antes).
 */
export function DanoDeQuien() {
	const { t } = useLang();
	const vidas = useMesa(useShallow(vidasDe));
	const pregunta = useDanoDe((s) => s.pregunta);
	const anterior = useRef<number[] | null>(null);
	const acumulado = useRef<Record<number, number>>({});
	const reloj = useRef<number | null>(null);

	// Una partida nueva: se olvida lo de la anterior.
	const activa = useMesa((s) => s.activa);
	useEffect(() => {
		if (!activa) {
			useDanoDe.getState().limpiar();
			anterior.current = null;
			acumulado.current = {};
		}
	}, [activa]);

	useEffect(() => {
		const antes = anterior.current;
		anterior.current = vidas;
		if (!antes || antes.length !== vidas.length) return;
		let hubo = false;
		vidas.forEach((v, i) => {
			const perdida = vidaPerdida(antes[i] ?? v, v);
			if (perdida > 0) {
				acumulado.current[i] = (acumulado.current[i] ?? 0) + perdida;
				hubo = true;
			} else if (v > (antes[i] ?? v)) {
				// Si se recupera vida, lo de antes ya no cuenta.
				acumulado.current[i] = 0;
			}
		});
		if (!hubo) return;
		if (reloj.current !== null) window.clearTimeout(reloj.current);
		reloj.current = window.setTimeout(() => {
			reloj.current = null;
			const m = useMesa.getState();
			const total = acumulado.current;
			acumulado.current = {};
			const activo = m.general?.turno.activo ?? 0;
			const vivos = m.asientos.filter((id) => m.jugadores[id]?.eliminado == null);
			const enRed = useRed.getState().papel !== null;
			const yo = m.yo ?? 0;
			const vistos = new Set<number>();
			for (const [i, id] of m.asientos.entries()) {
				const dano = total[i] ?? 0;
				const eq = m.jugadores[id]?.equipo;
				// Con vida compartida, una pregunta por equipo basta.
				if (eq != null) {
					if (vistos.has(eq)) continue;
					vistos.add(eq);
				}
				if (
					debePreguntarDano({
						total: dano,
						jugador: id,
						activo,
						vivos,
						controlado: !enRed || id === yo,
						terminada: m.general?.fin != null || m.general?.mulligan != null,
					})
				) {
					useDanoDe.getState().preguntar({ jugador: id, dano });
					return;
				}
			}
		}, REPOSO_DANO_MS);
	}, [vidas]);

	useEffect(
		() => () => {
			if (reloj.current !== null) window.clearTimeout(reloj.current);
		},
		[],
	);

	// Sin contestar, la pregunta se va sola.
	useEffect(() => {
		if (!pregunta) return;
		const id = window.setTimeout(() => useDanoDe.getState().preguntar(null), DURA_PREGUNTA_MS);
		return () => window.clearTimeout(id);
	}, [pregunta]);

	const asientos = useMesa(useShallow((s) => s.asientos));
	const jugadores = useMesa((s) => s.jugadores);
	const activo = useMesa((s) => s.general?.turno.activo ?? 0);
	if (!pregunta) return null;
	const quien = jugadores[pregunta.jugador];
	if (!quien) return null;
	const candidatos = asientos
		.filter((id) => id !== pregunta.jugador && jugadores[id]?.eliminado == null)
		.sort((a, b) => Number(b === activo) - Number(a === activo));
	return (
		<div className="absolute top-14 left-1/2 z-[7500] flex max-w-[90%] -translate-x-1/2 flex-col gap-1.5 rounded-xl border border-white/15 bg-black/85 p-2.5 text-xs text-white shadow-lg backdrop-blur-md">
			<div className="flex items-center gap-2">
				<Icono icon="game-icons:crossed-swords" className="size-3.5 text-amber-400" />
				<span>
					{t("{quien} ha recibido {dano} de daño en el turno de otro. ¿De quién fue?", {
						quien: quien.nombre,
						dano: pregunta.dano,
					})}
				</span>
				<button
					type="button"
					aria-label={t("Cerrar")}
					className="ml-auto rounded p-0.5 text-white/60 hover:bg-white/10"
					onClick={() => useDanoDe.getState().preguntar(null)}
				>
					<Icono icon="lucide:x" className="size-3.5" />
				</button>
			</div>
			<div className="flex flex-wrap gap-1.5">
				{candidatos.map((id) => (
					<Button
						key={id}
						size="sm"
						variant={id === activo ? "default" : "secondary"}
						className={cn("gap-1.5", id !== activo && "text-foreground")}
						onClick={() => useDanoDe.getState().apuntar(pregunta.jugador, id)}
					>
						<span
							className="size-2 rounded-full"
							style={{ background: jugadores[id]?.color ?? "transparent" }}
						/>
						{jugadores[id]?.nombre}
					</Button>
				))}
				<Button
					size="sm"
					variant="ghost"
					className="text-white/70"
					onClick={() => useDanoDe.getState().preguntar(null)}
				>
					{t("No importa")}
				</Button>
			</div>
		</div>
	);
}
