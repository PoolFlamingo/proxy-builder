import { Icono } from "@/components/icono";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import { WithTooltip } from "@/components/ui/with-tooltip";
import {
	alternarPantallaCompleta,
	cerrarVentana,
	esMac,
	minimizarVentana,
	useVentana,
} from "@/lib/ventana";

/**
 * **Minimizar, salir de pantalla completa y cerrar**, al final de la barra de
 * arriba. A pantalla completa el sistema esconde los suyos y sin esto no habría
 * cómo cerrar ni minimizar con el ratón (como la barra de VS Code).
 *
 * Solo existe mientras dura la pantalla completa: no ocupa sitio en la barra el
 * resto del tiempo y entra con un fundido corto (nada de saltos al pulsar F11).
 * En macOS no se pintan: su pantalla completa ya baja los botones del sistema.
 * «Cerrar» pide cerrar y `ConfirmarCierre` decide (guarda y, si hay algo en
 * curso, pregunta), igual que la X del sistema.
 */
export function ControlesVentana() {
	const { t } = useLang();
	const completa = useVentana((s) => s.completa);
	if (!completa || esMac()) return null;
	return (
		<div className="flex items-center gap-0.5 border-l pl-2 duration-150 animate-in fade-in-0 slide-in-from-right-2 motion-reduce:animate-none">
			<WithTooltip content={t("Minimizar")}>
				<Button
					size="icon-sm"
					variant="ghost"
					onClick={() => void minimizarVentana()}
					aria-label={t("Minimizar")}
				>
					<Icono icon="lucide:minus" className="size-4" />
				</Button>
			</WithTooltip>
			<WithTooltip content={t("Salir de pantalla completa (F11)")}>
				<Button
					size="icon-sm"
					variant="ghost"
					onClick={() => void alternarPantallaCompleta()}
					aria-label={t("Salir de pantalla completa (F11)")}
				>
					<Icono icon="lucide:minimize" className="size-4" />
				</Button>
			</WithTooltip>
			<WithTooltip content={t("Cerrar")}>
				<Button
					size="icon-sm"
					variant="ghost"
					onClick={() => void cerrarVentana()}
					aria-label={t("Cerrar")}
					className="hover:bg-destructive hover:text-white dark:hover:bg-destructive"
				>
					<Icono icon="lucide:x" className="size-4" />
				</Button>
			</WithTooltip>
		</div>
	);
}
