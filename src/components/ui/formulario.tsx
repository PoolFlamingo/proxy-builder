import type * as React from "react";
import { useCallback, useContext, useEffect, useState } from "react";
import { SalidaDelDialogo } from "@/components/ui/salida-dialogo";

/**
 * **Un `<form>` para `action={…}` que NO deja que React lo resetee después.**
 *
 * React 19 hace `form.reset()` al terminar una acción de formulario, y lo hace
 * tras CADA acción, también tras una que capturó su propio error y lo enseñó
 * en un aviso. Con campos controlados no haría daño, pero el `Checkbox`, el
 * `Switch` y el `RadioGroup` de Radix escuchan el `reset` del formulario y
 * devuelven su valor de MONTAJE por `onCheckedChange`. Medido en la web el
 * 09/10/2026: un error del servidor volvió a encender en silencio un
 * interruptor que se había apagado, y el reintento mandó lo que nadie eligió.
 *
 * Por eso se cancela el reset, y el estado de React queda como única fuente.
 * Un formulario que de verdad quiera vaciarse vacía su ESTADO, nunca el DOM.
 *
 * 🛑 **Con un escuchador NATIVO en el documento, en captura; no con
 * `onResetCapture`.** React llama a `form.reset()` DURANTE el commit, con sus
 * propios eventos desactivados (`_enabled = false` en `react-dom`), así que un
 * manejador de React no se entera (medido en JUDLEX con react-dom 19.2.7). El
 * del documento en captura corre antes que cualquiera puesto en el propio
 * formulario, como el de Radix, y no depende del orden de captura y burbuja en
 * el destino, que no todos los motores resuelven igual.
 *
 * 🛑 Todo `<form action>` es un `Formulario` (o un `DialogFormulario`, que lo
 * usa por dentro), nunca un `<form>` a secas.
 */
export function Formulario({
	ref,
	action,
	...props
}: Omit<React.ComponentProps<"form">, "action"> & {
	/**
	 * La acción. Su segundo argumento es la señal de parar: se activa si,
	 * dentro de un diálogo, la persona sale mientras trabaja y dice que sí
	 * (`DialogContent`). Una acción de varios pasos la mira entre paso y paso
	 * y, si está activa, no sigue ni guarda lo que quedaba.
	 */
	action?: (datos: FormData, senal: AbortSignal) => void | Promise<void>;
}) {
	// 🛑 Dentro de un diálogo, la acción avisa de que trabaja: así salir
	// (Esc, el fondo, la X, «Cancelar») pregunta antes de cortarla.
	const salida = useContext(SalidaDelDialogo);
	const accion = action
		? async (datos: FormData) => {
				const control = new AbortController();
				const terminar = salida?.empezarProceso(() => control.abort());
				try {
					await action(datos, control.signal);
				} finally {
					terminar?.();
				}
			}
		: undefined;
	// En un estado y no en una ref: el formulario puede montarse dentro de un
	// portal un instante después que este componente (lo mismo que hace
	// `DialogContent` con `useFocoEnMarcos`).
	const [form, setForm] = useState<HTMLFormElement | null>(null);
	useEffect(() => {
		if (!form) return;
		const cancelar = (e: Event) => {
			if (e.target !== form) return;
			e.preventDefault();
			e.stopImmediatePropagation();
		};
		const doc = form.ownerDocument;
		doc.addEventListener("reset", cancelar, true);
		return () => doc.removeEventListener("reset", cancelar, true);
	}, [form]);
	const unir = useCallback(
		(n: HTMLFormElement | null) => {
			setForm(n);
			if (typeof ref === "function") ref(n);
			else if (ref) ref.current = n;
		},
		[ref],
	);
	// sin-formulario: este ES el `Formulario`; el `<form>` a secas solo vive aquí.
	return <form ref={unir} action={accion} {...props} />;
}
