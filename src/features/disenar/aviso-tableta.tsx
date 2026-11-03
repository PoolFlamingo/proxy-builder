import { DialogoAdaptable } from "@/components/dialogo-adaptable";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import {
	DialogClose,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";

/**
 * Lo que no entra en el móvil (ajustes finos, letras y packs, colecciones,
 * diseñar en lote, exportar imágenes): si alguien llega hasta ahí, este
 * aviso en vez del diálogo entero. La interfaz no enseña un camino hasta
 * ellos; esto es la red por si llega uno (un enlace, un atajo).
 */
export function AvisoSoloTableta() {
	const { t } = useLang();
	return (
		<DialogoAdaptable className="sm:max-w-sm">
			<DialogHeader>
				<DialogTitle>{t("Esto se hace en una tableta o en el ordenador")}</DialogTitle>
				<DialogDescription className="text-xs">
					{t("En el móvil el editor solo trae lo básico: textos, ilustración, marco y colores.")}
				</DialogDescription>
			</DialogHeader>
			<DialogFooter>
				<DialogClose asChild>
					<Button size="sm">{t("Entendido")}</Button>
				</DialogClose>
			</DialogFooter>
		</DialogoAdaptable>
	);
}
