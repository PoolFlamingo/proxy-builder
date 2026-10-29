import { Icono } from "@/components/icono";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import { WithTooltip } from "@/components/ui/with-tooltip";
import { guardarAhora, useDeck } from "@/stores/deck-store";

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
				<Icono icon="lucide:check" className="size-3.5 text-primary" />
				{t("Guardado")}
			</>
		),
		saving: (
			<>
				<Icono icon="lucide:loader-2" className="size-3.5 animate-spin" />
				{t("Guardando…")}
			</>
		),
		dirty: <>{t("Sin guardar")}</>,
		error: (
			<>
				<Icono icon="lucide:alert-triangle" className="size-3.5 text-destructive" />
				{t("Error al guardar")}
			</>
		),
	}[guardado];

	const etiqueta = (
		<span className="flex items-center gap-1.5 text-[11px] text-muted-foreground">{contenido}</span>
	);
	// Con error, se puede volver a intentar ahí mismo (el tooltip dice qué pasó).
	return guardado === "error" ? (
		<WithTooltip content={`${mensaje(error)} ${t("Pulsa para reintentar.")}`}>
			<Button
				size="xs"
				variant="ghost"
				className="text-[11px] text-muted-foreground"
				onClick={() => void guardarAhora().catch(() => {})}
			>
				{contenido}
			</Button>
		</WithTooltip>
	) : (
		etiqueta
	);
}
