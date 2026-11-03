import type { ComponentProps, ReactNode } from "react";
import { cn } from "@/lib/utils";

/**
 * **La barra de arriba de una pantalla del móvil**, igual en todas: 3 rem de
 * alto, el título a la izquierda (se recorta si no cabe) y las acciones a la
 * derecha como botones de icono (`size="icon-sm"`, con `aria-label`), la
 * principal con texto y lo secundario en un «⋯» (`BotonMenu`). Nunca se sale
 * por el lado: el título encoge y las acciones no.
 */
export function BarraSuperior({
	titulo,
	inicio,
	children,
	className,
	...props
}: {
	titulo?: ReactNode;
	/** Lo que va antes del título (un icono, un botón). */
	inicio?: ReactNode;
	/** Las acciones, a la derecha. */
	children?: ReactNode;
} & ComponentProps<"div">) {
	return (
		<div
			className={cn(
				"flex h-12 min-w-0 shrink-0 items-center gap-1 border-b bg-background px-2",
				className,
			)}
			{...props}
		>
			{inicio}
			{titulo != null && (
				<h2 className="min-w-0 flex-1 truncate px-1 text-sm font-semibold">{titulo}</h2>
			)}
			{children && <div className="ml-auto flex shrink-0 items-center gap-1">{children}</div>}
		</div>
	);
}
