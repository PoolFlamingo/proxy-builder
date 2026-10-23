import {
	ChevronRight,
	FilterX,
	HardDriveDownload,
	Import,
	Layers,
	MoreHorizontal,
	Plus,
	Star,
	Trash2,
	X,
} from "lucide-react";
import { type ReactNode, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { toast } from "sonner";
import type { DeckSummary } from "@/bindings/DeckSummary";
import type { Tag } from "@/bindings/Tag";
import { Confirmar } from "@/components/confirmar";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import {
	Dialog,
	DialogContent,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { DropdownMenu, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import {
	Empty,
	EmptyContent,
	EmptyDescription,
	EmptyHeader,
	EmptyMedia,
	EmptyTitle,
} from "@/components/ui/empty";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import { megas, nombreSeguro } from "@/features/export/acciones";
import { RaizScroll } from "@/features/preview/visibilidad";
import { abrirParaImportar, guardarParaExportar } from "@/lib/dialogos";
import { call } from "@/lib/ipc";
import { cn } from "@/lib/utils";
import { abrirDeck, guardarAhora, useDeck } from "@/stores/deck-store";
import { useUi } from "@/stores/ui-store";
import { BarraHerramientas, FiltrosActivos } from "./barra-herramientas";
import { BarraLateral } from "./barra-lateral";
import { type EditandoEtiqueta, EtiquetaDialog } from "./etiqueta-dialog";
import { type Acciones, AccionesDeck, MenuDeck } from "./menu-deck";
import { agrupar, type ClaveGrupo, filtrar, hayFiltros, ordenar, seleccionar } from "./organizar";
import { ChipEtiqueta, CLASES, IDIOMAS, Identidad, nombreColores, useDinero } from "./piezas";
import { useFiltrosBiblioteca, usePrefsBiblioteca } from "./preferencias";
import { PropiedadesDialog } from "./propiedades-dialog";
import { useBiblioteca } from "./store";
import { type Seleccion, SeleccionDecks, VistaCuadricula } from "./vista-cuadricula";
import { VistaLista } from "./vista-lista";

/**
 * **Biblioteca**: los decks guardados en este equipo, con etiquetas,
 * favoritos, búsqueda, filtros, orden, grupos y dos vistas (cuadrícula y
 * lista). Abrir, renombrar, duplicar, borrar (uno o varios), llevarse un deck
 * con sus imágenes (`.pbdeck`) y liberar el espacio de las imágenes que ya no
 * usa nadie.
 */
export function LibraryScreen() {
	const { t, mensaje } = useLang();
	const actual = useDeck((s) => s.deck?.id ?? null);
	const decks = useBiblioteca((s) => s.decks);
	const detalles = useBiblioteca((s) => s.detalles);
	const etiquetas = useBiblioteca((s) => s.etiquetas);
	const contando = useBiblioteca((s) => s.contando);
	const prefs = usePrefsBiblioteca();
	const { filtros, ponerFiltros, limpiar } = useFiltrosBiblioteca();

	const [seleccion, setSeleccion] = useState<string[]>([]);
	const ancla = useRef<string | null>(null);
	const [plegados, setPlegados] = useState<Set<string>>(new Set());
	const [renombrar, setRenombrar] = useState<DeckSummary | null>(null);
	const [borrar, setBorrar] = useState<string[] | null>(null);
	const [propiedades, setPropiedades] = useState<string | null>(null);
	const [editandoEtiqueta, setEditandoEtiqueta] = useState<EditandoEtiqueta | null>(null);
	const [borrarEtiqueta, setBorrarEtiqueta] = useState<Tag | null>(null);
	const [viewport, setViewport] = useState<HTMLDivElement | null>(null);
	const buscar = useRef<HTMLInputElement>(null);

	const intentar = useCallback(
		<A extends unknown[]>(f: (...a: A) => Promise<unknown>) =>
			(...a: A) => {
				f(...a).catch((e) => toast.error(mensaje(e)));
			},
		[mensaje],
	);

	useEffect(() => {
		useBiblioteca
			.getState()
			.cargar()
			.catch((e) => toast.error(mensaje(e)));
	}, [mensaje]);

	// Lo que se ve: filtrado, ordenado y repartido en grupos.
	const visibles = useMemo(
		() =>
			ordenar(
				filtrar(decks ?? [], detalles, etiquetas, filtros),
				detalles,
				prefs.orden,
				prefs.ascendente,
				prefs.favoritosArriba,
			),
		[decks, detalles, etiquetas, filtros, prefs.orden, prefs.ascendente, prefs.favoritosArriba],
	);
	const grupos = useMemo(
		() => agrupar(visibles, detalles, etiquetas, prefs.agrupar),
		[visibles, detalles, etiquetas, prefs.agrupar],
	);
	// En el orden de la pantalla (sin repetir), para Mayús + clic.
	const enPantalla = useMemo(
		() => [
			...new Set(
				grupos.filter((g) => !plegados.has(g.id)).flatMap((g) => g.decks.map((d) => d.id)),
			),
		],
		[grupos, plegados],
	);

	// Fuera de la selección lo que ya no existe.
	useEffect(() => {
		if (!decks) return;
		const hay = new Set(decks.map((d) => d.id));
		setSeleccion((s) => (s.every((id) => hay.has(id)) ? s : s.filter((id) => hay.has(id))));
	}, [decks]);

	const refrescar = () => useBiblioteca.getState().refrescar();
	const recargar = () => useBiblioteca.getState().cargar();

	const abrir = intentar(async (id: string) => {
		await abrirDeck(id);
		useUi.getState().setVista("deck");
	});

	const nuevo = intentar(async () => {
		await guardarAhora();
		const d = await call("deck_create", { name: t("Deck sin título") });
		// Si se está mirando una etiqueta, el deck nuevo nace con ella.
		if (filtros.etiquetas.length === 1)
			await call("deck_tags_set", { decks: [d.id], add: filtros.etiquetas, remove: [] });
		await abrirDeck(d.id);
		useUi.getState().setVista("deck");
	});

	const duplicar = intentar(async (d: DeckSummary) => {
		await guardarAhora();
		await call("deck_duplicate", { id: d.id, name: t("{n} (copia)", { n: d.name }) });
		await recargar();
	});

	const exportarPaquete = intentar(async (d: DeckSummary) => {
		await guardarAhora();
		const destino = await guardarParaExportar(
			{
				title: t("Exportar deck con sus imágenes"),
				filters: [{ name: "Proxy Builder", extensions: ["pbdeck"] }],
			},
			nombreSeguro(d.name, "pbdeck"),
		);
		if (!destino) return;
		const bytes = await call("deck_export_package", { id: d.id, path: destino });
		toast.success(t("Deck exportado ({mb})", { mb: megas(bytes) }), {
			action: {
				label: t("Mostrar en carpeta"),
				onClick: () => void call("export_reveal", { path: destino }),
			},
		});
	});

	const importarPaquete = intentar(async () => {
		const ruta = (
			await abrirParaImportar({
				title: t("Importar deck"),
				multiple: false,
				directory: false,
				filters: [{ name: "Proxy Builder", extensions: ["pbdeck"] }],
			})
		)?.[0];
		if (!ruta) return;
		const d = await call("deck_import_package", { path: ruta });
		toast.success(t("«{n}» importado", { n: d.name }));
		await recargar();
	});

	const liberar = intentar(async () => {
		const r = await call("blob_gc", {});
		toast.success(
			r.removed > 0
				? t("{n} imágenes sin usar borradas ({mb})", { n: r.removed, mb: megas(r.freedBytes) })
				: t("No había imágenes sin usar (se guardan 24 h por si acaso)."),
		);
	});

	const acciones: Acciones = {
		abrir,
		propiedades: setPropiedades,
		renombrar: setRenombrar,
		duplicar,
		exportar: exportarPaquete,
		borrar: setBorrar,
		nuevaEtiqueta: (ids) => setEditandoEtiqueta({ tipo: "nueva", para: ids }),
		seleccion,
	};

	const sel: Seleccion = {
		seleccion,
		abierto: actual,
		pulsar: (id, e) => {
			if (e.shiftKey) {
				setSeleccion((s) => seleccionar(s, enPantalla, id, ancla.current, "tramo"));
			} else if (e.ctrlKey || e.metaKey || seleccion.length > 0) {
				// Con algo elegido, pulsar sigue eligiendo (abrir: doble clic o el menú).
				setSeleccion((s) => seleccionar(s, enPantalla, id, null, "alternar"));
				ancla.current = id;
			} else {
				abrir(id);
			}
		},
		alternar: (id, e) => {
			if (e?.shiftKey) {
				setSeleccion((s) => seleccionar(s, enPantalla, id, ancla.current, "tramo"));
				return;
			}
			setSeleccion((s) => seleccionar(s, enPantalla, id, null, "alternar"));
			ancla.current = id;
		},
		marcar: (ids, si) =>
			setSeleccion((s) => (si ? [...new Set([...s, ...ids])] : s.filter((x) => !ids.includes(x)))),
		abrir,
	};

	// Teclado: Ctrl+F busca, Ctrl+A elige todo lo visible, Esc suelta, Supr borra.
	useEffect(() => {
		const tecla = (e: KeyboardEvent) => {
			const el = e.target as HTMLElement | null;
			if (el?.closest('[role="dialog"],[role="menu"],[role="alertdialog"]')) return;
			const escribiendo = !!el?.closest("input,textarea,[contenteditable=true]");
			const ctrl = e.ctrlKey || e.metaKey;
			if (ctrl && e.key.toLowerCase() === "f") {
				e.preventDefault();
				buscar.current?.focus();
				buscar.current?.select();
			} else if (escribiendo) {
				if (e.key === "Escape" && el === buscar.current && filtros.texto) {
					ponerFiltros({ texto: "" });
				}
			} else if (ctrl && e.key.toLowerCase() === "a") {
				e.preventDefault();
				setSeleccion(enPantalla);
			} else if (e.key === "Escape" && seleccion.length > 0) {
				setSeleccion([]);
			} else if ((e.key === "Delete" || e.key === "Supr") && seleccion.length > 0) {
				setBorrar(seleccion);
			}
		};
		window.addEventListener("keydown", tecla);
		return () => window.removeEventListener("keydown", tecla);
	}, [enPantalla, seleccion, filtros.texto, ponerFiltros]);

	const plegar = (id: string) =>
		setPlegados((p) => {
			const n = new Set(p);
			if (n.has(id)) n.delete(id);
			else n.add(id);
			return n;
		});

	const totalDecks = decks?.length ?? 0;
	const Vista = prefs.vista === "lista" ? VistaLista : VistaCuadricula;
	const conGrupos = prefs.agrupar !== "ninguno";

	return (
		<AccionesDeck.Provider value={acciones}>
			<SeleccionDecks.Provider value={sel}>
				<section className="flex h-full min-h-0 flex-col">
					<BarraHerramientas
						ref={buscar}
						acciones={
							<>
								<Button size="sm" variant="ghost" onClick={liberar}>
									<HardDriveDownload />
									{t("Liberar espacio")}
								</Button>
								<Button size="sm" variant="outline" onClick={importarPaquete}>
									<Import />
									{t("Importar .pbdeck")}
								</Button>
								<Button size="sm" onClick={nuevo}>
									<Plus />
									{t("Nuevo deck")}
								</Button>
							</>
						}
					/>
					<div className="flex min-h-0 flex-1">
						{prefs.lateral && (
							<BarraLateral
								onNuevaEtiqueta={() => setEditandoEtiqueta({ tipo: "nueva", para: seleccion })}
								onEditarEtiqueta={(tag) => setEditandoEtiqueta({ tipo: "editar", tag })}
								onBorrarEtiqueta={setBorrarEtiqueta}
							/>
						)}
						<div className="relative flex min-w-0 flex-1 flex-col" data-tutorial="biblioteca-decks">
							<FiltrosActivos />
							{seleccion.length > 0 && (
								<BarraSeleccion
									seleccion={seleccion}
									visibles={enPantalla}
									onTodos={() => setSeleccion(enPantalla)}
									onSoltar={() => setSeleccion([])}
									onBorrar={() => setBorrar(seleccion)}
								/>
							)}
							<ScrollArea className="min-h-0 flex-1" onViewportChange={setViewport}>
								<RaizScroll.Provider value={viewport}>
									{decks === null ? null : totalDecks === 0 ? (
										<Empty className="mt-16">
											<EmptyHeader>
												<EmptyMedia variant="icon">
													<Layers />
												</EmptyMedia>
												<EmptyTitle>{t("No hay decks guardados")}</EmptyTitle>
												<EmptyDescription>
													{t("Los decks se guardan solos mientras trabajas.")}
												</EmptyDescription>
											</EmptyHeader>
										</Empty>
									) : visibles.length === 0 ? (
										<Empty className="mt-16">
											<EmptyHeader>
												<EmptyMedia variant="icon">
													<FilterX />
												</EmptyMedia>
												<EmptyTitle>{t("Ningún deck coincide")}</EmptyTitle>
												<EmptyDescription>
													{contando
														? t("Aún se están calculando los colores y precios de los decks.")
														: t("Prueba con otra búsqueda o quita algún filtro.")}
												</EmptyDescription>
											</EmptyHeader>
											{hayFiltros(filtros) && (
												<EmptyContent>
													<Button size="sm" variant="outline" onClick={limpiar}>
														<X />
														{t("Quitar los filtros")}
													</Button>
												</EmptyContent>
											)}
										</Empty>
									) : (
										<div className="flex flex-col gap-4 p-4">
											{grupos.map((g) =>
												conGrupos ? (
													<section key={g.id} className="flex flex-col gap-2.5">
														<button
															type="button"
															onClick={() => plegar(g.id)}
															className="flex items-center gap-2 self-start rounded-md px-1 py-0.5 text-xs font-semibold hover:bg-accent"
														>
															<ChevronRight
																className={cn(
																	"size-3.5 text-muted-foreground transition-transform",
																	!plegados.has(g.id) && "rotate-90",
																)}
															/>
															<TituloGrupo clave={g.clave} contando={contando} />
															<span className="font-mono text-[10px] font-normal text-muted-foreground tabular-nums">
																{g.decks.length}
															</span>
														</button>
														{!plegados.has(g.id) && <Vista decks={g.decks} />}
													</section>
												) : (
													<Vista key={g.id} decks={g.decks} />
												),
											)}
										</div>
									)}
								</RaizScroll.Provider>
							</ScrollArea>
							<Pie visibles={visibles} total={totalDecks} />
						</div>
					</div>
				</section>

				<DialogoRenombrar
					deck={renombrar}
					onCerrar={() => setRenombrar(null)}
					onHecho={intentar(async (id: string, nombre: string) => {
						await call("deck_rename", { id, name: nombre });
						if (id === useDeck.getState().deck?.id)
							useDeck.getState().setSinHistorial({ name: nombre });
						await refrescar();
					})}
				/>
				<PropiedadesDialog
					id={propiedades}
					onCerrar={() => setPropiedades(null)}
					onNuevaEtiqueta={(para) => setEditandoEtiqueta({ tipo: "nueva", para })}
				/>
				<EtiquetaDialog editando={editandoEtiqueta} onCerrar={() => setEditandoEtiqueta(null)} />
				<ConfirmarBorrar
					ids={borrar}
					onCerrar={() => setBorrar(null)}
					onConfirmar={intentar(async () => {
						if (!borrar) return;
						const abierto = useDeck.getState().deck?.id;
						for (const id of borrar) await call("deck_delete", { id });
						if (abierto && borrar.includes(abierto)) {
							const d = await call("deck_create", { name: t("Deck sin título") });
							await abrirDeck(d.id);
						}
						setSeleccion((s) => s.filter((x) => !borrar.includes(x)));
						await recargar();
					})}
				/>
				<Confirmar
					abierto={borrarEtiqueta !== null}
					onCerrar={() => setBorrarEtiqueta(null)}
					titulo={t("¿Eliminar la etiqueta «{n}»?", { n: borrarEtiqueta?.name ?? "" })}
					descripcion={t("Se quita de los decks que la llevan; los decks no se borran.")}
					accion={t("Eliminar")}
					destructiva
					onConfirmar={intentar(async () => {
						if (!borrarEtiqueta) return;
						const id = borrarEtiqueta.id;
						await useBiblioteca.getState().borrarEtiqueta(id);
						const f = useFiltrosBiblioteca.getState().filtros;
						if (f.etiquetas.includes(id))
							ponerFiltros({ etiquetas: f.etiquetas.filter((x) => x !== id) });
					})}
				/>
			</SeleccionDecks.Provider>
		</AccionesDeck.Provider>
	);
}

/** El título de un grupo según de qué es. */
function TituloGrupo({ clave: c, contando }: { clave: ClaveGrupo; contando: boolean }) {
	const { t } = useLang();
	switch (c.tipo) {
		case "todos":
			return <span>{t("Todos los decks")}</span>;
		case "etiqueta":
			return <ChipEtiqueta tag={c.tag} className="text-[11px]" />;
		case "sinEtiqueta":
			return <span className="text-muted-foreground">{t("Sin etiqueta")}</span>;
		case "color":
			return (
				<span className="flex items-center gap-1.5">
					<Identidad colores={c.colores} />
					{t(nombreColores(c.colores))}
				</span>
			);
		case "clase":
			return <span>{t(CLASES[c.clase])}</span>;
		case "idioma":
			return c.idioma ? (
				<span>{t(IDIOMAS[c.idioma] ?? c.idioma.toUpperCase())}</span>
			) : (
				<span className="text-muted-foreground">{t("Sin idioma")}</span>
			);
		case "sinDatos":
			return (
				<span className="text-muted-foreground">
					{contando ? t("Calculando…") : t("Sin datos")}
				</span>
			);
	}
}

/** La franja de arriba cuando hay decks elegidos. */
function BarraSeleccion({
	seleccion,
	visibles,
	onTodos,
	onSoltar,
	onBorrar,
}: {
	seleccion: string[];
	visibles: string[];
	onTodos: () => void;
	onSoltar: () => void;
	onBorrar: () => void;
}) {
	const { t } = useLang();
	const decks = useBiblioteca((s) => s.decks) ?? [];
	const elegidos = decks.filter((d) => seleccion.includes(d.id));
	const primero = elegidos[0];
	const todosFavoritos = elegidos.length > 0 && elegidos.every((d) => d.pinned);
	const faltan = visibles.some((id) => !seleccion.includes(id));
	return (
		<div className="absolute bottom-9 left-1/2 z-10 flex -translate-x-1/2 items-center gap-1.5 rounded-xl border bg-popover py-1.5 pr-1.5 pl-3 text-xs whitespace-nowrap shadow-lg">
			<span className="font-medium">
				{seleccion.length === 1
					? t("1 deck elegido")
					: t("{n} decks elegidos", { n: seleccion.length })}
			</span>
			{faltan && (
				<Button size="xs" variant="ghost" onClick={onTodos}>
					{t("Elegir todos")}
				</Button>
			)}
			<div className="ml-2 flex items-center gap-1.5">
				<Button
					size="xs"
					variant="ghost"
					onClick={() => void useBiblioteca.getState().favorito(seleccion, !todosFavoritos)}
				>
					<Star className={cn(todosFavoritos && "fill-amber-400 text-amber-500")} />
					{todosFavoritos ? t("Quitar de favoritos") : t("Añadir a favoritos")}
				</Button>
				{primero && (
					<DropdownMenu>
						<DropdownMenuTrigger asChild>
							<Button size="xs" variant="outline">
								<MoreHorizontal />
								{t("Acciones")}
							</Button>
						</DropdownMenuTrigger>
						<MenuDeck deck={primero} />
					</DropdownMenu>
				)}
				<Button size="xs" variant="ghost" className="text-destructive" onClick={onBorrar}>
					<Trash2 />
					{t("Eliminar")}
				</Button>
				<Button
					size="icon-xs"
					variant="ghost"
					onClick={onSoltar}
					aria-label={t("Soltar la selección")}
				>
					<X />
				</Button>
			</div>
		</div>
	);
}

/** Abajo: cuántos se ven, cuántas cartas suman y lo que costarían. */
function Pie({ visibles, total }: { visibles: DeckSummary[]; total: number }) {
	const { t } = useLang();
	const detalles = useBiblioteca((s) => s.detalles);
	const contando = useBiblioteca((s) => s.contando);
	const dinero = useDinero();
	let cartas = 0;
	let eur = 0;
	let usd = 0;
	let conEur = false;
	for (const d of visibles) {
		cartas += d.cardCount;
		const x = detalles[d.id];
		if (x?.priceEur != null) {
			eur += x.priceEur;
			conEur = true;
		}
		if (x?.priceUsd != null) usd += x.priceUsd;
	}
	const precio = conEur ? dinero(eur, "EUR") : usd > 0 ? dinero(usd, "USD") : null;
	const partes: [string, ReactNode][] = [
		[
			"decks",
			visibles.length === total
				? t("{n} decks", { n: total })
				: t("Mostrando {a} de {b} decks", { a: visibles.length, b: total }),
		],
		["cartas", t("{n} cartas", { n: cartas })],
	];
	if (precio) partes.push(["precio", `≈ ${precio}`]);
	return (
		<footer className="flex items-center gap-2 border-t px-3 py-1 text-[10px] text-muted-foreground">
			{partes.map(([k, p], i) => (
				<span key={k} className="flex items-center gap-2">
					{i > 0 && <span aria-hidden>·</span>}
					{p}
				</span>
			))}
			{contando && (
				<span className="ml-auto animate-pulse">{t("Calculando colores y precios…")}</span>
			)}
		</footer>
	);
}

function ConfirmarBorrar({
	ids,
	onCerrar,
	onConfirmar,
}: {
	ids: string[] | null;
	onCerrar: () => void;
	onConfirmar: () => void;
}) {
	const { t } = useLang();
	const decks = useBiblioteca((s) => s.decks) ?? [];
	const uno = ids?.length === 1 ? decks.find((d) => d.id === ids[0]) : undefined;
	return (
		<Confirmar
			abierto={ids !== null}
			onCerrar={onCerrar}
			titulo={
				uno
					? t("¿Eliminar «{n}»?", { n: uno.name })
					: t("¿Eliminar {n} decks?", { n: ids?.length ?? 0 })
			}
			descripcion={
				uno
					? t("El deck se borra de la biblioteca. Esto no se puede deshacer.")
					: t("Los decks se borran de la biblioteca. Esto no se puede deshacer.")
			}
			accion={t("Eliminar")}
			destructiva
			onConfirmar={onConfirmar}
		/>
	);
}

function DialogoRenombrar({
	deck,
	onCerrar,
	onHecho,
}: {
	deck: DeckSummary | null;
	onCerrar: () => void;
	onHecho: (id: string, nombre: string) => void;
}) {
	const { t } = useLang();
	const [nombre, setNombre] = useState("");
	useEffect(() => setNombre(deck?.name ?? ""), [deck]);
	const hecho = () => {
		if (!deck || !nombre.trim()) return;
		onHecho(deck.id, nombre.trim());
		onCerrar();
	};
	return (
		<Dialog open={deck !== null} onOpenChange={(o) => !o && onCerrar()}>
			<DialogContent className="sm:max-w-sm" cambiosSinGuardar={false}>
				<DialogHeader>
					<DialogTitle>{t("Renombrar deck")}</DialogTitle>
				</DialogHeader>
				<Input
					autoFocus
					value={nombre}
					onChange={(e) => setNombre(e.target.value)}
					onKeyDown={(e) => e.key === "Enter" && hecho()}
					className="text-sm"
				/>
				<DialogFooter>
					<Button size="sm" variant="outline" onClick={onCerrar}>
						{t("Cancelar")}
					</Button>
					<Button size="sm" onClick={hecho} disabled={!nombre.trim()}>
						{t("Guardar")}
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	);
}
