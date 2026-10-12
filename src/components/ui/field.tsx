import type * as React from "react";

import { Label } from "@/components/ui/label";
import { cn } from "@/lib/utils";

/**
 * Los primitivos `Field` de shadcn, escritos a mano.
 *
 * El entorno no instala paquetes, así que en vez de `npx shadcn add field` se
 * escriben aquí con la misma API pública (`FieldSet`, `FieldLegend`,
 * `FieldGroup`, `Field`, `FieldLabel`, `FieldDescription`, `FieldError`) para
 * que el día que se pueda instalar el oficial no haya que tocar quien los usa.
 *
 * Se apartan del original en una cosa: la **densidad**. El sistema visual de
 * JUDLEX es de 10 px (`gap-2.5`) y texto base `text-xs`; el de shadcn es de
 * 16 px y `text-sm`, y mezclarlos deja un formulario que respira el doble que
 * la tarjeta que lo contiene.
 *
 * 🛑 Un `Field` **no dibuja el control**: envuelve el que le pongas dentro. Eso
 * es lo que permite meter ahí un `<Input>` propio o —y este es el caso que hizo
 * falta— el iframe de Stripe, que no es nuestro y no admite que le pongamos
 * nada por dentro.
 */

/** Un bloque de campos con su título. `<fieldset>` de verdad: es lo que
 *  agrupa los controles para quien navega con teclado o con lector. */
function FieldSet({ className, ...props }: React.ComponentProps<"fieldset">) {
	return (
		<fieldset
			data-slot="field-set"
			className={cn(
				"flex min-w-0 flex-col gap-2.5",
				// `disabled` en el fieldset apaga todo lo de dentro sin que cada
				// control tenga que enterarse.
				"disabled:opacity-55",
				className,
			)}
			{...props}
		/>
	);
}

/** El título del bloque. `<legend>` y no un `<span>`: el lector lo anuncia
 *  antes de cada campo del grupo, que es justo para lo que existe. */
function FieldLegend({ className, ...props }: React.ComponentProps<"legend">) {
	return (
		<legend data-slot="field-legend" className={cn("text-xs font-medium", className)} {...props} />
	);
}

/** Varios campos seguidos, con la separación de la casa. */
function FieldGroup({ className, ...props }: React.ComponentProps<"div">) {
	return (
		<div
			data-slot="field-group"
			className={cn("flex min-w-0 flex-col gap-2.5", className)}
			{...props}
		/>
	);
}

/**
 * Un campo: etiqueta, control, ayuda y error.
 *
 * `orientacion="horizontal"` pone la etiqueta al lado (casillas e
 * interruptores); en vertical, encima, que es lo normal en un formulario de
 * datos.
 *
 * 🛑 `min-w-0` no es decoración: sin él, un control con contenido largo —el
 * iframe de Stripe, un nombre de fichero— empuja la columna y desborda la
 * tarjeta a lo ancho en vez de recortarse.
 */
function Field({
	className,
	orientacion = "vertical",
	...props
}: React.ComponentProps<"div"> & {
	orientacion?: "vertical" | "horizontal";
}) {
	return (
		<div
			data-slot="field"
			data-orientacion={orientacion}
			role="group"
			className={cn(
				"flex min-w-0 gap-1",
				orientacion === "horizontal" ? "flex-row items-center gap-2.5" : "flex-col",
				// Un campo con error se marca en ÁMBAR, no en rojo: en JUDLEX el rojo
				// es «se acabó» y esto es «revísalo». Ver `.badge-conflicto`.
				"has-[[data-slot=field-error]]:[&_[data-slot=input]]:border-amber-500/60",
				className,
			)}
			{...props}
		/>
	);
}

/** La etiqueta del campo. Reusa el `Label` del sistema para heredar el
 *  comportamiento de `peer-disabled` y el `htmlFor`. */
function FieldLabel({ className, ...props }: React.ComponentProps<typeof Label>) {
	return <Label data-slot="field-label" className={cn("text-[11px]", className)} {...props} />;
}

/** La ayuda de debajo: qué se espera, o qué va a pasar al rellenarlo. */
function FieldDescription({ className, ...props }: React.ComponentProps<"p">) {
	return (
		<p
			data-slot="field-description"
			className={cn("text-[10px] leading-relaxed text-muted-foreground", className)}
			{...props}
		/>
	);
}

/**
 * Lo que no cuadra, en ámbar.
 *
 * 🛑 Se **oculta si no hay nada que decir** (`children` vacío) en vez de
 * reservar el hueco: una línea en blanco permanente debajo de cada campo mueve
 * el formulario entero cada vez que aparece un mensaje.
 *
 * `role="alert"` para que el lector lo anuncie al aparecer; sin él, quien no ve
 * la pantalla pulsa Guardar y no se entera de por qué no pasa nada.
 */
function FieldError({ className, children, ...props }: React.ComponentProps<"p">) {
	if (!children) return null;
	return (
		<p
			data-slot="field-error"
			role="alert"
			className={cn("text-[10px] leading-relaxed text-amber-600 dark:text-amber-400", className)}
			{...props}
		>
			{children}
		</p>
	);
}

export { Field, FieldDescription, FieldError, FieldGroup, FieldLabel, FieldLegend, FieldSet };
