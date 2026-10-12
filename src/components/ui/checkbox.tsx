/**
 * La casilla de verificación de shadcn/ui.
 *
 * 🛑 Se monta sobre el paquete unificado `radix-ui`, que **ya está instalado**
 * (`package.json`), y no sobre `@radix-ui/react-checkbox` suelto: el entorno del
 * asistente no instala paquetes, y añadir una dependencia nueva dejaría el
 * proyecto sin arrancar en el Mac hasta que alguien corriera `npm install`. Es
 * el mismo camino que `switch.tsx` y el resto de primitivas de esta carpeta.
 *
 * 🛑 Y es una casilla, no un interruptor. Un `Switch` dice «así queda
 * configurado»; una casilla dice «esto va con lo que estoy comprando ahora», y
 * eso es exactamente lo que significa «repetir cada mes» dentro de la tarjeta
 * de un paquete.
 */

import { Check } from "lucide-react";
import { Checkbox as CheckboxPrimitive } from "radix-ui";
import type * as React from "react";

import { cn } from "@/lib/utils";

function Checkbox({ className, ...props }: React.ComponentProps<typeof CheckboxPrimitive.Root>) {
	return (
		<CheckboxPrimitive.Root
			data-slot="checkbox"
			className={cn(
				"peer size-4 shrink-0 rounded-[4px] border border-input shadow-xs transition-shadow outline-none",
				"focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/50",
				"disabled:cursor-not-allowed disabled:opacity-50",
				// 🛑 También en oscuro: el `dark:bg-input/30` de abajo ganaba a
				// `bg-primary`, y una casilla marcada parecía sin marcar.
				"data-[state=checked]:border-primary data-[state=checked]:bg-primary data-[state=checked]:text-primary-foreground dark:data-[state=checked]:bg-primary",
				"aria-invalid:border-destructive aria-invalid:ring-destructive/20 dark:bg-input/30",
				className,
			)}
			{...props}
		>
			<CheckboxPrimitive.Indicator
				data-slot="checkbox-indicator"
				className="flex items-center justify-center text-current transition-none"
			>
				<Check className="size-3" />
			</CheckboxPrimitive.Indicator>
		</CheckboxPrimitive.Root>
	);
}

export { Checkbox };
