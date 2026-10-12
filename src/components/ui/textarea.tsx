import type * as React from "react";

import { cn } from "@/lib/utils";

/**
 * Con corrector ortográfico por defecto (Pablo, 08/10/2026): en Linux el motor
 * lo trae apagado y se enciende en `ventana.rs` (`corrector_de_linux`); el
 * atributo explícito hace falta en el resto. Un campo de códigos o de claves lo
 * apaga con `spellCheck={false}`.
 */
function Textarea({ className, spellCheck = true, ...props }: React.ComponentProps<"textarea">) {
	return (
		<textarea
			data-slot="textarea"
			spellCheck={spellCheck}
			className={cn(
				"flex min-h-16 w-full rounded-md border border-input bg-transparent px-3 py-2 text-base shadow-xs transition-[color,box-shadow] outline-none placeholder:text-muted-foreground disabled:pointer-events-none disabled:cursor-not-allowed disabled:opacity-50 md:text-sm dark:bg-input/30",
				"focus-visible:border-ring focus-visible:ring-[1px] focus-visible:ring-ring/50",
				"aria-invalid:border-destructive aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40",
				className,
			)}
			{...props}
		/>
	);
}

export { Textarea };
