import type { ReactNode } from "react";
import { Icono } from "@/components/icono";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";

export interface EntradaSeccion {
	id: string;
	titulo: string;
	/** Un id de Iconify. */
	icono: string;
	detalle?: ReactNode;
}

/**
 * La lista que sustituye a las pestañas (o a las secciones apiladas) de un
 * diálogo en móvil: una fila grande por sección, que entra en ella. Para salir
 * de la sección, `BarraSeccion`; el «atrás» de Android lo gestiona el diálogo
 * con `volverConEscape` (`lib/seccion-movil.ts`).
 */
export function ListaSecciones({
	secciones,
	onElegir,
}: {
	secciones: EntradaSeccion[];
	onElegir: (id: string) => void;
}) {
	return (
		<ul className="flex flex-col gap-2.5 p-1">
			{secciones.map((s) => (
				<li key={s.id}>
					<button
						type="button"
						onClick={() => onElegir(s.id)}
						className="flex min-h-12 w-full items-center gap-2.5 rounded-lg border bg-card/40 px-3 py-2 text-left outline-none transition-colors hover:bg-accent focus-visible:ring-2 focus-visible:ring-ring"
					>
						<Icono icon={s.icono} className="size-4 shrink-0 text-muted-foreground" />
						<span className="flex min-w-0 flex-1 flex-col">
							<span className="truncate text-xs font-medium">{s.titulo}</span>
							{s.detalle && (
								<span className="truncate text-[11px] text-muted-foreground">{s.detalle}</span>
							)}
						</span>
						<Icono icon="lucide:chevron-right" className="size-4 shrink-0 text-muted-foreground" />
					</button>
				</li>
			))}
		</ul>
	);
}

/** La fila de arriba de una sección abierta: volver a la lista y su título. */
export function BarraSeccion({ titulo, onVolver }: { titulo: string; onVolver: () => void }) {
	const { t } = useLang();
	return (
		<div className="flex items-center gap-1">
			<Button
				type="button"
				size="icon"
				variant="ghost"
				onClick={onVolver}
				aria-label={t("Volver a la lista")}
			>
				<Icono icon="lucide:chevron-left" />
			</Button>
			<span className="min-w-0 flex-1 truncate text-sm font-semibold">{titulo}</span>
		</div>
	);
}
