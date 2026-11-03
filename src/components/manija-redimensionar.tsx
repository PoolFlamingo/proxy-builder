import type { ComponentProps } from "react";
import { ResizableHandle } from "@/components/ui/resizable";
import { cn } from "@/lib/utils";

/**
 * El separador entre paneles (`ResizableHandle`), más grueso con el dedo: en
 * táctil mide 8 px a la vista, con 32 de zona que se agarra, y el pomo
 * crece. Con ratón es el de siempre. `components/ui/` no se toca.
 */
export function ManijaRedimensionar({
	className,
	...props
}: ComponentProps<typeof ResizableHandle>) {
	return (
		<ResizableHandle
			className={cn(
				"[[data-tactil]_&]:w-2 [[data-tactil]_&]:after:w-8 [[data-tactil]_&]:aria-[orientation=horizontal]:h-2 [[data-tactil]_&]:aria-[orientation=horizontal]:w-full [[data-tactil]_&]:aria-[orientation=horizontal]:after:h-8 [[data-tactil]_&]:aria-[orientation=horizontal]:after:w-full [[data-tactil]_&>div]:h-8 [[data-tactil]_&>div]:w-5",
				className,
			)}
			{...props}
		/>
	);
}
