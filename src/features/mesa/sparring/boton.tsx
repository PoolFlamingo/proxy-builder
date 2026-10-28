import { Ghost } from "lucide-react";
import { useState } from "react";
import type { Intensidad } from "@/bindings/Intensidad";
import { useLang } from "@/components/language-provider";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import { ScrollArea } from "@/components/ui/scroll-area";
import { cn } from "@/lib/utils";
import { useMesa } from "@/stores/mesa-store";
import { useUi } from "@/stores/ui-store";
import { hacer } from "../acciones";
import { ComoFunciona } from "./como-funciona";
import { INTENSIDADES, nombreIntensidad, resumen } from "./textos";
import { useSenalarFantasma } from "./vista-fantasma";

const NINGUNA: never[] = [];

/** El nombre de la carta de una reacción: al pasar el ratón sale en grande. */
function NombreSenalable({ nombre }: { nombre: string }) {
	const senalar = useSenalarFantasma(nombre);
	return (
		<p className="w-fit max-w-full cursor-zoom-in truncate font-medium" {...senalar}>
			{nombre}
		</p>
	);
}

/**
 * **El botón «Sparring»** de la barra (`capturas/pablo/00.jpg`,
 * `capturas/playgroup/83-sparring-panel.jpg`): la intensidad, y al abrirlo el
 * panel con la intensidad, las reacciones de esta partida y «Cómo funciona».
 * Solo en solitario. Cambiar la intensidad vale desde la siguiente ocasión.
 */
export function BotonSparring() {
	const { t } = useLang();
	const solo = useMesa((s) => s.asientos.length === 1);
	const intensidad = useMesa((s) => s.general?.sparring.intensidad ?? "apagado");
	const reacciones = useMesa((s) => s.general?.sparring.reacciones ?? NINGUNA);
	const cartas = useMesa((s) => s.cartas);
	const [comoFunciona, setComoFunciona] = useState<"no" | "ver" | "primera">("no");
	if (!solo) return null;

	const elegir = (i: Intensidad) => {
		void hacer({ que: "intensidadSparring", intensidad: i });
		const ui = useUi.getState();
		if (i !== "apagado" && !ui.mesa.sparringVisto) {
			ui.setMesa({ sparringVisto: true });
			setComoFunciona("primera");
		}
	};
	const encendido = intensidad !== "apagado";
	const nombre = (id: number | undefined) =>
		id === undefined ? "" : (cartas[id]?.origen?.nombre ?? t("una carta"));

	return (
		<>
			<Popover>
				<PopoverTrigger asChild>
					<button
						type="button"
						className={cn(
							"flex h-7 items-center gap-1.5 rounded-lg border px-2 text-left uppercase",
							encendido
								? "border-primary/60 bg-primary/15 text-primary"
								: "border-white/15 text-white/80 hover:bg-white/10",
						)}
						data-tutorial="mesa-sparring"
					>
						<Ghost className="size-3.5 shrink-0" />
						<span className="flex flex-col leading-none">
							<span className="text-[10px] font-semibold">{t("Sparring")}</span>
							<span className={cn("text-[9px]", !encendido && "text-white/50")}>
								{nombreIntensidad(intensidad, t)}
							</span>
						</span>
					</button>
				</PopoverTrigger>
				<PopoverContent align="end" className="flex w-80 flex-col gap-2.5 text-xs">
					<p className="text-muted-foreground">
						{t(
							"Un rival fantasma responde a algunas de tus jugadas con contrahechizos, eliminaciones y barridos.",
						)}
					</p>
					<h4 className="text-[11px] font-semibold text-muted-foreground uppercase">
						{t("Intensidad")}
					</h4>
					<div className="flex flex-wrap gap-1.5">
						{INTENSIDADES.map((i) => (
							<button
								key={i.valor}
								type="button"
								onClick={() => elegir(i.valor)}
								className={cn(
									"rounded-full border px-2.5 py-1",
									i.valor === intensidad
										? "border-primary bg-primary/15 font-semibold text-primary"
										: "hover:bg-muted",
								)}
							>
								{t(i.nombre)}
							</button>
						))}
					</div>
					<h4 className="text-[11px] font-semibold text-muted-foreground uppercase">
						{t("Reacciones de esta partida")}
					</h4>
					{reacciones.length === 0 ? (
						<p className="text-muted-foreground">
							{t(
								"Todavía ninguna. Sigue jugando y el fantasma responderá a algunas de tus jugadas.",
							)}
						</p>
					) : (
						<ScrollArea viewportClassName="max-h-56">
							<ul className="flex flex-col gap-1.5 p-1">
								{[...reacciones].reverse().map((r) => (
									<li
										key={r.reaccion.id}
										className="flex items-center gap-2 rounded-md border bg-muted/40 px-2 py-1.5"
									>
										<div className="min-w-0 flex-1">
											<NombreSenalable nombre={r.reaccion.carta.nombre} />
											<p className="truncate text-muted-foreground">
												{resumen(r.reaccion, nombre(r.reaccion.objetivo[0]), t)}
											</p>
										</div>
										<span className="rounded border px-1 font-mono text-[10px]">
											{t("R{ronda}", { ronda: r.ronda })}
										</span>
										<span
											className={cn(
												"text-[10px] font-semibold uppercase",
												r.decision === "aceptar" ? "text-orange-500" : "text-muted-foreground",
											)}
										>
											{r.decision === "aceptar"
												? t("Resuelta")
												: r.decision === "teniaRespuesta"
													? t("Tenías respuesta")
													: r.decision === "noAplica"
														? t("No aplicaba")
														: t("Pendiente")}
										</span>
									</li>
								))}
							</ul>
						</ScrollArea>
					)}
					<button
						type="button"
						onClick={() => setComoFunciona("ver")}
						className="self-start font-semibold text-primary uppercase underline-offset-2 hover:underline"
					>
						{t("Cómo funciona")}
					</button>
				</PopoverContent>
			</Popover>
			<ComoFunciona
				abierto={comoFunciona !== "no"}
				primeraVez={comoFunciona === "primera"}
				onCerrar={() => setComoFunciona("no")}
			/>
		</>
	);
}
