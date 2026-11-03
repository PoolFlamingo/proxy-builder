import type { ComponentProps, ReactNode } from "react";
import { AyudaTactil } from "@/components/ayuda-tactil";
import { WithTooltip } from "@/components/ui/with-tooltip";
import { cn } from "@/lib/utils";

type PropsTooltip = Omit<ComponentProps<typeof WithTooltip>, "className">;

/**
 * Un `WithTooltip` cuyo texto es información de verdad (una explicación, un
 * aviso): en escritorio es el tooltip de siempre (la envoltura es
 * `display: contents`, no cambia nada de la maqueta) y en táctil, donde el
 * `Tooltip` de Radix no sale con un toque, pone al lado el «ⓘ» de
 * `AyudaTactil` con el mismo texto.
 *
 * Para los botones que solo llevan un icono NO: su nombre va en `aria-label` y
 * `etiquetas-tactiles.tsx` lo enseña al mantenerlos pulsados.
 */
export function ConAyuda({
	content,
	children,
	className,
	ayuda,
	claseAyuda,
	...tooltip
}: PropsTooltip & {
	/** Clases de la envoltura en táctil (en escritorio es `contents`). */
	className?: string;
	/** El texto del «ⓘ», si no es el mismo del tooltip. */
	ayuda?: ReactNode;
	/** Clases del «ⓘ» (p. ej. más grande junto a un botón). */
	claseAyuda?: string;
}) {
	const texto = ayuda ?? content;
	return (
		<span
			className={cn(
				"contents [[data-tactil]_&]:inline-flex [[data-tactil]_&]:items-center [[data-tactil]_&]:gap-1",
				className,
			)}
		>
			<WithTooltip content={content} {...tooltip}>
				{children}
			</WithTooltip>
			{texto ? <AyudaTactil className={claseAyuda}>{texto}</AyudaTactil> : null}
		</span>
	);
}
