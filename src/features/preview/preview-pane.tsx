import {
	DndContext,
	type DragEndEvent,
	DragOverlay,
	type DragStartEvent,
	PointerSensor,
	useSensor,
	useSensors,
} from "@dnd-kit/core";
import {
	AlertTriangle,
	BarChart3,
	Eye,
	FilePlus2,
	Info,
	Layers,
	Minus,
	Plus,
	SquareStack,
	Wand2,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { toast } from "sonner";
import type { Page } from "@/bindings/Page";
import type { RectMm } from "@/bindings/RectMm";
import { Confirmar } from "@/components/confirmar";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import {
	Empty,
	EmptyDescription,
	EmptyHeader,
	EmptyMedia,
	EmptyTitle,
} from "@/components/ui/empty";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Toggle } from "@/components/ui/toggle";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { WithTooltip } from "@/components/ui/with-tooltip";
import { ANCHOS, type AnchoMiniatura } from "@/lib/image-url";
import { useDeck } from "@/stores/deck-store";
import { usePlan } from "@/stores/plan-store";
import { useUi, ZOOM_MAX, ZOOM_MIN } from "@/stores/ui-store";
import {
	anadirPagina,
	autoOrdenar,
	duplicarCartas,
	eliminarCartas,
	objetivo,
	soltarSobre,
} from "./acciones-carta";
import { useArrastrarFicheros } from "./arrastrar-ficheros";
import { PageView } from "./page-view";
import { RaizScroll } from "./visibilidad";

/** Píxeles CSS por milímetro a zoom 100 %. */
const PX_POR_MM = 96 / 25.4;

/** El ancho de miniatura más pequeño que cubre lo que se pinta (con el tope elegido). */
function anchoPara(mmCarta: number, escala: number, tope: AnchoMiniatura): AnchoMiniatura {
	const necesario = mmCarta * escala * (window.devicePixelRatio || 1);
	const cabe = ANCHOS.find((a) => a >= necesario) ?? ANCHOS[ANCHOS.length - 1];
	return Math.min(cabe, tope) as AnchoMiniatura;
}

/**
 * **Panel central**: las hojas como saldrán impresas (las caras frontales;
 * «Ver reversos» enseña en cada casilla el dorso que llevará).
 */
export function PreviewPane() {
	const { t, mensaje } = useLang();
	const deck = useDeck((s) => s.deck);
	const plan = usePlan((s) => s.plan);
	const errorPlan = usePlan((s) => s.error);
	const zoom = useUi((s) => s.zoom);
	const setZoom = useUi((s) => s.setZoom);
	const infoCarta = useUi((s) => s.infoCarta);
	const verReversos = useUi((s) => s.verReversos);
	const calidad = useUi((s) => s.calidadVista);
	const [viewport, setViewport] = useState<HTMLDivElement | null>(null);
	const [borrar, setBorrar] = useState<Page | null>(null);
	const [arrastrando, setArrastrando] = useState<string[] | null>(null);
	useArrastrarFicheros();
	useTeclado();

	const escala = zoom * PX_POR_MM;
	const ajustes = deck?.settings;
	const cartas = useMemo(() => new Map((deck?.cards ?? []).map((c) => [c.id, c])), [deck?.cards]);
	const frentes = useMemo(() => (plan?.pages ?? []).filter((p) => p.side === "front"), [plan]);
	const ancho = ajustes ? anchoPara(ajustes.cardSize.widthMm, escala, calidad) : calidad;
	const porPagina = plan ? plan.grid.cols * plan.grid.rows : 9;

	// Las casillas vacías de la última hoja.
	const vacias = useMemo<RectMm[]>(() => {
		const ultima = frentes.at(-1);
		if (!plan || !ultima) return [];
		const ocupadas = new Set(ultima.slots.map((s) => s.index));
		return plan.cells.filter((_, i) => !ocupadas.has(i));
	}, [plan, frentes]);

	const sensores = useSensors(useSensor(PointerSensor, { activationConstraint: { distance: 6 } }));
	const alEmpezar = useCallback((e: DragStartEvent) => {
		setArrastrando(objetivo(String(e.active.id)));
	}, []);
	const alSoltar = useCallback(
		(e: DragEndEvent) => {
			const ids = arrastrando ?? [String(e.active.id)];
			setArrastrando(null);
			if (!e.over) return;
			const destino = String(e.over.id);
			soltarSobre(ids, destino.startsWith("vacia:") ? null : destino);
		},
		[arrastrando],
	);

	const onBorrar = useCallback((p: Page) => setBorrar(p), []);

	if (!deck || !ajustes) return <section className="h-full" />;

	const avisos = plan?.warnings ?? [];
	const papel = ajustes.landscape ? t("{papel} apaisado", { papel: ajustes.paper }) : ajustes.paper;

	return (
		<section className="flex h-full min-h-0 flex-col bg-muted/30 [&_[data-slot=button]]:text-xs">
			<div className="flex flex-wrap items-center gap-1.5 border-b bg-background px-2.5 py-1.5">
				<WithTooltip content={t("La vista es aproximada; el resultado exacto es el del PDF.")}>
					<span className="mr-1 flex items-center gap-1.5 text-xs font-medium">
						<Eye className="size-3.5 text-muted-foreground" />
						{t("Vista {papel}", { papel })}
					</span>
				</WithTooltip>
				<WithTooltip
					content={t("Junta las copias, quita los huecos y deja las fijadas en su sitio")}
				>
					<Button
						size="xs"
						variant="outline"
						disabled={deck.cards.length < 2}
						onClick={() => autoOrdenar(t).catch((e) => toast.error(mensaje(e)))}
					>
						<Wand2 />
						{t("Auto ordenar")}
					</Button>
				</WithTooltip>
				<Toggle
					size="sm"
					variant="outline"
					className="h-6 gap-1 px-2 text-xs"
					pressed={infoCarta}
					onPressedChange={(v) => useUi.getState().setInfoCarta(v)}
				>
					<Info className="size-3" />
					{t("Info de carta")}
				</Toggle>
				<Toggle
					size="sm"
					variant="outline"
					className="h-6 gap-1 px-2 text-xs"
					pressed={verReversos}
					onPressedChange={(v) => useUi.getState().setVerReversos(v)}
				>
					<SquareStack className="size-3" />
					{t("Ver reversos")}
				</Toggle>
				<ToggleGroup
					type="single"
					variant="outline"
					size="sm"
					value={ajustes.dfcFilter}
					onValueChange={(v) =>
						(v === "all" || v === "hideDfc") &&
						useDeck.getState().setAjustes((s) => ({ ...s, dfcFilter: v }))
					}
				>
					<ToggleGroupItem value="all" className="h-6 px-2 text-xs">
						{t("Todas")}
					</ToggleGroupItem>
					<WithTooltip content={t("Quita las cartas de doble cara de las hojas y del PDF")}>
						<ToggleGroupItem value="hideDfc" className="h-6 px-2 text-xs">
							{t("Ocultar DFC")}
						</ToggleGroupItem>
					</WithTooltip>
				</ToggleGroup>
				<WithTooltip content={t("Curva de maná, colores, precio y legalidad")}>
					<Button
						size="xs"
						variant="outline"
						disabled={deck.cards.length === 0}
						onClick={() => useUi.getState().abrir({ tipo: "estadisticas" })}
					>
						<BarChart3 />
						{t("Estadísticas")}
					</Button>
				</WithTooltip>
				<div className="ml-auto flex items-center gap-1">
					<Button
						size="icon-xs"
						variant="ghost"
						onClick={() => setZoom(zoom - 0.1)}
						disabled={zoom <= ZOOM_MIN}
						aria-label={t("Alejar")}
					>
						<Minus />
					</Button>
					<button
						type="button"
						onClick={() => setZoom(1)}
						className="w-11 text-center font-mono text-[11px] text-muted-foreground hover:text-foreground"
					>
						{Math.round(zoom * 100)} %
					</button>
					<Button
						size="icon-xs"
						variant="ghost"
						onClick={() => setZoom(zoom + 0.1)}
						disabled={zoom >= ZOOM_MAX}
						aria-label={t("Acercar")}
					>
						<Plus />
					</Button>
					<Button size="xs" onClick={() => useUi.getState().abrir({ tipo: "anadir" })}>
						<Plus />
						{t("Añadir carta")}
					</Button>
				</div>
			</div>

			{(avisos.length > 0 || errorPlan !== null) && (
				<div className="flex flex-col gap-1 border-b bg-amber-500/10 px-2.5 py-1.5 text-[11px] text-amber-700 dark:text-amber-300">
					{errorPlan !== null && (
						<span className="flex items-center gap-1.5">
							<AlertTriangle className="size-3.5" />
							{mensaje(errorPlan)}
						</span>
					)}
					{avisos.map((w) => (
						<span key={w.kind} className="flex items-center gap-1.5">
							<AlertTriangle className="size-3.5" />
							{w.kind === "dfcWithoutBack"
								? t(
										"{n} cartas de doble cara sin dúplex: su otra cara no se imprime (activa «Pares DFC intercalados» o el dúplex).",
										{ n: w.count },
									)
								: t("{n} cartas sin reverso: el deck no tiene reverso por defecto.", {
										n: w.count,
									})}
						</span>
					))}
				</div>
			)}

			<div className="relative min-h-0 flex-1">
				<ScrollArea className="h-full" onViewportChange={setViewport}>
					<RaizScroll.Provider value={viewport}>
						<DndContext
							sensors={sensores}
							onDragStart={alEmpezar}
							onDragEnd={alSoltar}
							onDragCancel={() => setArrastrando(null)}
						>
							{frentes.length === 0 ? (
								<Empty className="mt-16">
									<EmptyHeader>
										<EmptyMedia variant="icon">
											<Layers />
										</EmptyMedia>
										<EmptyTitle>{t("El deck está vacío")}</EmptyTitle>
										<EmptyDescription>
											{t(
												"Pega una lista a la izquierda y pulsa «Cargar lista», o añade cartas una a una.",
											)}
										</EmptyDescription>
									</EmptyHeader>
									<Button size="sm" onClick={() => useUi.getState().abrir({ tipo: "anadir" })}>
										<Plus />
										{t("Añadir carta")}
									</Button>
								</Empty>
							) : (
								<div className="flex flex-col items-center gap-6 px-4 py-5 pb-20">
									{frentes.map((p, i) => (
										<PageView
											key={p.index}
											pagina={p}
											numero={i + 1}
											total={frentes.length}
											escala={escala}
											cartas={cartas}
											ajustes={ajustes}
											dorsoDeck={deck.defaultBack}
											ancho={ancho}
											celdas={i === frentes.length - 1 ? vacias : []}
											huecosLibres={verReversos === false}
											onBorrar={onBorrar}
										/>
									))}
								</div>
							)}
							<DragOverlay dropAnimation={null}>
								{arrastrando && (
									<div className="rounded-md bg-primary px-2 py-1 text-xs font-medium text-primary-foreground shadow-lg">
										{arrastrando.length === 1
											? cartas.get(arrastrando[0])?.meta.name || t("Hueco")
											: t("{n} cartas", { n: arrastrando.length })}
									</div>
								)}
							</DragOverlay>
						</DndContext>
					</RaizScroll.Provider>
				</ScrollArea>
				<Button
					size="sm"
					variant="secondary"
					className="absolute right-4 bottom-4 shadow-lg"
					onClick={() => anadirPagina(porPagina)}
				>
					<FilePlus2 />
					{t("Añadir página")}
				</Button>
			</div>

			<Confirmar
				abierto={borrar !== null}
				onCerrar={() => setBorrar(null)}
				titulo={t("¿Eliminar las cartas de esta página?")}
				descripcion={t("Se puede deshacer con Ctrl+Z.")}
				accion={t("Eliminar")}
				destructiva
				onConfirmar={() => {
					if (!borrar) return;
					eliminarCartas([...new Set(borrar.slots.map((s) => s.cardId))]);
				}}
			/>
		</section>
	);
}

/** Supr borra la selección, Ctrl+D la duplica, Ctrl+A selecciona todo y Esc la quita. */
function useTeclado() {
	useEffect(() => {
		const alTeclear = (e: KeyboardEvent) => {
			const el = e.target as HTMLElement | null;
			if (
				el?.closest(
					"input, textarea, select, [contenteditable='true'], [role='dialog'], [role='menu']",
				)
			)
				return;
			const ui = useUi.getState();
			const mod = e.ctrlKey || e.metaKey;
			if (e.key === "Delete" && ui.seleccion.length) {
				e.preventDefault();
				eliminarCartas(ui.seleccion);
			} else if (mod && e.key.toLowerCase() === "d" && ui.seleccion.length) {
				e.preventDefault();
				duplicarCartas(ui.seleccion);
			} else if (mod && e.key.toLowerCase() === "a") {
				e.preventDefault();
				ui.seleccionar(useDeck.getState().deck?.cards.map((c) => c.id) ?? [], null);
			} else if (e.key === "Escape" && ui.seleccion.length) {
				ui.seleccionar([], null);
			}
		};
		window.addEventListener("keydown", alTeclear);
		return () => window.removeEventListener("keydown", alTeclear);
	}, []);
}
