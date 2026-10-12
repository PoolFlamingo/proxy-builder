import { ScrollArea as ScrollAreaPrimitive } from "radix-ui";
import * as React from "react";
import { cn } from "@/lib/utils";

/**
 * Los hijos del `ScrollArea`, entregados por contexto al interior del
 * viewport. Ver `ScrollArea`, más abajo, para el porqué.
 */
const HijosDelViewport = React.createContext<React.ReactNode>(null);

function ContenidoDelViewport() {
	return <>{React.useContext(HijosDelViewport)}</>;
}

/** Dos `style` con las mismas propiedades y los mismos valores. */
function mismoEstilo(a: React.CSSProperties | undefined, b: React.CSSProperties | undefined) {
	if (a === b) return true;
	if (!a || !b) return false;
	const claves = Object.keys(a) as (keyof React.CSSProperties)[];
	if (claves.length !== Object.keys(b).length) return false;
	return claves.every((k) => a[k] === b[k]);
}

function ScrollArea({
	className,
	children,
	type = "scroll",
	viewportClassName,
	viewportStyle,
	onViewportChange,
	...props
}: React.ComponentProps<typeof ScrollAreaPrimitive.Root> & {
	/**
	 * Clases para el VIEWPORT, que es el elemento que recorta y desplaza.
	 *
	 * 🛑 Un `max-height` puesto en la raíz NO limita nada: la raíz no recorta y
	 * el viewport es `h-full`, que contra un padre de alto automático vale
	 * «auto». El contenido crecía y se pintaba fuera de la tarjeta, sin barra.
	 * Pasó tres veces con la tabla de Rendimiento (13-14/08/2026). Todo tope de
	 * alto va aquí.
	 */
	viewportClassName?: string;
	/** Igual que `viewportClassName`, para valores que no son clase (24rem, %…). */
	viewportStyle?: React.CSSProperties;
	/**
	 * Entrega el nodo DOM real del viewport (el que hace scroll) a quien lo
	 * pida desde fuera — hoy lo usa `Table` para las tablas virtualizadas
	 * (`useTablaVirtualizada` necesita el elemento de scroll real, no uno
	 * inventado). Ver plan `20260819 - virtualizacion-filas-tablas`.
	 */
	onViewportChange?: (el: HTMLDivElement | null) => void;
}) {
	const setViewportRef = React.useCallback(
		(el: HTMLDivElement | null) => {
			onViewportChange?.(el);
		},
		[onViewportChange],
	);

	// 🛑 **El viewport de Radix NO se vuelve a pintar cuando cambian los hijos.**
	//
	// `ScrollAreaPrimitive.Viewport` pinta a su lado un `<style>` con
	// `dangerouslySetInnerHTML={{ __html: "…" }}`, y ese objeto es nuevo en cada
	// render. React 19 compara el objeto, no el texto, así que en CADA render
	// del viewport vuelve a escribir el `innerHTML` de esa hoja de estilos. En
	// Chrome da igual; en WebKitGTK (la ventana de JUDLEX en Linux) cambiar una
	// hoja de estilos obliga a recalcular el estilo del documento ENTERO:
	// ~50 ms cada vez. Y el viewport se repintaba siempre que se repintaba lo
	// que lleva dentro —en una tabla virtualizada, cada fila que entra al hacer
	// scroll—. Eso eran los tirones con la rueda en Clasificados y el del
	// primer golpe tras una pausa en el resto de la app (Claudio, 26/09/2026;
	// medido en WebKitGTK 2.52 con un `MutationObserver` sobre ese `<style>`).
	//
	// Por eso el viewport se crea una sola vez (`useMemo`) con un hijo fijo,
	// `ContenidoDelViewport`, que lee los hijos de verdad por contexto: cuando
	// cambian, se repinta ese hijo y no el viewport ni su `<style>`. Para que
	// el `useMemo` aguante, el `style` se compara por valor —`Table` pasa un
	// objeto nuevo en cada render— y la clase es una cadena.
	const estiloRef = React.useRef(viewportStyle);
	if (!mismoEstilo(estiloRef.current, viewportStyle)) estiloRef.current = viewportStyle;
	const estiloViewport = estiloRef.current;
	// 🛑 `relative` en el viewport, y no es adorno (12/10/2026). Lo que va
	// posicionado en absoluto dentro —los `<input>` ocultos que Radix pone junto
	// a cada `Checkbox` de un formulario, por ejemplo— tomaba como referencia la
	// RAÍZ, que no recorta: no se desplazaba con la lista y alargaba el scroll
	// de lo que hubiera fuera. En «Añadir un proveedor», con los 80 modelos de
	// NVIDIA, el cuerpo del diálogo medía 1.188 px de contenido y 2.500 de
	// scroll: un hueco vacío enorme debajo. Con el viewport como referencia, lo
	// absoluto vive dentro de lo que se desplaza.
	const claseViewport = cn(
		"relative size-full rounded-[inherit] transition-[color,box-shadow] outline-none focus-visible:ring-[3px] focus-visible:ring-ring/50 focus-visible:outline-1",
		viewportClassName,
	);
	const viewport = React.useMemo(
		() => (
			<ScrollAreaPrimitive.Viewport
				ref={setViewportRef}
				data-slot="scroll-area-viewport"
				style={estiloViewport}
				className={claseViewport}
			>
				<ContenidoDelViewport />
			</ScrollAreaPrimitive.Viewport>
		),
		[setViewportRef, estiloViewport, claseViewport],
	);

	return (
		<ScrollAreaPrimitive.Root
			data-slot="scroll-area"
			type={type}
			className={cn("relative", className)}
			{...props}
		>
			<HijosDelViewport.Provider value={children}>{viewport}</HijosDelViewport.Provider>
			{/* Las dos barras van AQUÍ, hermanas del viewport. Radix las esconde
          sola cuando no hay nada que desplazar en ese eje. Antes la
          horizontal la pasaba `Table` como hijo, y un hijo de `ScrollArea`
          acaba DENTRO del viewport — es decir, dentro de lo que se desplaza:
          una barra de scroll viajando con el contenido que debía gobernar. */}
			<ScrollBar />
			<ScrollBar orientation="horizontal" />
			<ScrollAreaPrimitive.Corner />
		</ScrollAreaPrimitive.Root>
	);
}

function ScrollBar({
	className,
	orientation = "vertical",
	...props
}: React.ComponentProps<typeof ScrollAreaPrimitive.ScrollAreaScrollbar>) {
	return (
		// 🛑 `forceMount`: la barra se queda montada y se esconde con CSS.
		// Con `type="scroll"` Radix la DESMONTABA a los 600 ms de parar y la
		// volvía a montar con el primer movimiento. Montarla o desmontarla
		// cambia un estado de la raíz de Radix (qué barra hay), eso cambia el
		// contexto, el viewport se vuelve a pintar y reescribe su `<style>` (ver
		// `ScrollArea`, arriba): en WebKitGTK, un recálculo de estilo del
		// documento entero en el primer golpe de rueda tras cada pausa. Medido
		// en WebKitGTK 2.52: ~120 ms de fotograma en ese golpe sin esto, ~35 ms
		// con esto (Claudio, 26/09/2026). Oculta no recibe el ratón, igual que
		// cuando no estaba montada.
		<ScrollAreaPrimitive.ScrollAreaScrollbar
			data-slot="scroll-area-scrollbar"
			orientation={orientation}
			forceMount
			className={cn(
				"flex touch-none p-px transition-[color,background-color,border-color,opacity] duration-150 select-none data-[state=hidden]:pointer-events-none data-[state=hidden]:opacity-0",
				orientation === "vertical" && "h-full w-1.5 border-l border-l-transparent",
				orientation === "horizontal" && "h-1.5 flex-col border-t border-t-transparent",
				className,
			)}
			{...props}
		>
			<ScrollAreaPrimitive.ScrollAreaThumb
				data-slot="scroll-area-thumb"
				className="relative flex-1 rounded-full"
			/>
		</ScrollAreaPrimitive.ScrollAreaScrollbar>
	);
}

export { ScrollArea, ScrollBar };
