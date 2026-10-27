import { Flag } from "lucide-react";
import { useState } from "react";
import type { ComoGano } from "@/bindings/ComoGano";
import type { IdJugador } from "@/bindings/IdJugador";
import { Selector } from "@/components/campos";
import { paraTraducir, useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { cn } from "@/lib/utils";
import { useMesa } from "@/stores/mesa-store";
import { hacer } from "../acciones";
import { ganadoresEnPie } from "../cuentas-fin";
import { usePantalla } from "../pantalla";

export const COMO: { valor: ComoGano; nombre: string }[] = [
	{ valor: "combate", nombre: paraTraducir("Combate") },
	{ valor: "combo", nombre: paraTraducir("Combo") },
	{ valor: "comandante", nombre: paraTraducir("Daño de comandante") },
	{ valor: "veneno", nombre: paraTraducir("Veneno") },
	{ valor: "moler", nombre: paraTraducir("Moler") },
	{ valor: "alternativa", nombre: paraTraducir("Victoria alternativa") },
	{ valor: "noCombate", nombre: paraTraducir("Daño sin combate") },
	{ valor: "concesion", nombre: paraTraducir("Concesión") },
	{ valor: "empate", nombre: paraTraducir("Empate") },
];

/**
 * **Terminar la partida** (`capturas/pablo/21.jpg`, sin valoraciones): quién
 * sigue en pie y quién quedó fuera, quién ganó (o varios: empate o un
 * equipo) y cómo. Terminar es definitivo; después, «Otra partida» o salir.
 */
export function TerminarDialog() {
	const { t } = useLang();
	const asientos = useMesa((s) => s.asientos);
	const jugadores = useMesa((s) => s.jugadores);
	const solitario = asientos.length === 1;
	// Si solo queda uno en pie (o un equipo), se propone como ganador.
	const [ganadores, setGanadores] = useState<IdJugador[]>(() =>
		solitario
			? [asientos[0] ?? 0]
			: ganadoresEnPie(asientos, (id) => {
					const j = useMesa.getState().jugadores[id];
					return j ? { eliminado: j.eliminado != null, equipo: j.equipo } : undefined;
				}),
	);
	const [como, setComo] = useState<ComoGano>("combate");
	const [nota, setNota] = useState("");
	const cerrar = () => usePantalla.getState().abrir(null);
	const alternar = (j: IdJugador) =>
		setGanadores((g) => (g.includes(j) ? g.filter((x) => x !== j) : [...g, j]));
	return (
		<Dialog open onOpenChange={(o) => !o && cerrar()}>
			<DialogContent className="sm:max-w-md" cambiosSinGuardar={false}>
				<DialogHeader>
					<DialogTitle>{t("¿Quién ganó?")}</DialogTitle>
					<DialogDescription>
						{t("Terminar es definitivo: la partida ya no se puede seguir.")}
					</DialogDescription>
				</DialogHeader>
				<div className="flex flex-col gap-1.5">
					{asientos.map((id) => {
						const j = jugadores[id];
						if (!j) return null;
						return (
							<button
								key={id}
								type="button"
								onClick={() => alternar(id)}
								className={cn(
									"flex items-center justify-between rounded-lg border px-2.5 py-1.5 text-xs",
									ganadores.includes(id) && "border-primary bg-primary/10",
								)}
							>
								<span className="flex items-center gap-1.5">
									<span className="size-2.5 rounded-full" style={{ background: j.color }} />
									{j.nombre}
								</span>
								<span className="text-muted-foreground">
									{!j.eliminado
										? t("Sigue en pie")
										: j.eliminado.por === null || jugadores[j.eliminado.por] === undefined
											? t("Fuera en la ronda {r}", { r: j.eliminado.ronda })
											: t("Fuera en la ronda {r}, por {quien}", {
													r: j.eliminado.ronda,
													quien: jugadores[j.eliminado.por]?.nombre ?? "?",
												})}
								</span>
							</button>
						);
					})}
					{solitario && (
						<button
							type="button"
							onClick={() => setGanadores([])}
							className={cn(
								"rounded-lg border px-2.5 py-1.5 text-left text-xs",
								ganadores.length === 0 && "border-primary bg-primary/10",
							)}
						>
							{t("El rival (perdí)")}
						</button>
					)}
				</div>
				<Selector<ComoGano>
					valor={como}
					onChange={setComo}
					opciones={COMO.map((c) => ({ valor: c.valor, etiqueta: t(c.nombre) }))}
					ancho="w-full"
				/>
				<Input
					value={nota}
					onChange={(e) => setNota(e.target.value)}
					maxLength={200}
					placeholder={t("Una nota (opcional)")}
					className="h-8 text-xs"
				/>
				<DialogFooter>
					<Button variant="outline" onClick={cerrar}>
						{t("Cancelar")}
					</Button>
					<Button
						onClick={() => {
							cerrar();
							void hacer({ que: "terminar", resultado: { ganadores, como, nota } });
						}}
					>
						<Flag className="size-4" />
						{t("Terminar")}
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	);
}
