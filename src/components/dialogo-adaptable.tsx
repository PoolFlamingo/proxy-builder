import type { ComponentProps } from "react";
import { DialogContent } from "@/components/ui/dialog";
import { useFormato } from "@/lib/formato";
import { cn } from "@/lib/utils";

/**
 * Cómo se ve un diálogo en un móvil: a pantalla completa, con las zonas
 * seguras, la X de cerrar de 44 px arriba a la derecha (a la altura del
 * título), el título sin pisarla y los botones del pie apilados y pegados
 * abajo, con el principal —el último— en el fondo.
 */
export const CLASES_MOVIL = cn(
	// Con el teléfono en horizontal el ancho ya es de `sm:`: los `sm:max-w-*`
	// de quien llama también se anulan.
	"sm:max-w-none md:max-w-none lg:max-w-none xl:max-w-none 2xl:max-w-none",
	"top-0 left-0 h-dvh w-full max-w-none max-h-none translate-x-0 translate-y-0 rounded-none border-0 px-4 pt-[max(1rem,env(safe-area-inset-top))] pb-[max(1rem,env(safe-area-inset-bottom))] data-[state=closed]:zoom-out-100 data-[state=open]:zoom-in-100",
	// La X de cerrar de `DialogContent`: 44 px, a la altura de la primera línea.
	"[&>[data-slot=dialog-close]]:top-[calc(max(1rem,env(safe-area-inset-top))_-_0.75rem)] [&>[data-slot=dialog-close]]:right-1 [&>[data-slot=dialog-close]]:flex [&>[data-slot=dialog-close]]:size-11 [&>[data-slot=dialog-close]]:items-center [&>[data-slot=dialog-close]]:justify-center",
	"[&_[data-slot=dialog-header]]:pr-11 [&_[data-slot=dialog-header]]:text-left",
	"[&_[data-slot=dialog-footer]]:mt-auto [&_[data-slot=dialog-footer]]:flex-col [&_[data-slot=dialog-footer]]:items-stretch [&_[data-slot=dialog-footer]]:justify-end",
);

/**
 * El `DialogContent` de siempre en escritorio y tableta, y a pantalla completa
 * en móvil (con las zonas seguras). Los diálogos lo usan en vez de llamar a
 * `DialogContent` directamente; `components/ui/` no se toca.
 *
 * `compacto`: un aviso de pocas líneas que en móvil se queda como tarjeta
 * centrada (con la anchura de la pantalla) en vez de ocuparla entera.
 */
export function DialogoAdaptable({
	className,
	compacto = false,
	...props
}: ComponentProps<typeof DialogContent> & { compacto?: boolean }) {
	const { tamano } = useFormato();
	return (
		<DialogContent
			className={cn(className, tamano === "movil" && !compacto && CLASES_MOVIL)}
			{...props}
		/>
	);
}
