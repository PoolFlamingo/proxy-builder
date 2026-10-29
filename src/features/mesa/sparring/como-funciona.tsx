import { Icono } from "@/components/icono";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";

/**
 * **Cómo funciona el sparring** (`capturas/playgroup/84-sparring-cutthroat.jpg`):
 * sale la primera vez que se enciende (con «Empezar el sparring») y desde
 * «Cómo funciona» del panel.
 */
export function ComoFunciona({
	abierto,
	primeraVez,
	onCerrar,
}: {
	abierto: boolean;
	primeraVez: boolean;
	onCerrar: () => void;
}) {
	const { t } = useLang();
	return (
		<Dialog open={abierto} onOpenChange={(o) => !o && onCerrar()}>
			<DialogContent className="sm:max-w-md" cambiosSinGuardar={false}>
				<DialogHeader>
					<DialogTitle className="flex items-center gap-2">
						<Icono icon="game-icons:ghost" className="size-4 text-primary" />
						{t("Cómo funciona el sparring")}
					</DialogTitle>
					<DialogDescription>
						{t(
							"Un rival fantasma reacciona a algunas de tus jugadas, como lo haría una mesa de Commander: contrarresta lo que lanzas, elimina las amenazas que llegan al campo y barre la mesa cuando pasas el turno con muchas criaturas.",
						)}
					</DialogDescription>
				</DialogHeader>
				<div className="flex flex-col gap-2.5 text-xs text-foreground/90">
					<p>
						{t(
							"No hay inteligencia artificial ni reglas: tablas de cuándo suele pasar cada cosa (más hacia la mitad de la partida) y unas 120 cartas de interacción reales.",
						)}
					</p>
					<p>
						{t(
							"Tiene sus límites: no lanza lo que no podría pagar (en la ronda N tiene N manás, salvo las gratis como Force of Will), respeta el antimaleficio y de tus tierras solo toca las muy fuertes, como Gaea's Cradle.",
						)}
					</p>
					<p>
						{t(
							"Va de buena fe: nunca mira tu mano. Tres formas de responder: «Resolverla» la aplica sola, «Tengo respuesta» la deja sin efecto y «No aplica» la quita si no tiene sentido. Intro, A y Esc hacen lo mismo.",
						)}
					</p>
				</div>
				<DialogFooter>
					<Button size="sm" onClick={onCerrar}>
						{primeraVez ? t("Empezar el sparring") : t("Entendido")}
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	);
}
