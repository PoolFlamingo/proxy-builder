import { Icono } from "@/components/icono";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

/**
 * **Lo que se ve mientras algo carga, o si no se pudo**: un giro y su texto,
 * o el error con «Reintentar». Guarda el alto (`className`) para que la lista
 * que viene después no haga saltar el diálogo.
 */
export function EstadoCarga({
	error,
	onReintentar,
	className,
}: {
	/** El error de la última carga (`null`: cargando). */
	error: unknown;
	onReintentar: () => void;
	className?: string;
}) {
	const { t, mensaje } = useLang();
	return (
		<div
			className={cn(
				"flex flex-col items-center justify-center gap-2.5 text-xs text-muted-foreground",
				className,
			)}
		>
			{error ? (
				<>
					<p className="text-center text-destructive">{mensaje(error)}</p>
					<Button size="sm" variant="outline" onClick={onReintentar}>
						<Icono icon="lucide:refresh-cw" className="size-4" />
						{t("Reintentar")}
					</Button>
				</>
			) : (
				<>
					<Icono icon="lucide:loader-2" className="size-5 animate-spin" />
					{t("Cargando…")}
				</>
			)}
		</div>
	);
}
