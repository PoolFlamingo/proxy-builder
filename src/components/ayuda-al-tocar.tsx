import type { ReactElement, ReactNode } from "react";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import { WithTooltip } from "@/components/ui/with-tooltip";
import { useFormato } from "@/lib/formato";

/**
 * Lo que hace `WithTooltip` con el ratón (una ayuda al pasar por encima) y, con
 * el dedo, un `Popover` al tocar el mismo elemento, porque el `Tooltip` de
 * Radix no sale con un toque. Para marcas e iconos sin acción propia que
 * llevan información de verdad (de quién es el daño de comandante, quién mira
 * la partida…); un botón con su propia acción lleva el nombre en `aria-label`
 * (`etiquetas-tactiles.tsx` lo enseña al mantenerlo pulsado), y la ayuda larga
 * de un ajuste, `AyudaTactil`.
 */
export function AyudaAlTocar({
	ayuda,
	children,
	side = "top",
}: {
	ayuda: ReactNode;
	children: ReactElement;
	side?: "top" | "right" | "bottom" | "left";
}) {
	const { tactil } = useFormato();
	if (!tactil) {
		return (
			<WithTooltip content={ayuda} side={side}>
				{children}
			</WithTooltip>
		);
	}
	return (
		<Popover>
			<PopoverTrigger asChild>{children}</PopoverTrigger>
			<PopoverContent side={side} className="w-auto max-w-64 p-2 text-xs">
				{ayuda}
			</PopoverContent>
		</Popover>
	);
}
