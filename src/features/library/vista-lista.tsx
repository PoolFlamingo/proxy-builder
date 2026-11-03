import { useEffect, useMemo, useState } from "react";
import type { DeckDetails } from "@/bindings/DeckDetails";
import type { DeckSummary } from "@/bindings/DeckSummary";
import { Icono } from "@/components/icono";
import { paraTraducir, useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { DropdownMenu, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import { WithTooltip } from "@/components/ui/with-tooltip";
import { useVisible } from "@/features/preview/visibilidad";
import { useFormato } from "@/lib/formato";
import { urlMiniatura } from "@/lib/image-url";
import { usePulsacionLarga } from "@/lib/pulsacion-larga";
import { cn } from "@/lib/utils";
import {
	type ColumnaLista,
	columnasQueCaben,
	type OpcionesLista,
	plantillaLista,
} from "./disposicion";
import { MenuContextualDeck, MenuDeck } from "./menu-deck";
import type { Orden } from "./organizar";
import {
	ChipsEtiquetas,
	CLASES,
	Identidad,
	NombresIdiomas,
	useDinero,
	useFecha,
	useHace,
} from "./piezas";
import { ascendentePorDefecto, usePrefs } from "./preferencias";
import { useBiblioteca } from "./store";
import { PORTADA, useSeleccion } from "./vista-cuadricula";

type Columna = ColumnaLista & {
	titulo: string;
	orden?: Orden;
	derecha?: boolean;
};

/** Las columnas de la lista. `orden`: por dónde ordena pulsar su cabecera. */
const COLUMNAS: Columna[] = [
	{ id: "nombre", titulo: paraTraducir("Nombre"), px: 240, orden: "nombre", movil: true },
	{ id: "colores", titulo: paraTraducir("Colores"), px: 96, orden: "colores" },
	{ id: "clase", titulo: paraTraducir("Clase"), px: 104, quitar: 5 },
	{
		id: "cartas",
		titulo: paraTraducir("Cartas"),
		px: 64,
		orden: "cartas",
		derecha: true,
		movil: true,
	},
	{
		id: "coste",
		titulo: paraTraducir("Coste medio"),
		px: 96,
		orden: "coste",
		derecha: true,
		quitar: 4,
	},
	{
		id: "precio",
		titulo: paraTraducir("Precio"),
		px: 80,
		orden: "precio",
		derecha: true,
		quitar: 6,
	},
	{ id: "idioma", titulo: paraTraducir("Idioma"), px: 120, quitar: 2 },
	{ id: "propias", titulo: paraTraducir("Propias"), px: 64, quitar: 1 },
	{ id: "modificado", titulo: paraTraducir("Modificado"), px: 112, orden: "modificado" },
	{ id: "abierto", titulo: paraTraducir("Abierto"), px: 112, orden: "abierto", quitar: 3 },
];

export function VistaLista({ decks }: { decks: DeckSummary[] }) {
	// Las columnas que caben: según el ancho de la lista.
	const [ancho, setAncho] = useState(1200);
	const [el, setEl] = useState<HTMLDivElement | null>(null);
	useEffect(() => {
		if (!el) return;
		const ro = new ResizeObserver(() => setAncho(el.clientWidth));
		ro.observe(el);
		setAncho(el.clientWidth);
		return () => ro.disconnect();
	}, [el]);
	const { casillas } = useSeleccion();
	const { tamano, tactil } = useFormato();
	const movil = tamano === "movil";
	const o = useMemo<OpcionesLista>(() => ({ casillas, movil, tactil }), [casillas, movil, tactil]);
	const cols = useMemo(() => columnasQueCaben(COLUMNAS, ancho, o), [ancho, o]);
	return (
		<div ref={setEl} className="min-w-0 rounded-lg border bg-card/40">
			<Cabecera decks={decks} cols={cols} o={o} />
			<div className="divide-y">
				{decks.map((d) => (
					<Fila key={d.id} deck={d} cols={cols} o={o} />
				))}
			</div>
		</div>
	);
}

function Cabecera({ decks, cols, o }: { decks: DeckSummary[]; cols: Columna[]; o: OpcionesLista }) {
	const { t } = useLang();
	const sel = useSeleccion();
	const { orden, ascendente, poner } = usePrefs();
	const todos = decks.length > 0 && decks.every((d) => sel.seleccion.includes(d.id));
	const alguno = decks.some((d) => sel.seleccion.includes(d.id));
	return (
		<div
			className="grid items-center gap-3 rounded-t-lg border-b bg-card px-2 py-1.5 text-[10px] font-semibold tracking-wide text-muted-foreground uppercase [&>*]:min-w-0"
			style={{ gridTemplateColumns: plantillaLista(cols, o) }}
		>
			{sel.casillas && (
				<Checkbox
					checked={todos ? true : alguno ? "indeterminate" : false}
					onCheckedChange={() =>
						sel.marcar(
							decks.map((d) => d.id),
							!todos,
						)
					}
					aria-label={t("Seleccionar todos")}
				/>
			)}
			<span />
			{cols.map((c) => (
				<div key={c.id} className={cn("flex", c.derecha && "justify-end")}>
					{c.orden ? (
						<button
							type="button"
							className={cn(
								"inline-flex min-w-0 items-center gap-0.5 whitespace-nowrap uppercase hover:text-foreground",
								orden === c.orden && "text-foreground",
							)}
							onClick={() => {
								const o = c.orden;
								if (!o) return;
								poner(
									orden === o
										? { ascendente: !ascendente }
										: { orden: o, ascendente: ascendentePorDefecto(o) },
								);
							}}
						>
							{t(c.titulo)}
							{orden === c.orden &&
								(ascendente ? (
									<Icono icon="lucide:arrow-up" className="size-3" />
								) : (
									<Icono icon="lucide:arrow-down" className="size-3" />
								))}
						</button>
					) : (
						<span className="truncate">{t(c.titulo)}</span>
					)}
				</div>
			))}
			<span />
		</div>
	);
}

function Fila({ deck: d, cols, o }: { deck: DeckSummary; cols: Columna[]; o: OpcionesLista }) {
	const { t } = useLang();
	const sel = useSeleccion();
	const det = useBiblioteca((s) => s.detalles[d.id]);
	const etiquetas = useBiblioteca((s) => s.etiquetas);
	const fecha = useFecha();
	const hace = useHace();
	const [ref, setRef] = useState<HTMLDivElement | null>(null);
	const visible = useVisible(ref);
	const marcado = sel.seleccion.includes(d.id);
	// Con el dedo y casillas, mantener pulsado elige la fila (el menú es el «⋯»).
	const eligeAlMantener = o.tactil && sel.casillas && !!sel.entrarSeleccion;
	const larga = usePulsacionLarga(() => sel.entrarSeleccion?.(d.id));
	const celda = (id: string) => {
		switch (id) {
			case "nombre":
				return (
					<div className="flex min-w-0 flex-col gap-0.5">
						<div className="flex min-w-0 items-center gap-1">
							{d.pinned && (
								<Icono
									icon="lucide:star"
									className="size-3 shrink-0 fill-amber-400 text-amber-500"
								/>
							)}
							<button
								type="button"
								onClick={(e) => sel.pulsar(d.id, e)}
								className="truncate text-left font-semibold hover:underline"
							>
								{d.name}
							</button>
							{d.notes && (
								<WithTooltip content={<span className="whitespace-pre-line">{d.notes}</span>}>
									<Icono
										icon="lucide:sticky-note"
										className="size-3 shrink-0 text-muted-foreground"
									/>
								</WithTooltip>
							)}
						</div>
						<div className="flex min-w-0 items-center gap-1.5">
							{o.movil && det && det.deckSize > 0 && <Identidad colores={det.colorIdentity} />}
							{d.coverName && (
								<span className="truncate text-[10px] text-muted-foreground">{d.coverName}</span>
							)}
							<ChipsEtiquetas ids={d.tags} etiquetas={etiquetas} max={4} className="flex-nowrap" />
						</div>
					</div>
				);
			case "colores":
				if (!det) return <Cargando />;
				return det.deckSize > 0 ? <Identidad colores={det.colorIdentity} /> : <Vacio />;
			case "clase":
				if (det?.deckSize === 0) return <Vacio />;
				return det ? (
					<span className="truncate">
						{t(CLASES[det.kind])}
						{det.bracket !== null && (
							<WithTooltip content={t("Bracket mínimo que permiten las cartas")}>
								<span className="ml-1 text-muted-foreground">B{det.bracket}</span>
							</WithTooltip>
						)}
					</span>
				) : (
					<Cargando />
				);
			case "cartas":
				return (
					<div className="flex flex-col items-end font-mono tabular-nums">
						<span>{det ? det.deckSize : d.cardCount}</span>
						{det && det.tokens > 0 && (
							<span className="text-[10px] text-muted-foreground">
								{t("+{n} fichas", { n: det.tokens })}
							</span>
						)}
					</div>
				);
			case "coste":
				return det ? <Coste det={det} /> : <Cargando />;
			case "precio":
				return det ? <Precio det={det} /> : <Cargando />;
			case "idioma":
				return det || d.langOverride ? <NombresIdiomas deck={d} det={det} /> : <Cargando />;
			case "propias":
				return det ? <Propias det={det} /> : <Cargando />;
			case "modificado":
				return (
					<WithTooltip content={fecha(d.updatedAt)}>
						<span className="truncate text-muted-foreground">{hace(d.updatedAt)}</span>
					</WithTooltip>
				);
			case "abierto":
				return (
					<WithTooltip content={d.openedAt ? fecha(d.openedAt) : null}>
						<span className="truncate text-muted-foreground">{hace(d.openedAt)}</span>
					</WithTooltip>
				);
		}
		return null;
	};
	return (
		<MenuContextualDeck deck={d} deshabilitado={eligeAlMantener}>
			<div
				ref={setRef}
				{...(eligeAlMantener ? larga : {})}
				data-pulsacion-larga={eligeAlMantener ? "" : undefined}
				onDoubleClick={() => (sel.alDobleClic ?? sel.abrir)(d.id)}
				className={cn(
					"group grid items-center gap-3 px-2 py-1.5 text-xs hover:bg-accent/50 [&>*]:min-w-0",
					marcado && "bg-primary/10 hover:bg-primary/15",
					d.id === sel.abierto && "shadow-[inset_3px_0_0_var(--primary)]",
				)}
				style={{ gridTemplateColumns: plantillaLista(cols, o) }}
			>
				{sel.casillas && (
					<Checkbox
						checked={marcado}
						onClick={(e) => {
							e.preventDefault();
							sel.alternar(d.id, e);
						}}
						aria-label={t("Seleccionar")}
					/>
				)}
				<button
					type="button"
					onClick={(e) => sel.pulsar(d.id, e)}
					className="flex aspect-[63/88] w-8 items-center justify-center overflow-hidden rounded-[3px] bg-muted"
				>
					{d.thumbnail && visible ? (
						<img
							src={urlMiniatura(d.thumbnail, PORTADA, 146)}
							alt=""
							loading="lazy"
							draggable={false}
							className="size-full object-cover"
						/>
					) : (
						<Icono icon="lucide:layers" className="size-3.5 text-muted-foreground" />
					)}
				</button>
				{cols.map((c) => (
					<div key={c.id} className={cn("flex min-w-0", c.derecha && "justify-end")}>
						{celda(c.id)}
					</div>
				))}
				<DropdownMenu>
					<DropdownMenuTrigger asChild>
						<Button size="icon-xs" variant="ghost" aria-label={t("Más acciones")}>
							<Icono icon="lucide:more-horizontal" />
						</Button>
					</DropdownMenuTrigger>
					<MenuDeck deck={d} />
				</DropdownMenu>
			</div>
		</MenuContextualDeck>
	);
}

function Cargando() {
	return <span className="h-2 w-8 animate-pulse rounded bg-muted" />;
}

function Coste({ det }: { det: DeckDetails }) {
	const sinTierras = det.deckSize - det.lands - det.unknown;
	return <span>{sinTierras > 0 ? det.averageCmc.toFixed(2) : "—"}</span>;
}

function Precio({ det }: { det: DeckDetails }) {
	const { t } = useLang();
	const dinero = useDinero();
	const texto =
		det.priceEur !== null
			? dinero(det.priceEur)
			: det.priceUsd !== null
				? dinero(det.priceUsd, "USD")
				: "—";
	return det.unknown > 0 && texto !== "—" ? (
		<WithTooltip content={t("Sin contar {n} cartas sin datos de Scryfall", { n: det.unknown })}>
			<span>≈ {texto}</span>
		</WithTooltip>
	) : (
		<span>{texto}</span>
	);
}

/** Imágenes propias (vinculadas) y cartas diseñadas. */
function Propias({ det }: { det: DeckDetails }) {
	const { t } = useLang();
	if (det.custom === 0 && det.designed === 0)
		return <span className="text-muted-foreground">—</span>;
	return (
		<WithTooltip
			content={
				<div className="flex flex-col gap-0.5">
					<span>
						{t("Imágenes propias: {n} ({v} vinculadas)", { n: det.custom, v: det.customLinked })}
					</span>
					<span>{t("Diseñadas: {n} ({l} listas)", { n: det.designed, l: det.designedReady })}</span>
				</div>
			}
		>
			<span className="truncate font-mono tabular-nums">
				{det.custom > 0 && `${det.custom}`}
				{det.designed > 0 && (
					<span className="ml-1 inline-flex items-center gap-0.5 text-primary">
						<Icono icon="lucide:pencil" className="size-2.5" />
						{det.designed}
					</span>
				)}
			</span>
		</WithTooltip>
	);
}

/** Una celda sin dato (un deck vacío). */
function Vacio() {
	return <span className="text-muted-foreground">—</span>;
}
