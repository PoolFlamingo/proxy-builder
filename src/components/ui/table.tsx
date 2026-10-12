import * as React from "react";
import { ScrollArea } from "@/components/ui/scroll-area";
import { cn } from "@/lib/utils";

interface TableProps extends React.HTMLAttributes<HTMLTableElement> {
	/**
	 * Ancho mínimo antes de que la tabla empiece a recortar columnas.
	 * Por debajo, `ScrollArea` saca su barra horizontal temática en vez de
	 * aplastar las celdas hasta hacerlas ilegibles (responsive de verdad:
	 * la tabla decide su mínimo, la ventana decide si hace falta scroll).
	 * Cada pantalla pasa el suyo según cuántas columnas tenga.
	 */
	minWidth?: number | string;
	/**
	 * Alto máximo antes de que la tabla empiece a hacer scroll vertical
	 * DENTRO de sí misma.
	 *
	 * 🛑 Existe para que nadie envuelva un `Table` en otro `ScrollArea`. La
	 * tabla ya trae el suyo, y anidarlos rompe el recorte.
	 *
	 * 🛑 Y va al **viewport**, no a la raíz. Un `max-height` en la raíz de un
	 * `ScrollArea` no limita nada — la raíz no recorta y el viewport es
	 * `h-full`, que contra un padre de alto automático vale «auto». La tabla
	 * de Rendimiento se salió del card por abajo tres veces (13-14/08/2026)
	 * por exactamente esto: parecía puesto y no hacía nada.
	 */
	maxHeight?: number | string;
	/**
	 * Ref al nodo de scroll REAL de esta tabla (el viewport de Radix, no un
	 * elemento inventado) — solo hace falta cuando la pantalla virtualiza sus
	 * filas con `useTablaVirtualizada` (`src/lib/tabla-virtual.ts`), que
	 * necesita ese nodo para saber qué hay visible. `Table` no crea el
	 * virtualizador (cada pantalla tiene columnas distintas, no hay fila
	 * genérica): solo presta el elemento. Sin esta prop, `Table` funciona
	 * exactamente igual que siempre. Ver plan
	 * `20260819 - virtualizacion-filas-tablas`.
	 */
	scrollElementRef?: React.RefObject<HTMLDivElement | null>;
}

const Table = React.forwardRef<HTMLTableElement, TableProps>(
	({ className, style, minWidth = 640, maxHeight, scrollElementRef, children, ...props }, ref) => {
		const asignarViewport = React.useCallback(
			(el: HTMLDivElement | null) => {
				if (scrollElementRef) scrollElementRef.current = el;
			},
			[scrollElementRef],
		);
		return (
			// Desbordamiento horizontal con la barra temática (fina y del color
			// del tema), nunca la nativa del sistema. Y vertical si se le pone
			// `maxHeight`.
			//
			// `min-w-0` + `max-w-full`: sin ellos, dentro de un flex/grid el
			// propio ScrollArea se ensancha al contenido en vez de recortarlo, y
			// la tabla asoma por el costado del card. `overflow-hidden` es el
			// cinturón: aunque alguien vuelva a romper el alto, nada se pinta
			// fuera del recuadro.
			<ScrollArea
				className={cn(
					// `rounded-lg` (= --radius-lg = --radius, 14px en toda la app) +
					// el `overflow-hidden` de siempre: sin el radio aquí, el fondo
					// nuevo de la cabecera (ver TableHeader) pintaba una esquina
					// cuadrada por encima de la esquina redondeada del Card que la
					// envuelve — casi todas las tablas viven dentro de un
					// `<CardContent className="p-0">`, a ras del borde de la tarjeta.
					"relative w-full max-w-full min-w-0 overflow-hidden rounded-lg",
					// Un tope en porcentaje solo significa algo si hay contra qué
					// medirlo: con la raíz de alto automático, `max-height: 100%` no
					// vale nada. Si se pide en %, la tabla ocupa el hueco que le dé
					// su contenedor (que es quien tiene la altura), y ahí sí recorta.
					typeof maxHeight === "string" && maxHeight.endsWith("%") && "h-full",
				)}
				viewportStyle={
					maxHeight === undefined
						? undefined
						: { maxHeight: typeof maxHeight === "number" ? `${maxHeight}px` : maxHeight }
				}
				onViewportChange={scrollElementRef ? asignarViewport : undefined}
			>
				<table
					ref={ref}
					data-slot="table"
					// table-layout: fixed — sin esto, una celda con `truncate` no
					// frena: el navegador igual ensancha la columna a su contenido y
					// aplasta a las demás. Con columnas de ancho fijo (o repartido a
					// partes iguales las que no lo tengan), el elipsis por fin
					// recorta de verdad.
					className={cn(
						"w-full table-fixed caption-bottom text-sm",
						// Con scroll vertical, la cabecera se queda arriba: una tabla
						// de catorce modelos y diez columnas sin cabecera visible no
						// se lee. Va con fondo propio y un filete inferior dibujado
						// por `shadow`, porque el `border-b` de una celda `sticky` no
						// viaja con ella.
						maxHeight !== undefined &&
							"[&_thead_th]:sticky [&_thead_th]:top-0 [&_thead_th]:z-10 [&_thead_th]:bg-muted [&_thead_th]:shadow-[inset_0_-1px_0_0_var(--border)]",
						className,
					)}
					style={{ minWidth: typeof minWidth === "number" ? `${minWidth}px` : minWidth, ...style }}
					{...props}
				>
					{children}
				</table>
			</ScrollArea>
		);
	},
);
Table.displayName = "Table";

const TableHeader = React.forwardRef<
	HTMLTableSectionElement,
	React.HTMLAttributes<HTMLTableSectionElement>
>(({ className, ...props }, ref) => (
	// `bg-muted`: un fondo un pelín distinto del `bg-card` de la tarjeta que
	// envuelve la tabla, para que la cabecera se distinga del cuerpo de un
	// vistazo (en el tema claro de fábrica, --muted es apenas más oscuro que
	// --card; en cada paleta tweakcn cambia de matiz, pero siempre se separa
	// del fondo de la tarjeta). Las celdas (`TableHead`) van transparentes:
	// el color sale de aquí, del `<thead>`, salvo en la cabecera fija
	// (`maxHeight`), que lo fuerza celda a celda porque tiene que quedar
	// opaca sobre el contenido que se desplaza debajo (ver `Table` arriba).
	<thead
		ref={ref}
		className={cn("bg-muted [&_tr]:border-b [&_tr]:border-border/40", className)}
		{...props}
	/>
));
TableHeader.displayName = "TableHeader";

const TableBody = React.forwardRef<
	HTMLTableSectionElement,
	React.HTMLAttributes<HTMLTableSectionElement>
>(({ className, ...props }, ref) => (
	<tbody
		ref={ref}
		// El cebrado por posición NO toca las filas virtualizadas (`data-index`)
		// ni las de relleno: en una tabla virtualizada la posición en el DOM se
		// desplaza una fila cada vez que la ventana avanza, y el cebrado CSS se
		// sumaba al de `impar` —cada ~50px de scroll la tabla entera pasaba de
		// cebrada a toda sombreada y vuelta—. Esas filas cebran con `impar`.
		className={cn(
			"[&_tr:last-child]:border-0 [&>tr:nth-child(even):not([data-index]):not([data-relleno])]:bg-muted/25",
			className,
		)}
		{...props}
	/>
));
TableBody.displayName = "TableBody";

interface TableRowProps extends React.HTMLAttributes<HTMLTableRowElement> {
	/**
	 * Cebrado EXPLÍCITO, para filas virtualizadas.
	 *
	 * 🛑 El `[&>tr:nth-child(even)]` de `TableBody` cebra por POSICIÓN en el
	 * DOM. En una tabla virtualizada solo hay montado un puñado de `<tr>` a
	 * la vez, así que su posición en el DOM ya no coincide con su índice
	 * LÓGICO en la lista completa — al hacer scroll el patrón "salta" en vez
	 * de seguir a la fila. `impar` pinta el mismo `bg-muted/25` de siempre
	 * pero calculado por la propia pantalla desde `virtualRow.index % 2`, así
	 * que no depende del DOM. Las tablas NO virtualizadas no la pasan y
	 * siguen cebrando por CSS, sin cambios. Ver plan
	 * `20260819 - virtualizacion-filas-tablas`.
	 */
	impar?: boolean;
}

/**
 * Fila vacía que ocupa el sitio de las filas NO montadas de una tabla
 * virtualizada: una antes de las filas visibles y otra después. Ver
 * `src/lib/tabla-virtual.ts` para el porqué de rellenar en vez de desplazar.
 *
 * 🛑 `columnas` tiene que ser el número REAL de columnas de la tabla. Una
 * celda con `colSpan` mayor que la tabla no se recorta: crea columnas nuevas,
 * y con `table-fixed` eso le quita ancho a las de verdad.
 *
 * Con alto 0 no se pinta: al final de la lista, una fila vacía de 0px sería
 * la última del cuerpo y le quitaría a la última fila de verdad el
 * `[&_tr:last-child]:border-0` de `TableBody`, dejando una raya abajo.
 */
function FilaRelleno({ altoPx, columnas }: { altoPx: number; columnas: number }) {
	if (altoPx <= 0) return null;
	return (
		<tr aria-hidden data-relleno="">
			<td colSpan={columnas} style={{ height: altoPx, padding: 0, border: 0 }} />
		</tr>
	);
}

const TableRow = React.forwardRef<HTMLTableRowElement, TableRowProps>(
	({ className, impar, ...props }, ref) => (
		<tr
			ref={ref}
			className={cn(
				// Rejilla suave: separación horizontal (border-b) y vertical (divide-x)
				// a media opacidad, apenas visible — solo para que el ojo siga la fila
				// y la columna sin que la tabla "grite". El cebreado va en TableBody
				// (necesita saber qué fila es par); aquí solo el hover, más marcado.
				"divide-x divide-border/40 border-b border-border/40 transition-colors hover:bg-muted/60 data-[state=selected]:bg-muted",
				impar && "bg-muted/25",
				className,
			)}
			{...props}
		/>
	),
);
TableRow.displayName = "TableRow";

const TableHead = React.forwardRef<
	HTMLTableCellElement,
	React.ThHTMLAttributes<HTMLTableCellElement>
>(({ className, ...props }, ref) => (
	<th
		ref={ref}
		className={cn(
			"h-[var(--pb-row-h)] px-2 text-left align-middle font-medium text-muted-foreground",
			className,
		)}
		{...props}
	/>
));
TableHead.displayName = "TableHead";

const TableCell = React.forwardRef<
	HTMLTableCellElement,
	React.TdHTMLAttributes<HTMLTableCellElement>
>(({ className, ...props }, ref) => (
	<td ref={ref} className={cn("p-2 align-middle", className)} {...props} />
));
TableCell.displayName = "TableCell";

/**
 * Ancho mínimo de la columna de botones al final de una fila.
 *
 * 🛑 **Existe porque el ojo no hace esta cuenta y la tabla no avisa.** Con
 * `table-fixed`, una columna demasiado estrecha no se ensancha: los botones
 * se salen y se montan sobre el filete de la rejilla. No hay error, no hay
 * warning — solo se ve feo, y solo si te fijas. Pasó en Expedientes con tres
 * botones en una columna de 96px (11/08/2026) y estaba pasando igual, sin que
 * nadie lo hubiera notado, en la tabla de Partes.
 *
 * La cuenta: `n · lado + (n−1) · hueco + 16`, donde el 16 es el `p-2` de
 * `TableCell` (8px a cada lado). Se redondea a la clase de Tailwind
 * inmediatamente superior.
 *
 * | botones | `size-6` + `gap-0.5` | `size-7` + `gap-1` |
 * |--------:|:---------------------|:-------------------|
 * |       1 | 40 → `w-12`          | 44 → `w-12`        |
 * |       2 | 66 → `w-20`          | 76 → `w-20`        |
 * |       3 | 92 → `w-24`          | 108 → `w-28`       |
 *
 * Al añadir un botón a una fila hay que subir la columna al tramo siguiente.
 * Si un día esto se repite lo bastante, se convierte en un componente; de
 * momento la tabla de arriba es más útil que una abstracción.
 */
export const ANCHO_ACCIONES = {
	/**
	 * 🛑 **El de siempre desde el 5/09/2026**: un solo botón de tres puntos que
	 * abre el menú de la fila (`MenuFila`). Las hileras de iconos se retiraron —
	 * se comían el ancho de las columnas con datos y solo aparecían al pasar el
	 * ratón—, así que los tres tramos de abajo son para lo que quede sin migrar.
	 */
	menu: "w-12",
	/** 3 botones `size-7` con `gap-1` (Expedientes: editar, archivar, borrar). */
	tresGrandes: "w-28",
	/** 3 botones `size-6` con `gap-0.5` (Partes: corregir, confirmar, borrar). */
	tresPequenos: "w-24",
	/** 2 botones, de cualquiera de los dos tamaños. */
	dos: "w-20",
	/** 1 botón, de cualquiera de los dos tamaños. */
	uno: "w-12",
} as const;

export { FilaRelleno, Table, TableBody, TableCell, TableHead, TableHeader, TableRow };
