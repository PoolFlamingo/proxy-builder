import { useEffect, useState } from "react";
import { useShallow } from "zustand/react/shallow";
import type { ResumenPartida } from "@/bindings/ResumenPartida";
import { Icono } from "@/components/icono";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import { call } from "@/lib/ipc";
import { useMesa } from "@/stores/mesa-store";
import { useRed } from "@/stores/red-store";
import { hacer } from "./acciones";
import { BanquilloDialog } from "./banquillo-dialog";
import { salir } from "./conexion";
import { formatoDuracion } from "./cuentas-fin";
import { COMO } from "./dialogos/terminar";
import { useEncuentro } from "./encuentro";
import { verRepeticionActual } from "./repeticion";

/** El resumen de la partida (puestos, rondas, duración), pedido a Rust al terminar. */
export function useResumen(): ResumenPartida | null {
	const fin = useMesa((s) => s.general?.fin ?? null);
	const [r, setR] = useState<ResumenPartida | null>(null);
	useEffect(() => {
		if (!fin) return setR(null);
		let vivo = true;
		call("mesa_resumen")
			.then((x) => vivo && setR(x))
			.catch(() => vivo && setR(null));
		return () => {
			vivo = false;
		};
	}, [fin]);
	return r;
}

/**
 * **Al terminar** (`capturas/pablo/21`): quién ganó y cómo, cómo quedó cada
 * uno (y quién lo eliminó), cuánto duró, y otra partida o salir.
 */
export function FinDePartida() {
	const { t } = useLang();
	const fin = useMesa((s) => s.general?.fin ?? null);
	const jugadores = useMesa((s) => s.jugadores);
	const asientos = useMesa(useShallow((s) => s.asientos));
	const encuentro = useEncuentro();
	const resumen = useResumen();
	// En red, la siguiente partida del encuentro la abre el anfitrión; el
	// invitado solo cambia su banquillo.
	const invitado = useRed((s) => s.papel) === "jugador";
	const [oculto, setOculto] = useState(false);
	const [banquillo, setBanquillo] = useState(false);
	useEffect(() => {
		if (fin) setOculto(false);
	}, [fin]);
	// Escape lo aparta para ver la mesa (como «Ver la mesa»).
	useEffect(() => {
		if (!fin || oculto) return;
		const tecla = (e: KeyboardEvent) => e.key === "Escape" && setOculto(true);
		window.addEventListener("keydown", tecla);
		return () => window.removeEventListener("keydown", tecla);
	}, [fin, oculto]);
	if (!fin) return null;
	// Apartado: un botón para volver al resultado (y a «Otra partida»).
	if (oculto && !banquillo)
		return (
			<div className="absolute bottom-3 left-1/2 z-[8800] -translate-x-1/2">
				<Button size="sm" className="rounded-full shadow-lg" onClick={() => setOculto(false)}>
					<Icono icon="game-icons:checkered-flag" className="size-4" />
					{t("Ver el resultado")}
				</Button>
			</div>
		);
	const nombre = (g: number) => jugadores[g]?.nombre ?? "?";
	const ganadores = fin.ganadores.map(nombre);
	const como = COMO.find((c) => c.valor === fin.como)?.nombre ?? "";
	const estado = encuentro?.estado ?? null;
	const decide = estado?.terminado ?? false;
	return (
		<div className="absolute inset-0 z-[8800] flex items-center justify-center bg-black/60 backdrop-blur-sm">
			<div className="flex w-[34rem] max-w-[94%] flex-col items-center gap-2.5 rounded-xl border bg-card p-2.5 py-4 text-center shadow-2xl">
				<Icono icon="game-icons:checkered-flag" className="size-6 text-primary" />
				<h2 className="text-base font-semibold">
					{ganadores.length === 0
						? asientos.length > 1
							? t("Empate")
							: t("Ganó el rival")
						: t("Ganó {quien}", { quien: ganadores.join(", ") })}
				</h2>
				<p className="text-xs text-muted-foreground">{t(como)}</p>
				{fin.nota && <p className="text-xs">{fin.nota}</p>}
				{resumen && asientos.length > 1 && (
					<div className="flex w-full flex-col gap-0.5 rounded-lg border bg-background/40 p-2 text-xs">
						{resumen.puestos.map((p) => {
							const j = jugadores[p.jugador];
							return (
								<div key={p.jugador} className="flex items-center justify-between gap-2">
									<span className="flex min-w-0 items-center gap-1.5">
										<span className="w-16 shrink-0 text-left whitespace-nowrap text-muted-foreground tabular-nums">
											{t("Puesto {puesto}", { puesto: p.puesto })}
										</span>
										<span
											className="size-2.5 shrink-0 rounded-full"
											style={{ background: j?.color }}
										/>
										<span className="truncate">{nombre(p.jugador)}</span>
									</span>
									<span className="shrink-0 text-muted-foreground">
										{p.ronda === null
											? t("Sigue en pie")
											: p.por === null
												? t("Fuera en la ronda {r}", { r: p.ronda })
												: t("Fuera en la ronda {r}, por {quien}", {
														r: p.ronda,
														quien: nombre(p.por),
													})}
									</span>
								</div>
							);
						})}
					</div>
				)}
				{resumen && (
					<p className="text-[11px] text-muted-foreground tabular-nums">
						{t("Rondas: {rondas} · Duración: {duracion}", {
							rondas: resumen.rondas,
							duracion: formatoDuracion(resumen.duracionMs),
						})}
					</p>
				)}
				{estado && (
					<div className="flex w-full flex-col gap-1 rounded-lg border bg-background/40 p-2 text-xs">
						<span className="text-muted-foreground">
							{t("Al mejor de {total}: partida {actual}", {
								total: estado.alMejorDe,
								actual: estado.jugadas,
							})}
						</span>
						<span className="flex flex-wrap justify-center gap-x-4 gap-y-0.5 tabular-nums">
							{asientos.map((id) => (
								<span key={id}>
									{nombre(id)}: <b>{estado.marcador[id] ?? 0}</b>
								</span>
							))}
						</span>
						{decide && (
							<b className="text-primary">
								{estado.ganadores.length === 0
									? t("El encuentro acaba en tablas")
									: t("Gana el encuentro {quien}", {
											quien: estado.ganadores.map(nombre).join(", "),
										})}
							</b>
						)}
					</div>
				)}
				<div className="flex flex-wrap justify-center gap-2.5">
					<Button variant="outline" autoFocus onClick={() => setOculto(true)}>
						{t("Ver la mesa")}
					</Button>
					<Button variant="outline" onClick={() => void verRepeticionActual()}>
						<Icono icon="lucide:history" className="size-4" />
						{t("Ver repetición")}
					</Button>
					<Button variant="outline" onClick={() => void salir(false)}>
						<Icono icon="lucide:log-out" className="size-4" />
						{t("Salir")}
					</Button>
					{estado ? (
						!decide &&
						(invitado ? (
							<Button variant="outline" onClick={() => setBanquillo(true)}>
								<Icono icon="lucide:rotate-ccw" className="size-4" />
								{t("Banquillo")}
							</Button>
						) : (
							<Button onClick={() => setBanquillo(true)}>
								<Icono icon="lucide:rotate-ccw" className="size-4" />
								{t("Siguiente partida")}
							</Button>
						))
					) : (
						<Button onClick={() => void hacer({ que: "reiniciar" })}>
							<Icono icon="lucide:rotate-ccw" className="size-4" />
							{t("Otra partida")}
						</Button>
					)}
				</div>
			</div>
			{banquillo && <BanquilloDialog onCerrar={() => setBanquillo(false)} />}
		</div>
	);
}
