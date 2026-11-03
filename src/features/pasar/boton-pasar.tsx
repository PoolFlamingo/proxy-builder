import { Icono } from "@/components/icono";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import { useFormato } from "@/lib/formato";
import { usePasarUi } from "./store";

/** «Enviar / Recibir» en la barra de la biblioteca (en móvil, solo el icono). */
export function BotonPasar() {
	const { t } = useLang();
	const movil = useFormato().tamano === "movil";
	return (
		<Button
			size={movil ? "icon-sm" : "sm"}
			variant="outline"
			onClick={() => usePasarUi.getState().abrirPasar()}
			aria-label={t("Enviar / Recibir")}
		>
			<Icono icon="lucide:arrow-left-right" />
			{!movil && t("Enviar / Recibir")}
		</Button>
	);
}
