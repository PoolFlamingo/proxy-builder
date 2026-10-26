import { Undo2 } from "lucide-react";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { ScrollArea } from "@/components/ui/scroll-area";
import { WithTooltip } from "@/components/ui/with-tooltip";
import { localeDe } from "@/lib/idiomas";
import { useMesa } from "@/stores/mesa-store";
import { hacer } from "../acciones";
import { usePantalla } from "../pantalla";
import { textoDeEvento } from "../textos-eventos";

/**
 * **El registro de la partida**: cada evento en una frase, con su hora y
 * «Deshacer hasta aquí» en solitario.
 */
export function RegistroDialog() {
	const { t, lang } = useLang();
	const registro = useMesa((s) => s.registro);
	const cartas = useMesa((s) => s.cartas);
	const jugadores = useMesa((s) => s.jugadores);
	const solitario = useMesa((s) => s.asientos.length === 1);
	const hora = new Intl.DateTimeFormat(localeDe(lang), {
		hour: "2-digit",
		minute: "2-digit",
		second: "2-digit",
	});
	return (
		<Dialog open onOpenChange={(o) => !o && usePantalla.getState().abrir(null)}>
			<DialogContent className="sm:max-w-lg">
				<DialogHeader>
					<DialogTitle>{t("Registro de la partida")}</DialogTitle>
					<DialogDescription>{t("Lo último, arriba.")}</DialogDescription>
				</DialogHeader>
				<ScrollArea className="h-[60vh]">
					<div className="flex flex-col gap-0.5 p-1 text-xs">
						{[...registro].reverse().map((ev) => (
							<div
								key={ev.seq}
								className="group flex items-center gap-2 rounded px-1.5 py-1 hover:bg-muted/60"
							>
								<span className="w-16 shrink-0 text-[10px] text-muted-foreground tabular-nums">
									{hora.format(new Date(Number(ev.tMs)))}
								</span>
								<span className="flex-1">{textoDeEvento(ev, { cartas, jugadores }, t)}</span>
								{solitario && ev.cambio.tipo !== "restaurar" && (
									<WithTooltip content={t("Deshacer hasta aquí")}>
										<Button
											size="icon-sm"
											variant="ghost"
											className="opacity-0 group-hover:opacity-100"
											onClick={() => void hacer({ que: "deshacer", hasta: Number(ev.seq) })}
											aria-label={t("Deshacer hasta aquí")}
										>
											<Undo2 className="size-3.5" />
										</Button>
									</WithTooltip>
								)}
							</div>
						))}
						{registro.length === 0 && (
							<p className="p-4 text-center text-muted-foreground">
								{t("Todavía no ha pasado nada.")}
							</p>
						)}
					</div>
				</ScrollArea>
			</DialogContent>
		</Dialog>
	);
}
