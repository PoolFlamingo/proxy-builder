import type { ReactNode } from "react";
import { Icono } from "@/components/icono";
import { useLang } from "@/components/language-provider";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import { cn } from "@/lib/utils";

/**
 * El «ⓘ» que abre un `Popover` con un toque: donde un `Tooltip` lleva
 * información de verdad (una explicación, un aviso), porque el de Radix no
 * sale con el dedo. Se enseña solo en táctil (`[data-tactil]`).
 */
export function AyudaTactil({ children, className }: { children: ReactNode; className?: string }) {
	const { t } = useLang();
	return (
		<Popover>
			<PopoverTrigger
				type="button"
				aria-label={t("Ayuda")}
				className={cn(
					"hidden size-6 shrink-0 items-center justify-center rounded-full text-muted-foreground [[data-tactil]_&]:inline-flex",
					className,
				)}
			>
				<Icono icon="lucide:info" className="size-4" />
			</PopoverTrigger>
			<PopoverContent className="w-64 text-xs">{children}</PopoverContent>
		</Popover>
	);
}
