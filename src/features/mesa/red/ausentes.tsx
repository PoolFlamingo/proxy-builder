import { useEffect, useState } from "react";
import { useShallow } from "zustand/react/shallow";
import { Icono } from "@/components/icono";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import { useMesa } from "@/stores/mesa-store";
import { useRed } from "@/stores/red-store";
import { hacer } from "../acciones";
import { ESPERA_SALTAR_MS, segundosAusente, siguienteVivo } from "../cuentas-muerte";

/**
 * **Quien se ha ido de la partida** (G8, en red): un aviso con quién falta.
 * Si es a quien le toca jugar, a los 30 s el siguiente jugador ve «Saltar el
 * turno de X» y los demás, quién puede hacerlo. Con la partida en el
 * mulligan, el anfitrión se queda por él con su mano pasado un minuto (lo
 * hace Rust; aquí solo se cuenta).
 */
export function Ausentes() {
	const { t } = useLang();
	const ausentes = useRed((s) => s.ausentes);
	const papel = useRed((s) => s.papel);
	const asientos = useMesa(useShallow((s) => s.asientos));
	const jugadores = useMesa((s) => s.jugadores);
	const activo = useMesa((s) => s.general?.turno.activo ?? 0);
	const yo = useMesa((s) => s.yo ?? 0);
	const mulligan = useMesa((s) => s.general?.mulligan != null);
	const fin = useMesa((s) => s.general?.fin != null);
	const [ahora, setAhora] = useState(() => Date.now());
	useEffect(() => {
		if (ausentes.length === 0) return;
		const id = window.setInterval(() => setAhora(Date.now()), 1000);
		return () => window.clearInterval(id);
	}, [ausentes.length]);
	const filas = ausentes.filter(
		(a) => jugadores[a.jugador] && jugadores[a.jugador]?.eliminado == null,
	);
	if (papel === null || fin || filas.length === 0) return null;
	const vivos = asientos.filter((id) => jugadores[id]?.eliminado == null);
	const siguiente = siguienteVivo(asientos, vivos, activo);
	return (
		<div className="absolute top-14 right-3 z-[7500] flex max-w-72 flex-col gap-1.5">
			{filas.map((a) => {
				const quien = jugadores[a.jugador]?.nombre ?? "";
				const seg = segundosAusente(a.haceMs, a.visto, ahora);
				const esElTurno = a.jugador === activo && !mulligan;
				const puedeSaltar = esElTurno && seg * 1000 >= ESPERA_SALTAR_MS;
				return (
					<div
						key={a.jugador}
						className="flex flex-col gap-1.5 rounded-xl border border-amber-500/40 bg-black/85 p-2.5 text-xs text-white shadow-lg backdrop-blur-md"
					>
						<div className="flex items-center gap-2">
							<Icono icon="lucide:user-x" className="size-3.5 shrink-0 text-amber-400" />
							<span>{t("{quien} está desconectado.", { quien })}</span>
						</div>
						{mulligan && (
							<p className="text-white/70">{t("Se quedará con su mano si no vuelve.")}</p>
						)}
						{esElTurno && !puedeSaltar && (
							<p className="flex items-center gap-1.5 text-white/70">
								<Icono icon="lucide:loader-2" className="size-3 animate-spin" />
								{t("Su turno se podrá saltar en {segundos} s.", {
									segundos: Math.max(0, Math.ceil(ESPERA_SALTAR_MS / 1000) - seg),
								})}
							</p>
						)}
						{puedeSaltar &&
							(siguiente === yo && papel !== "espectador" ? (
								<Button
									size="sm"
									onClick={() => void hacer({ que: "saltarTurno", jugador: a.jugador })}
								>
									{t("Saltar el turno de {quien}", { quien })}
								</Button>
							) : (
								siguiente !== null && (
									<p className="text-white/70">
										{t("{quien} puede saltar su turno.", {
											quien: jugadores[siguiente]?.nombre ?? "",
										})}
									</p>
								)
							))}
					</div>
				);
			})}
		</div>
	);
}
