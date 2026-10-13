import { AlertTriangle, Check, Loader2 } from "lucide-react";
import { useLang } from "@/components/language-provider";
import { WithTooltip } from "@/components/ui/with-tooltip";
import { useDeck } from "@/stores/deck-store";

/** «Guardado / Guardando… / Sin guardar / Error al guardar», en la barra superior. */
export function SaveIndicator() {
	const { t, mensaje } = useLang();
	const guardado = useDeck((s) => s.guardado);
	const error = useDeck((s) => s.errorGuardado);
	const nombre = useDeck((s) => s.deck?.name);
	if (!nombre) return null;

	const contenido = {
		saved: (
			<>
				<Check className="size-3.5 text-primary" />
				{t("Guardado")}
			</>
		),
		saving: (
			<>
				<Loader2 className="size-3.5 animate-spin" />
				{t("Guardando…")}
			</>
		),
		dirty: <>{t("Sin guardar")}</>,
		error: (
			<>
				<AlertTriangle className="size-3.5 text-destructive" />
				{t("Error al guardar")}
			</>
		),
	}[guardado];

	const etiqueta = (
		<span className="flex items-center gap-1.5 text-[11px] text-muted-foreground">{contenido}</span>
	);
	return guardado === "error" ? (
		<WithTooltip content={mensaje(error)}>{etiqueta}</WithTooltip>
	) : (
		etiqueta
	);
}
