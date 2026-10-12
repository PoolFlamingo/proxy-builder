"use client";

import {
	CircleCheckIcon,
	InfoIcon,
	Loader2Icon,
	OctagonXIcon,
	TriangleAlertIcon,
} from "lucide-react";
import { Toaster as Sonner, type ToasterProps } from "sonner";
import { useTheme } from "@/components/theme-provider";

/**
 * Avisos emergentes.
 *
 * OJO: la plantilla de shadcn lee el tema de `next-themes`, que esta app NO
 * monta. El resultado era que el toast seguía al tema del SISTEMA en vez de al
 * selector de la app: con Windows en oscuro y la app en claro, salía una caja
 * grisácea que parecía una mancha. Aquí se toma el tema ya resuelto por nuestro
 * ThemeProvider, que es la única fuente de verdad.
 */
const Toaster = ({ ...props }: ToasterProps) => {
	const { resolvedMode } = useTheme();

	return (
		<Sonner
			theme={resolvedMode}
			className="toaster group"
			// 🛑 Un botón de cerrar en CADA aviso (Pablo, 02/10/2026): los que llevan
			// texto largo tardan en irse y tapan lo de debajo. Sonner lo pinta por
			// defecto en la esquina de fuera y solo al pasar el ratón; aquí va dentro,
			// arriba a la derecha y siempre a la vista.
			closeButton
			toastOptions={{
				classNames: {
					// Hueco a la derecha para que la X no pise el texto.
					toast: "!pr-9",
					closeButton:
						"!left-auto !right-2 !top-2 !transform-none !border-0 !bg-transparent !opacity-100 text-muted-foreground hover:!bg-muted hover:text-foreground",
				},
			}}
			icons={{
				success: <CircleCheckIcon className="size-4" />,
				info: <InfoIcon className="size-4" />,
				warning: <TriangleAlertIcon className="size-4" />,
				error: <OctagonXIcon className="size-4" />,
				loading: <Loader2Icon className="size-4 animate-spin" />,
			}}
			style={
				{
					"--normal-bg": "var(--popover)",
					"--normal-text": "var(--popover-foreground)",
					"--normal-border": "var(--border)",
					"--border-radius": "var(--radius)",
				} as React.CSSProperties
			}
			{...props}
		/>
	);
};

export { Toaster };
