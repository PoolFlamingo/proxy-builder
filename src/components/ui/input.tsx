import type * as React from "react";

import { cn } from "@/lib/utils";

/** Los tipos en los que se escribe texto corrido: ahí sí se corrige. */
const TIPOS_DE_TEXTO = new Set([undefined, "text", "search"]);

/**
 * Con corrector ortográfico por defecto en los campos de texto (Pablo,
 * 08/10/2026; en Linux lo enciende `ventana.rs`, `corrector_de_linux`). No en
 * los que no son texto (clave, correo, URL, número, fecha…) ni en los de
 * letra monoespaciada, que en JUDLEX son siempre códigos (claves, hashes,
 * rutas). Un campo de códigos sin `font-mono` (un NIF, una referencia) lo apaga
 * con `spellCheck={false}`.
 */
function Input({ className, type, spellCheck, ...props }: React.ComponentProps<"input">) {
	const corregir =
		spellCheck ?? (TIPOS_DE_TEXTO.has(type) && !/\bfont-mono\b/.test(className ?? ""));
	return (
		<input
			type={type}
			spellCheck={corregir}
			data-slot="input"
			className={cn(
				"h-8 w-full min-w-0 rounded-md border border-input bg-transparent px-3 py-1 text-base shadow-xs transition-[color,box-shadow] outline-none selection:bg-primary selection:text-primary-foreground file:inline-flex file:h-7 file:border-0 file:bg-transparent file:text-sm file:font-medium file:text-foreground placeholder:text-muted-foreground disabled:pointer-events-none disabled:cursor-not-allowed disabled:opacity-50 md:text-sm dark:bg-input/30",
				"focus-visible:border-ring focus-visible:ring-[1px] focus-visible:ring-ring/50",
				"aria-invalid:border-destructive aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40",
				className,
			)}
			{...props}
		/>
	);
}

export { Input };
