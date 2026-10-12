"use client";

import { XIcon } from "lucide-react";
import { Dialog as DialogPrimitive } from "radix-ui";
import * as React from "react";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import { Formulario } from "@/components/ui/formulario";
import { type Salida, SalidaDelDialogo } from "@/components/ui/salida-dialogo";
import { ScrollArea } from "@/components/ui/scroll-area";
import { useFocoEnMarcos } from "@/hooks/use-foco-en-marcos";
import { cn } from "@/lib/utils";

function Dialog({ ...props }: React.ComponentProps<typeof DialogPrimitive.Root>) {
	return <DialogPrimitive.Root data-slot="dialog" {...props} />;
}

function DialogTrigger({ ...props }: React.ComponentProps<typeof DialogPrimitive.Trigger>) {
	return <DialogPrimitive.Trigger data-slot="dialog-trigger" {...props} />;
}

function DialogPortal({ ...props }: React.ComponentProps<typeof DialogPrimitive.Portal>) {
	return <DialogPrimitive.Portal data-slot="dialog-portal" {...props} />;
}

/**
 * Un botón que cierra el diálogo. Pasa por la misma pregunta que la X y el
 * fondo: si hay algo en marcha o algo escrito, se pregunta antes (ver
 * `DialogContent`).
 */
function DialogClose({ onClick, ...props }: React.ComponentProps<typeof DialogPrimitive.Close>) {
	const salida = React.useContext(SalidaDelDialogo);
	return (
		<DialogPrimitive.Close
			data-slot="dialog-close"
			onClick={(e) => {
				onClick?.(e);
				if (!e.defaultPrevented && salida?.hayQuePreguntar()) {
					e.preventDefault();
					salida.pedirSalida(salida.cerrar);
				}
			}}
			{...props}
		/>
	);
}

// ─── Salir de un diálogo ─────────────────────────────────────────────────────
//
// 🛑 **Un diálogo con algo en marcha, o con algo escrito, no se cierra de
// golpe** (Pablo, 12/10/2026: «no puede cerrarse con un backdrop así de
// primeras ni con la "x" ni con un cancelar, porque puede interrumpir y fallar
// el proceso. Debe de salir un aviso-diálogo e interrumpirlo adecuadamente si
// el usuario le da a aceptar»; y «si tiene algún tipo de dato relleno, lo
// mismo […] en todos los modales de la app»).
//
// Las salidas son cuatro y pasan todas por aquí: `Esc`, un clic en el fondo, la
// X y los botones de salir (`DialogCancelar`, `DialogClose`). Si la acción del
// formulario trabaja (cualquier `Formulario` de dentro), o hay un trabajo
// declarado (`ocupado`, `useTrabajoEnElDialogo`), se
// pregunta si se interrumpe; si se ha escrito algo, si se sale sin guardar.
// Con un «sí», la acción recibe la señal de parar (`AbortSignal`, su segundo
// argumento) y el diálogo se cierra por donde se iba a cerrar.
//
// Lo que se cierra porque TERMINÓ —la acción llama a `onCerrar` al guardar— no
// pasa por aquí: es el estado del padre, no una salida de la persona.

/**
 * **El botón de salir de un diálogo** («Cancelar», «Cerrar», «Volver»). Es un
 * `Button` que, antes de hacer lo suyo, pregunta si hay algo en marcha o algo
 * escrito. No se apaga mientras la acción trabaja: es justo la forma de
 * interrumpirla.
 */
function DialogCancelar({ onClick, ...props }: React.ComponentProps<typeof Button>) {
	const salida = React.useContext(SalidaDelDialogo);
	return (
		<Button
			type="button"
			variant="outline"
			data-salida-dialogo=""
			onClick={(e) => {
				if (!salida) {
					onClick?.(e);
					return;
				}
				salida.pedirSalida(() => onClick?.(e));
			}}
			{...props}
		/>
	);
}

/** La pregunta antes de salir. */
function ConfirmarSalida({
	abierta,
	enCurso,
	onSeguir,
	onSalir,
}: {
	abierta: boolean;
	enCurso: boolean;
	onSeguir: () => void;
	onSalir: () => void;
}) {
	const { t } = useLang();
	return (
		<DialogPrimitive.Root open={abierta} onOpenChange={(o) => !o && onSeguir()}>
			<DialogPortal>
				<DialogOverlay />
				<DialogPrimitive.Content
					data-slot="dialog-content"
					data-confirmar-salida=""
					className="fixed top-[50%] left-[50%] z-50 flex w-[calc(100%-2rem)] max-w-md translate-x-[-50%] translate-y-[-50%] flex-col gap-4 rounded-lg border bg-background p-6 shadow-lg outline-none data-[state=open]:animate-in data-[state=open]:fade-in-0 data-[state=open]:zoom-in-95"
				>
					<DialogHeader>
						<DialogTitle className="text-sm">
							{enCurso ? t("¿Interrumpir lo que está en marcha?") : t("¿Salir sin guardar?")}
						</DialogTitle>
						<DialogDescription className="text-xs">
							{enCurso
								? t(
										"Si sales ahora, se para. Lo que ya se haya enviado termina por su cuenta, pero no se sigue con lo que faltaba ni se guarda lo que quedó a medias.",
									)
								: t("Lo que has escrito en esta ventana se pierde.")}
						</DialogDescription>
					</DialogHeader>
					<DialogFooter>
						<Button type="button" variant="outline" size="sm" onClick={onSeguir}>
							{enCurso ? t("Seguir esperando") : t("Seguir editando")}
						</Button>
						<Button type="button" variant="destructive" size="sm" onClick={onSalir}>
							{enCurso ? t("Interrumpir y salir") : t("Salir sin guardar")}
						</Button>
					</DialogFooter>
				</DialogPrimitive.Content>
			</DialogPortal>
		</DialogPrimitive.Root>
	);
}

function DialogOverlay({
	className,
	...props
}: React.ComponentProps<typeof DialogPrimitive.Overlay>) {
	return (
		<DialogPrimitive.Overlay
			data-slot="dialog-overlay"
			className={cn(
				// Velo con un desenfoque SUAVE detrás (Claudio, 31/08/2026): lo que
				// queda debajo se reconoce, pero deja de competir con el diálogo.
				//
				// 🛑 Va aquí, en el overlay compartido, y no en un diálogo suelto: que
				// el login difumine y el de renombrar no es la clase de incoherencia
				// que nadie sabe explicar pero todo el mundo nota.
				//
				// 3px, no más. Por encima de ~6px el fondo se vuelve una mancha, y
				// entonces el velo negro sobra; y como `backdrop-filter` lo pinta la
				// GPU en cada fotograma de la ventana, subirlo se paga en una app de
				// escritorio que puede tener una tabla de mil filas detrás. Por eso
				// también baja el negro del 50 % al 40 %: con desenfoque, el mismo
				// velo de antes oscurecía de más.
				"fixed inset-0 z-50 bg-black/40 backdrop-blur-[3px] data-[state=closed]:animate-out data-[state=closed]:fill-mode-forwards data-[state=closed]:fade-out-0 data-[state=open]:animate-in data-[state=open]:fade-in-0",
				className,
			)}
			{...props}
		/>
	);
}

function DialogContent({
	className,
	children,
	showCloseButton = true,
	ref,
	ocupado = false,
	alInterrumpir,
	cambiosSinGuardar,
	onEscapeKeyDown,
	onPointerDownOutside,
	onFocusOutside,
	...props
}: React.ComponentProps<typeof DialogPrimitive.Content> & {
	showCloseButton?: boolean;
	/** Hay un trabajo en marcha que NO va por la acción del formulario (una
	 *  búsqueda, una espera). Con él, salir pregunta antes. */
	ocupado?: boolean;
	/** Cómo se para ese trabajo si la persona dice que sí. */
	alInterrumpir?: () => void;
	/** Si hay algo sin guardar. Sin decirlo, lo averigua el diálogo: cuenta
	 *  como escrito lo que se teclea en sus campos y las casillas que se
	 *  marcan, salvo dentro de algo con `data-filtro` (un buscador). Un diálogo que se queda abierto tras guardar lo dice con
	 *  `false`. */
	cambiosSinGuardar?: boolean;
}) {
	// 🛑 Un visor dentro (PDF) se queda con el foco y con él, con `Esc`. Ver
	// `useFocoEnMarcos`: vale para TODOS los diálogos, no solo el del visor.
	// El nodo va en un estado, no en una ref: el portal de Radix monta el
	// contenido un instante DESPUÉS que este componente.
	const [nodo, setNodo] = React.useState<HTMLDivElement | null>(null);
	useFocoEnMarcos(nodo);

	// Lo que decide si salir pregunta. En refs, porque lo leen manejadores
	// que Radix guarda.
	const procesos = React.useRef(new Set<() => void>());
	/** Dónde se ha escrito: el formulario del campo (o el campo, si no va en
	 *  uno). Cuenta mientras siga en pantalla: un formulario que se va —el
	 *  alta de una cuenta que ya se guardó y vuelve a la lista— se lleva lo
	 *  suyo. */
	const tocados = React.useRef(new Set<Element>());
	const [pregunta, setPregunta] = React.useState<{ salir: () => void; enCurso: boolean } | null>(
		null,
	);
	const estado = React.useRef({ ocupado, cambiosSinGuardar });
	estado.current = { ocupado, cambiosSinGuardar };
	const sinGuardarAhora = () =>
		estado.current.cambiosSinGuardar ?? [...tocados.current].some((el) => el.isConnected);
	const enCursoAhora = () => estado.current.ocupado || procesos.current.size > 0;
	const cerrarRef = React.useRef<HTMLButtonElement>(null);

	// Lo escrito: lo que se teclea en los campos de ESTE diálogo (no de uno
	// anidado) y las casillas que se pulsan. Solo lo que hace la persona
	// (`isTrusted`): un valor que pone el código no es «algo escrito». Y no
	// cuenta lo que busca o mueve en vez de guardar: lo que va en
	// `data-filtro` (`CampoBuscar` ya lo lleva) y los deslizadores.
	React.useEffect(() => {
		if (!nodo) return;
		const esDeAqui = (t: EventTarget | null) => {
			const el = t instanceof Element ? t : null;
			return (
				!!el &&
				el.closest('[data-slot="dialog-content"]') === nodo &&
				!el.closest('[data-filtro], [role="slider"], input[type="range"], input[type="search"]')
			);
		};
		const apuntar = (el: Element) => tocados.current.add(el.closest("form") ?? el);
		const alEscribir = (e: Event) => {
			if (e.isTrusted && e.target instanceof Element && esDeAqui(e.target)) apuntar(e.target);
		};
		const alPulsar = (e: Event) => {
			if (!e.isTrusted || !(e.target instanceof Element)) return;
			const casilla = e.target.closest('[role="checkbox"], [role="switch"], [role="radio"]');
			if (casilla && esDeAqui(casilla)) apuntar(casilla);
		};
		nodo.addEventListener("input", alEscribir, true);
		nodo.addEventListener("click", alPulsar, true);
		return () => {
			nodo.removeEventListener("input", alEscribir, true);
			nodo.removeEventListener("click", alPulsar, true);
		};
	}, [nodo]);

	const salida = React.useMemo<Salida>(
		() => ({
			empezarProceso: (interrumpir) => {
				procesos.current.add(interrumpir);
				return () => procesos.current.delete(interrumpir);
			},
			hayQuePreguntar: () => enCursoAhora() || sinGuardarAhora(),
			pedirSalida: (salir) => {
				const enCurso = enCursoAhora();
				if (enCurso || sinGuardarAhora()) setPregunta({ salir, enCurso });
				else salir();
			},
			cerrar: () => cerrarRef.current?.click(),
		}),
		[],
	);
	const interrumpirRef = React.useRef(alInterrumpir);
	interrumpirRef.current = alInterrumpir;
	const refDelNodo = React.useCallback(
		(n: HTMLDivElement | null) => {
			setNodo(n);
			if (typeof ref === "function") ref(n);
			else if (ref) (ref as React.RefObject<HTMLDivElement | null>).current = n;
		},
		[ref],
	);
	return (
		<DialogPortal data-slot="dialog-portal">
			<DialogOverlay />
			<DialogPrimitive.Content
				data-slot="dialog-content"
				ref={refDelNodo}
				onEscapeKeyDown={(e) => {
					onEscapeKeyDown?.(e);
					if (!e.defaultPrevented && salida.hayQuePreguntar()) {
						e.preventDefault();
						salida.pedirSalida(salida.cerrar);
					}
				}}
				onPointerDownOutside={(e) => {
					onPointerDownOutside?.(e);
					if (!e.defaultPrevented && salida.hayQuePreguntar()) {
						e.preventDefault();
						salida.pedirSalida(salida.cerrar);
					}
				}}
				onFocusOutside={(e) => {
					onFocusOutside?.(e);
					// Que el foco se vaya (un aviso, otra ventana) no es salir.
					if (!e.defaultPrevented && salida.hayQuePreguntar()) e.preventDefault();
				}}
				className={cn(
					// 🛑 **El ancho por defecto va SIN variante (`max-w-lg`, no `sm:max-w-lg`),
					// y no es cosmética.** `cn()` es `tailwind-merge`, y `tailwind-merge` NO
					// deduplica entre variantes distintas: con un `sm:max-w-lg` aquí, el
					// `max-w-5xl` del que llama se queda en la lista pero PIERDE, porque
					// Tailwind emite las utilidades con variante DESPUÉS de las planas. El
					// resultado es el diálogo clavado en 32 rem con su contenido desbordado
					// por los lados — lo que le pasaba a «Correo por colocar» (15/09/2026).
					// Sin variante, el `max-w-*` de quien llama gana siempre, lo escriba como
					// `max-w-5xl` o como `sm:max-w-5xl`.
					//
					// 🛑 Y el tope de pantalla pasa a `w-` en vez de a otro `max-w-`: son
					// propiedades distintas, así que el ancho real es
					// `min(pantalla − 2rem, max-w)` y el diálogo no se sale de la ventana por
					// ancho que lo pida quien llama.
					// 🛑 **`min-w-0` en los hijos, o el diálogo se rompe con un texto
					// largo** (Claudio, 17/09/2026: «arregla estos modales que
					// siempre se rompen»).
					//
					// Un hijo tiene `min-width: auto`, así que una línea larga que
					// no se puede partir —el asunto de un correo de la Comisión,
					// una ruta, un identificador— ESTIRA la caja y el contenido se
					// sale por los lados. Y no se arregla desde dentro: el
					// `truncate` del hijo no puede hacer nada si su columna ya ha
					// crecido para él. Con `[&>*]:min-w-0` el ancho lo manda el
					// diálogo y el texto se recorta donde tiene que recortarse. Va
					// AQUÍ, en el compartido, porque el fallo se repetía en un
					// diálogo detrás de otro.
					//
					// 🛑 **Y el contenedor es una COLUMNA FLEX, no una rejilla**
					// (19/09/2026). Era `grid grid-cols-[minmax(0,1fr)]`, y con el
					// tope de alto de abajo eso se rompe de una forma muy fea: en
					// una rejilla con el alto agotado, el contenido desborda su
					// fila y la fila siguiente ya está colocada, así que **los
					// botones se pintan encima del formulario**. En columna flex el
					// cuerpo se encoge y lo que no cabe se desplaza.
					//
					// 🛑 **Y el ALTO tiene tope, por el mismo motivo que el ancho**
					// (Claudio, 19/09/2026: «que se vean en ventanas pequeñas»).
					//
					// La ventana de JUDLEX baja hasta 960×640 (`minWidth`/`minHeight`
					// en `tauri.conf.json`), y un diálogo sin tope crece con su
					// contenido: en 640 px de alto, un formulario de cliente se sale
					// por arriba y por abajo. Y no es que se vea mal — es que el
					// diálogo está CENTRADO con `translate-y-[-50%]`, así que lo que
					// sobra queda en coordenadas negativas, no hay barra que llegue
					// hasta ahí y el botón de Guardar deja de existir para el abogado.
					//
					// 85vh y no `calc(100%-2rem)`: es el tope que ya usaban los
					// diálogos grandes —una fórmula, no dos— y deja ver un poco de lo
					// que hay detrás, que es lo que hace entender que es un diálogo.
					//
					// 🛑 El tope solo RECORTA. Para que el contenido siga siendo
					// alcanzable, el diálogo que pueda crecer mete su cuerpo en
					// [`DialogCuerpo`] y deja cabecera y botones fijos. Un diálogo
					// corto —una confirmación de tres líneas— no necesita nada: con
					// el tope ya no puede salirse.
					"fixed top-[50%] left-[50%] z-50 flex flex-col w-[calc(100%-2rem)] max-w-lg max-h-[85vh] overflow-hidden translate-x-[-50%] translate-y-[-50%] gap-4 rounded-lg border bg-background p-6 shadow-lg duration-200 outline-none [&>*]:min-w-0 data-[state=closed]:animate-out data-[state=closed]:fill-mode-forwards data-[state=closed]:fade-out-0 data-[state=closed]:zoom-out-95 data-[state=open]:animate-in data-[state=open]:fade-in-0 data-[state=open]:zoom-in-95",
					className,
				)}
				{...props}
			>
				<SalidaDelDialogo.Provider value={salida}>{children}</SalidaDelDialogo.Provider>
				{/* La salida «de verdad», sin pregunta: la pulsa el código cuando la
				    persona ya ha dicho que sí. */}
				<DialogPrimitive.Close ref={cerrarRef} hidden tabIndex={-1} aria-hidden />
				<ConfirmarSalida
					abierta={pregunta !== null}
					enCurso={pregunta?.enCurso ?? false}
					onSeguir={() => setPregunta(null)}
					onSalir={() => {
						const p = pregunta;
						setPregunta(null);
						if (enCursoAhora()) {
							for (const parar of procesos.current) parar();
							interrumpirRef.current?.();
						}
						p?.salir();
					}}
				/>
				{showCloseButton && (
					<DialogPrimitive.Close
						data-slot="dialog-close"
						onClick={(e) => {
							if (salida.hayQuePreguntar()) {
								e.preventDefault();
								salida.pedirSalida(salida.cerrar);
							}
						}}
						className="absolute top-4 right-4 rounded-xs opacity-70 ring-offset-background transition-opacity hover:opacity-100 focus:ring-2 focus:ring-ring focus:ring-offset-2 focus:outline-hidden disabled:pointer-events-none data-[state=open]:bg-accent data-[state=open]:text-muted-foreground [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4"
					>
						<XIcon />
						<span className="sr-only">Close</span>
					</DialogPrimitive.Close>
				)}
			</DialogPrimitive.Content>
		</DialogPortal>
	);
}

/**
 * El cuerpo desplazable de un diálogo: lo que queda entre la cabecera y los
 * botones, y lo único que se mueve cuando no cabe.
 *
 * 🛑 **`grid` y `overflow-hidden` en la raíz del `ScrollArea` no son adorno:
 * sin ellos NO HAY SCROLL y los botones acaban encima del formulario.**
 *
 * Medido en Chromium, no deducido. El diálogo es una columna flex con
 * `max-height` y **sin `height`**, así que sus hijos no tienen altura definida:
 * el `flex-1` sí le da su alto al `ScrollArea`, pero el viewport de Radix —que
 * es quien recorta— pide `height: 100%`, y un porcentaje contra un padre de
 * alto «auto» vale «auto». Resultado medido: raíz 233 px, viewport 330 px, el
 * contenido cruzando por encima del pie. En cuanto la raíz es una REJILLA, su
 * hijo se estira en el eje de bloque (que en flex-column es el eje principal, y
 * ahí no hay estiramiento), el 100 % se resuelve y vuelve a haber barra.
 * `overflow-hidden` es lo que además permite que la raíz encoja por debajo de
 * su contenido. Es la hermana de la regla de `ScrollArea`: el tope de alto vive
 * en el viewport, no en la raíz.
 *
 * 🛑 Su `DialogContent` sigue teniendo que ser una columna flex — lo es por
 * defecto; si alguien le pone `grid`, el contenido se solapa con los botones.
 *
 * ```tsx
 * <DialogContent className="flex max-w-2xl flex-col">
 *   <DialogHeader className="pr-6">…</DialogHeader>
 *   <DialogCuerpo>…</DialogCuerpo>
 *   <DialogFooter>…</DialogFooter>
 * </DialogContent>
 * ```
 *
 * 🛑 `ScrollArea` y nunca `overflow-y-auto`: la barra nativa no se tiñe con el
 * tema ni frena el desplazamiento, y en una app de escritorio con tres skins
 * eso se ve (regla del proyecto, y paso 8.5 del 26/08/2026).
 *
 * El `-mr-1` / `pr-1` es para que la barra caiga en el borde del diálogo y no
 * se coma un milímetro del texto.
 */
function DialogCuerpo({
	className,
	viewportClassName,
	children,
	...props
}: React.ComponentProps<typeof ScrollArea>) {
	return (
		<ScrollArea
			data-slot="dialog-cuerpo"
			className={cn("-mr-1 grid min-h-0 flex-1 overflow-hidden", className)}
			viewportClassName={cn("pr-1", viewportClassName)}
			{...props}
		>
			{/* Un respiro al pie (12/10/2026): sin él, el borde del último bloque
			    —un aviso, una tarjeta— quedaba pegado al corte del scroll y se
			    veía cortado al llegar abajo. */}
			<div className="pb-1.5">{children}</div>
		</ScrollArea>
	);
}

/**
 * **El formulario de un diálogo**: envuelve el cuerpo Y el pie, para que el
 * botón de enviar quede dentro del `<form>` (si no, `useFormStatus` no se
 * entera de nada, y Enter no envía). Es un `Formulario`: React no lo resetea.
 *
 * ```tsx
 * <DialogContent>
 *   <DialogHeader>…</DialogHeader>
 *   <DialogFormulario action={guardar}>
 *     <DialogCuerpo>…</DialogCuerpo>
 *     <DialogFooter>
 *       <DialogCancelar onClick={cerrar}>Cancelar</DialogCancelar>
 *       <BotonEnviar>Guardar</BotonEnviar>
 *     </DialogFooter>
 *   </DialogFormulario>
 * </DialogContent>
 * ```
 *
 * 🛑 **Es una columna flex con `min-h-0 flex-1`, y no es adorno.** Se mete
 * entre `DialogContent` y `DialogCuerpo`, y si fuera un bloque a secas el
 * cuerpo perdería el alto que le da el diálogo: dejaría de desplazarse y los
 * botones volverían a salirse de la ventana (las reglas de `DialogCuerpo`,
 * más arriba). El `gap-4` y el `min-w-0` repiten los del propio diálogo.
 */
function DialogFormulario({ className, ...props }: React.ComponentProps<typeof Formulario>) {
	return (
		<Formulario
			data-slot="dialog-formulario"
			className={cn("flex min-h-0 flex-1 flex-col gap-4 [&>*]:min-w-0", className)}
			{...props}
		/>
	);
}

function DialogHeader({ className, ...props }: React.ComponentProps<"div">) {
	return (
		<div
			data-slot="dialog-header"
			className={cn("flex flex-col gap-2 text-center sm:text-left", className)}
			{...props}
		/>
	);
}

function DialogFooter({
	className,
	showCloseButton = false,
	children,
	...props
}: React.ComponentProps<"div"> & {
	showCloseButton?: boolean;
}) {
	const salida = React.useContext(SalidaDelDialogo);
	return (
		<div
			data-slot="dialog-footer"
			className={cn("flex flex-col-reverse gap-2 sm:flex-row sm:justify-end", className)}
			{...props}
		>
			{children}
			{showCloseButton && (
				<DialogPrimitive.Close
					asChild
					onClick={(e) => {
						if (salida?.hayQuePreguntar()) {
							e.preventDefault();
							salida.pedirSalida(salida.cerrar);
						}
					}}
				>
					<Button variant="outline">Close</Button>
				</DialogPrimitive.Close>
			)}
		</div>
	);
}

function DialogTitle({ className, ...props }: React.ComponentProps<typeof DialogPrimitive.Title>) {
	return (
		<DialogPrimitive.Title
			data-slot="dialog-title"
			className={cn("text-lg leading-none font-semibold", className)}
			{...props}
		/>
	);
}

function DialogDescription({
	className,
	...props
}: React.ComponentProps<typeof DialogPrimitive.Description>) {
	return (
		<DialogPrimitive.Description
			data-slot="dialog-description"
			className={cn("text-sm text-muted-foreground", className)}
			{...props}
		/>
	);
}

export {
	Dialog,
	DialogCancelar,
	DialogClose,
	DialogContent,
	DialogCuerpo,
	DialogDescription,
	DialogFooter,
	DialogFormulario,
	DialogHeader,
	DialogOverlay,
	DialogPortal,
	DialogTitle,
	DialogTrigger,
};
