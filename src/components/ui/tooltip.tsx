"use client";

import { Tooltip as TooltipPrimitive } from "radix-ui";
import type * as React from "react";

import { cn } from "@/lib/utils";

/**
 * El retardo de TODOS los tooltips de la app, en ms: el ratón tiene que estar
 * encima este tiempo antes de que salga el globo, y si se va antes no sale
 * (Radix cancela el temporizador al salir). Lo usan el `TooltipProvider` (que
 * lo trae por defecto) y `TooltipsNativos`, que abre el suyo a mano. **Es el
 * único sitio donde está**: no se pasa `delayDuration` a mano en ningún otro
 * (Pablo, 08/10/2026).
 */
export const RETARDO_TOOLTIPS = 700;

function TooltipProvider({
	delayDuration = RETARDO_TOOLTIPS,
	// Radix deja abrir al instante el tooltip siguiente si se llega a él antes de
	// 300 ms desde que se cerró el anterior. Aquí no: siempre el mismo retardo.
	skipDelayDuration = 0,
	...props
}: React.ComponentProps<typeof TooltipPrimitive.Provider>) {
	return (
		<TooltipPrimitive.Provider
			data-slot="tooltip-provider"
			delayDuration={delayDuration}
			skipDelayDuration={skipDelayDuration}
			{...props}
		/>
	);
}

function Tooltip({ ...props }: React.ComponentProps<typeof TooltipPrimitive.Root>) {
	return <TooltipPrimitive.Root data-slot="tooltip" {...props} />;
}

function TooltipTrigger({ ...props }: React.ComponentProps<typeof TooltipPrimitive.Trigger>) {
	return <TooltipPrimitive.Trigger data-slot="tooltip-trigger" {...props} />;
}

/** Escapes que ya hemos reenviado nosotros: no se vuelven a reenviar. */
const escapesReenviados = new WeakSet<Event>();

/** Lo que puede haber DEBAJO de un tooltip y querer cerrarse con Escape. */
const CAPAS_CERRABLES = '[role="dialog"], [role="alertdialog"], [role="menu"], [role="listbox"]';

/**
 * 🛑 **Un tooltip no se queda con el Escape** (1/10/2026).
 *
 * Radix registra el tooltip como una capa más y solo la de más arriba atiende
 * Escape: con un tooltip abierto sobre un modal, la primera pulsación cerraba
 * el tooltip y el modal seguía ahí — había que pulsar dos veces. Un tooltip no
 * es algo que se «cierre»: se va solo al salir el ratón.
 *
 * No se puede sacar la capa del tooltip de la pila de Radix, así que se hace
 * lo equivalente: el tooltip se cierra como siempre y, **en cuanto ha
 * desaparecido de verdad** (sigue en la pila mientras dura su animación de
 * salida), se vuelve a lanzar el mismo Escape para la capa de debajo. Solo si
 * hay alguna debajo: con un tooltip sobre la pantalla normal, Escape se queda
 * donde estaba. Y el reenviado va marcado para no reenviarse otra vez.
 */
function reenviarEscape(e: KeyboardEvent) {
	if (escapesReenviados.has(e)) return;
	if (!document.querySelector(CAPAS_CERRABLES)) return;
	let intentos = 0;
	const lanzar = () => {
		if (document.querySelector('[data-slot="tooltip-content"]') && intentos++ < 60) {
			requestAnimationFrame(lanzar);
			return;
		}
		const nuevo = new KeyboardEvent("keydown", {
			key: "Escape",
			code: "Escape",
			bubbles: true,
			cancelable: true,
		});
		escapesReenviados.add(nuevo);
		document.dispatchEvent(nuevo);
	};
	requestAnimationFrame(lanzar);
}

function TooltipContent({
	className,
	sideOffset = 0,
	children,
	onEscapeKeyDown,
	...props
}: React.ComponentProps<typeof TooltipPrimitive.Content>) {
	return (
		<TooltipPrimitive.Portal>
			<TooltipPrimitive.Content
				data-slot="tooltip-content"
				sideOffset={sideOffset}
				onEscapeKeyDown={(e) => {
					onEscapeKeyDown?.(e);
					if (!e.defaultPrevented) reenviarEscape(e);
				}}
				className={cn(
					// Tokens del tema: el par bg-popover / text-popover-foreground SIEMPRE
					// contrasta (claro en light con texto oscuro, oscuro en dark con texto
					// claro). No puede invertirse y queda en sintonía con la app.
					"z-50 w-fit origin-(--radix-tooltip-content-transform-origin) animate-in rounded-md",
					"border border-border bg-popover text-popover-foreground",
					"px-3 py-1.5 text-xs text-balance shadow-md",
					"fade-in-0 zoom-in-95",
					"data-[side=bottom]:slide-in-from-top-2 data-[side=left]:slide-in-from-right-2",
					"data-[side=right]:slide-in-from-left-2 data-[side=top]:slide-in-from-bottom-2",
					"data-[state=closed]:animate-out data-[state=closed]:fill-mode-forwards data-[state=closed]:fade-out-0 data-[state=closed]:zoom-out-95",
					className,
				)}
				{...props}
			>
				{children}
				<TooltipPrimitive.Arrow className="z-50 size-2.5 translate-y-[calc(-50%_-_2px)] rotate-45 rounded-[2px] border-r border-b border-border bg-popover fill-popover" />
			</TooltipPrimitive.Content>
		</TooltipPrimitive.Portal>
	);
}

export { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger };
