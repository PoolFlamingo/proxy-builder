import { toast } from "sonner";
import { Icono } from "@/components/icono";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import { WithTooltip } from "@/components/ui/with-tooltip";
import { guardarAhora, useDeck } from "@/stores/deck-store";

/**
 * «Guardado / Guardando… / Sin guardar / Error al guardar», en la barra superior.
 * `compacto` (móvil): solo el icono; el texto queda para el lector de pantalla.
 */
export function SaveIndicator({ compacto = false }: { compacto?: boolean }) {
	const { t, mensaje } = useLang();
	const guardado = useDeck((s) => s.guardado);
	const error = useDeck((s) => s.errorGuardado);
	const nombre = useDeck((s) => s.deck?.name);
	if (!nombre) return null;

	const contenido = {
		saved: (
			<>
				<Icono icon="lucide:check" className="size-3.5 text-primary" />
				<Texto compacto={compacto}>{t("Guardado")}</Texto>
			</>
		),
		saving: (
			<>
				<Icono icon="lucide:loader-2" className="size-3.5 animate-spin" />
				<Texto compacto={compacto}>{t("Guardando…")}</Texto>
			</>
		),
		dirty: compacto ? (
			<>
				<Icono icon="lucide:circle-dot" className="size-3.5" />
				<Texto compacto>{t("Sin guardar")}</Texto>
			</>
		) : (
			t("Sin guardar")
		),
		error: (
			<>
				<Icono icon="lucide:alert-triangle" className="size-3.5 text-destructive" />
				<Texto compacto={compacto}>{t("Error al guardar")}</Texto>
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
				// táctil: sin tooltip, al tocarlo se reintenta y el motivo sale en un aviso (`toast.error`)
				onClick={() =>
					void guardarAhora().catch((e) => {
						toast.error(mensaje(e));
					})
				}
			>
				{contenido}
			</Button>
		</WithTooltip>
	) : (
		etiqueta
	);
}

function Texto({ compacto, children }: { compacto: boolean; children: string }) {
	return compacto ? <span className="sr-only">{children}</span> : children;
}
