import type { ComponentProps } from "react";
import { DialogContent } from "@/components/ui/dialog";
import { cn } from "@/lib/utils";

/**
 * Un panel que sube desde abajo (o entra por la derecha, en tableta) para
 * menús y paneles en móvil. Es un `DialogContent` con otra colocación, así que
 * se usa dentro de un `Dialog` y hereda todo: foco, `Esc`, el botón «atrás» de
 * Android (cierra la capa de arriba, `lib/atras.ts`) y la pregunta de salir
 * si hay algo escrito.
 */
export function Hoja({
	lado = "abajo",
	className,
	...props
}: ComponentProps<typeof DialogContent> & { lado?: "abajo" | "derecha" }) {
	return (
		<DialogContent
			className={cn(
				lado === "abajo"
					? "top-auto bottom-0 left-0 w-full max-w-none max-h-[85dvh] translate-x-0 translate-y-0 rounded-b-none rounded-t-[14px] border-x-0 border-b-0 pb-[max(1.5rem,env(safe-area-inset-bottom))] data-[state=closed]:slide-out-to-bottom data-[state=open]:slide-in-from-bottom data-[state=closed]:zoom-out-100 data-[state=open]:zoom-in-100"
					: "top-0 right-0 left-auto h-dvh w-[min(26rem,100%)] max-w-none max-h-none translate-x-0 translate-y-0 rounded-r-none pt-[max(1.5rem,env(safe-area-inset-top))] data-[state=closed]:slide-out-to-right data-[state=open]:slide-in-from-right data-[state=closed]:zoom-out-100 data-[state=open]:zoom-in-100",
				className,
			)}
			{...props}
		/>
	);
}
