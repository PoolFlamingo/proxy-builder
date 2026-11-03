import { createContext, type ReactNode, useContext, useState } from "react";
import { Hoja } from "@/components/hoja";
import { Icono } from "@/components/icono";
import { Button } from "@/components/ui/button";
import { Dialog, DialogCuerpo, DialogDescription, DialogTitle } from "@/components/ui/dialog";
import { cn } from "@/lib/utils";

/**
 * **El patrón de las hojas del móvil**, igual en toda la app: sube desde
 * abajo, con una cabecera de 3 rem (el título a la izquierda y la X a la
 * derecha, siempre en el mismo sitio) y debajo un cuerpo que se desplaza con
 * el dedo. Un menú es una hoja con filas (`FilaMenu`); un panel de opciones,
 * una hoja con lo que lleve dentro.
 *
 * Lo pinta `Hoja` (que no se toca): aquí solo se fijan la cabecera, la X y
 * el cuerpo para que todas se vean iguales.
 */
export const CLASES_HOJA_MENU = cn(
	"gap-0 p-0",
	// La X de `DialogContent`, centrada en la cabecera y del tamaño de un botón.
	"[&>[data-slot=dialog-close]]:top-1 [&>[data-slot=dialog-close]]:right-1 [&>[data-slot=dialog-close]]:flex [&>[data-slot=dialog-close]]:size-10 [&>[data-slot=dialog-close]]:items-center [&>[data-slot=dialog-close]]:justify-center [&>[data-slot=dialog-close]]:rounded-lg [&>[data-slot=dialog-close]]:opacity-100",
);

/** Cierra la hoja de un `BotonMenu` (lo usa `FilaMenu` al pulsar). */
const CerrarHoja = createContext<(() => void) | null>(null);

export function HojaMenu({
	abierta,
	onCerrar,
	titulo,
	children,
	pie,
	className,
	cuerpoClassName,
	sinRelleno = false,
}: {
	abierta: boolean;
	onCerrar: () => void;
	titulo: ReactNode;
	children: ReactNode;
	/** Lo que va fijo abajo (botones), fuera del scroll. */
	pie?: ReactNode;
	className?: string;
	cuerpoClassName?: string;
	/** Sin el relleno de 1 rem del cuerpo (una lista que pone el suyo). */
	sinRelleno?: boolean;
}) {
	return (
		<Dialog open={abierta} onOpenChange={(o) => !o && onCerrar()}>
			<Hoja
				cambiosSinGuardar={false}
				aria-describedby={undefined}
				// Sin foco al abrir: el primer botón enseñaría su tooltip al recibirlo.
				onOpenAutoFocus={(e) => e.preventDefault()}
				className={cn(CLASES_HOJA_MENU, className)}
			>
				<CerrarHoja.Provider value={onCerrar}>
					<CabeceraHoja titulo={titulo} />
					<DialogCuerpo className="mr-0" viewportClassName="pr-0">
						<div
							className={cn(
								"flex flex-col",
								sinRelleno ? "py-1" : "gap-4 p-4",
								!pie && "pb-[max(1rem,env(safe-area-inset-bottom))]",
								cuerpoClassName,
							)}
						>
							{children}
						</div>
					</DialogCuerpo>
					{pie && (
						<div className="flex shrink-0 flex-col gap-2 border-t p-4 pb-[max(1rem,env(safe-area-inset-bottom))]">
							{pie}
						</div>
					)}
				</CerrarHoja.Provider>
			</Hoja>
		</Dialog>
	);
}

/**
 * La cabecera de una hoja: 3 rem, el título a la izquierda y sitio para la X
 * (que `CLASES_HOJA_MENU` centra a su altura). Para las hojas que no usan
 * `HojaMenu` entera (un cuerpo con su propio scroll).
 */
export function CabeceraHoja({
	titulo,
	descripcion,
}: {
	titulo: ReactNode;
	/** Para el lector de pantalla (no se ve). */
	descripcion?: string;
}) {
	return (
		<div className="flex h-12 shrink-0 items-center border-b pr-12 pl-4">
			<DialogTitle className="min-w-0 truncate text-sm font-semibold">{titulo}</DialogTitle>
			{descripcion && <DialogDescription className="sr-only">{descripcion}</DialogDescription>}
		</div>
	);
}

/**
 * Una fila de menú dentro de una `HojaMenu`: icono, texto y, a la derecha, lo
 * que haga falta (una cuenta, un interruptor). Al pulsarla, la hoja se cierra
 * (salvo `mantener`).
 */
export function FilaMenu({
	icono,
	texto,
	onClick,
	destructiva,
	disabled,
	activa,
	mantener = false,
	marca,
	extra,
}: {
	icono?: string;
	texto: ReactNode;
	onClick: () => void;
	destructiva?: boolean;
	disabled?: boolean;
	/** Elegida (un orden, un filtro): en el color de la app. */
	activa?: boolean;
	/** No cerrar la hoja al pulsar. */
	mantener?: boolean;
	/** Algo pegado al icono (el punto de la actualización). */
	marca?: ReactNode;
	extra?: ReactNode;
}) {
	const cerrar = useContext(CerrarHoja);
	return (
		<button
			type="button"
			disabled={disabled}
			onClick={() => {
				if (!mantener) cerrar?.();
				onClick();
			}}
			className={cn(
				"flex min-h-11 w-full items-center gap-3 px-4 text-left text-sm active:bg-accent disabled:opacity-50",
				destructiva && "text-destructive",
				activa && "text-primary",
			)}
		>
			{icono && (
				<span className="relative shrink-0">
					<Icono
						icon={icono}
						className={cn("size-5", !destructiva && !activa && "text-muted-foreground")}
					/>
					{marca}
				</span>
			)}
			<span className="min-w-0 flex-1 truncate">{texto}</span>
			{extra}
		</button>
	);
}

/**
 * El «⋯» de una barra del móvil: un botón de icono que abre una `HojaMenu`
 * con las acciones secundarias (`FilaMenu`).
 */
export function BotonMenu({
	etiqueta,
	titulo,
	children,
	icono = "lucide:more-horizontal",
	className,
}: {
	/** El `aria-label` del botón (y lo que enseña al mantenerlo pulsado). */
	etiqueta: string;
	/** El título de la hoja (por defecto, `etiqueta`). */
	titulo?: ReactNode;
	children: ReactNode;
	icono?: string;
	className?: string;
}) {
	const [abierta, setAbierta] = useState(false);
	return (
		<>
			<Button
				size="icon-sm"
				variant="ghost"
				aria-label={etiqueta}
				aria-haspopup="dialog"
				aria-expanded={abierta}
				onClick={() => setAbierta(true)}
				className={className}
			>
				<Icono icon={icono} />
			</Button>
			<HojaMenu
				abierta={abierta}
				onCerrar={() => setAbierta(false)}
				titulo={titulo ?? etiqueta}
				sinRelleno
			>
				{children}
			</HojaMenu>
		</>
	);
}

/** Cierra la `HojaMenu` en la que se está (o nada, fuera de una). */
function useCerrarHoja(): () => void {
	const cerrar = useContext(CerrarHoja);
	return cerrar ?? (() => {});
}

const CLASES_FILA =
	"flex min-h-11 w-full items-center gap-3 px-4 text-left text-sm active:bg-accent disabled:opacity-50 [&_svg]:size-5 [&_svg]:shrink-0";

/**
 * Las piezas de un menú de Radix (`Item`, `Sub`, `CheckboxItem`…) pintadas
 * como filas de `HojaMenu`, para que un mismo menú se abra en el móvil como
 * hoja. Un submenú se despliega debajo de su fila. `onSelect` recibe un
 * evento: con `preventDefault()` la hoja sigue abierta (como en Radix).
 */
export function ItemHoja({
	onSelect,
	variant,
	disabled,
	children,
}: {
	onSelect?: () => void;
	variant?: "default" | "destructive";
	disabled?: boolean;
	children: ReactNode;
}) {
	const cerrar = useCerrarHoja();
	return (
		<button
			type="button"
			disabled={disabled}
			onClick={() => {
				cerrar();
				onSelect?.();
			}}
			className={cn(
				CLASES_FILA,
				variant === "destructive" ? "text-destructive" : "[&_svg]:text-muted-foreground",
			)}
		>
			{children}
		</button>
	);
}

export function SeparadorHoja() {
	return <div className="my-1 h-px bg-border" />;
}

const SubHojaAbierto = createContext<[boolean, (v: boolean) => void]>([false, () => {}]);

export function SubHoja({ children }: { children: ReactNode }) {
	const estado = useState(false);
	return <SubHojaAbierto.Provider value={estado}>{children}</SubHojaAbierto.Provider>;
}

export function SubHojaFila({ children }: { children: ReactNode }) {
	const [abierto, poner] = useContext(SubHojaAbierto);
	return (
		<button
			type="button"
			aria-expanded={abierto}
			onClick={() => poner(!abierto)}
			className={cn(CLASES_FILA, "[&_svg]:text-muted-foreground")}
		>
			{children}
			<Icono
				icon="lucide:chevron-right"
				className={cn("ml-auto transition-transform", abierto && "rotate-90")}
			/>
		</button>
	);
}

export function SubHojaContenido({ children }: { className?: string; children: ReactNode }) {
	const [abierto] = useContext(SubHojaAbierto);
	if (!abierto) return null;
	return <div className="flex flex-col border-y bg-muted/30 py-1 pl-4">{children}</div>;
}

export function CasillaHoja({
	checked,
	onSelect,
	children,
}: {
	checked?: boolean | "indeterminate";
	onSelect?: (e: Event) => void;
	children: ReactNode;
}) {
	const cerrar = useCerrarHoja();
	return (
		<button
			type="button"
			role="menuitemcheckbox"
			aria-checked={checked === "indeterminate" ? "mixed" : !!checked}
			onClick={() => {
				const e = new Event("select", { cancelable: true });
				onSelect?.(e);
				if (!e.defaultPrevented) cerrar();
			}}
			className={CLASES_FILA}
		>
			<span className="flex size-5 shrink-0 items-center justify-center text-primary">
				{checked === true && <Icono icon="lucide:check" />}
				{checked === "indeterminate" && <Icono icon="lucide:minus" />}
			</span>
			{children}
		</button>
	);
}
