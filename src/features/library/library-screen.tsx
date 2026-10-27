import { HardDriveDownload, Import, MoreHorizontal, Plus, Star, Trash2, X } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { toast } from "sonner";
import type { DeckSummary } from "@/bindings/DeckSummary";
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
import { Input } from "@/components/ui/input";
import { megas, nombreSeguro } from "@/features/export/acciones";
import { abrirParaImportar, guardarParaExportar } from "@/lib/dialogos";
import { call } from "@/lib/ipc";
import { cn } from "@/lib/utils";
import { abrirDeck, guardarAhora, useDeck } from "@/stores/deck-store";
import { useUi } from "@/stores/ui-store";
import { BarraHerramientas } from "./barra-herramientas";
import { BarraLateral } from "./barra-lateral";
import { useEtiquetasUi } from "./etiquetas-ui";
import { type Acciones, AccionesDeck, MenuDeck } from "./menu-deck";
import { seleccionar } from "./organizar";
import { useFiltros, usePrefs } from "./preferencias";
import { PropiedadesDialog } from "./propiedades-dialog";
import { useBiblioteca } from "./store";
import { type Seleccion, SeleccionDecks } from "./vista-cuadricula";
import { useOrganizados, ZonaDecks } from "./zona-decks";

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
	const prefs = usePrefs();
	const { filtros, ponerFiltros } = useFiltros();
	const org = useOrganizados();
	const { enPantalla } = org;
	const etiquetasUi = useEtiquetasUi();

	const [seleccion, setSeleccion] = useState<string[]>([]);
	const ancla = useRef<string | null>(null);
	const [renombrar, setRenombrar] = useState<DeckSummary | null>(null);
	const [borrar, setBorrar] = useState<string[] | null>(null);
	const [propiedades, setPropiedades] = useState<string | null>(null);
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
		nuevaEtiqueta: (ids) => etiquetasUi.editar({ tipo: "nueva", para: ids }),
		seleccion,
	};

	const sel: Seleccion = {
		seleccion,
		casillas: true,
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

	return (
		<AccionesDeck.Provider value={acciones}>
			<SeleccionDecks.Provider value={sel}>
				<section className="flex h-full min-h-0 flex-col">
					<BarraHerramientas
						ref={buscar}
						titulo={t("Biblioteca")}
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
								onNuevaEtiqueta={() => etiquetasUi.editar({ tipo: "nueva", para: seleccion })}
								onEditarEtiqueta={(tag) => etiquetasUi.editar({ tipo: "editar", tag })}
								onBorrarEtiqueta={etiquetasUi.borrar}
							/>
						)}
						<ZonaDecks
							org={org}
							flotante={
								seleccion.length > 0 && (
									<BarraSeleccion
										seleccion={seleccion}
										visibles={enPantalla}
										onTodos={() => setSeleccion(enPantalla)}
										onSoltar={() => setSeleccion([])}
										onBorrar={() => setBorrar(seleccion)}
									/>
								)
							}
						/>
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
					onNuevaEtiqueta={(para) => etiquetasUi.editar({ tipo: "nueva", para })}
				/>
				{etiquetasUi.dialogos}
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
			</SeleccionDecks.Provider>
		</AccionesDeck.Provider>
	);
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
