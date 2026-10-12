"use client";

import type { ReactElement, ReactNode } from "react";

import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";

/**
 * ⚠️ REGLA DE COMPOSICIÓN — leer antes de anidarlo con otro *Trigger de Radix.
 *
 * `WithTooltip` ya renderiza `<TooltipTrigger asChild>{children}</TooltipTrigger>`
 * por dentro. Al combinarlo con otro `*Trigger asChild` de Radix
 * (DropdownMenuTrigger, PopoverTrigger, SheetTrigger, DialogTrigger,
 * AccordionTrigger, etc.), el tooltip TIENE que ser el envoltorio EXTERIOR,
 * nunca el interior. El otro trigger espera que su hijo directo sea un
 * elemento consciente del DOM (un Button, un <a>...); un componente de React
 * como `WithTooltip` rompe la fusión de refs/handlers del `Slot` y el
 * desplegable/popover deja de abrirse.
 *
 *   ❌ Roto:                          ✅ Correcto:
 *   <DropdownMenu>                   <DropdownMenu>
 *     <DropdownMenuTrigger asChild>    <WithTooltip content="…">
 *       <WithTooltip content="…">         <DropdownMenuTrigger asChild>
 *         <Button>…</Button>                <Button>…</Button>
 *       </WithTooltip>                    </DropdownMenuTrigger>
 *     </DropdownMenuTrigger>           </WithTooltip>
 *   </DropdownMenu>                  </DropdownMenu>
 */
interface WithTooltipProps {
	/** Contenido del tooltip. Si es nullish o cadena vacía, el envoltorio renderiza los hijos sin tocar. */
	content: ReactNode;
	children: ReactElement;
	side?: "top" | "right" | "bottom" | "left";
	align?: "start" | "center" | "end";
	/** Fuerza a desactivar el tooltip (p. ej. mientras un padre está cargando). */
	disabled?: boolean;
	className?: string;
}

export function WithTooltip({
	content,
	children,
	side = "top",
	align = "center",
	disabled,
	className,
}: WithTooltipProps) {
	if (disabled || content == null || content === "" || content === false) {
		return children;
	}
	return (
		<Tooltip>
			<TooltipTrigger asChild>{children}</TooltipTrigger>
			<TooltipContent side={side} align={align} className={className}>
				{content}
			</TooltipContent>
		</Tooltip>
	);
}
