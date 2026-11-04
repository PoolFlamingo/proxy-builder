import { useCallback, useEffect, useRef, useState } from "react";
import { toast } from "sonner";
import type { DeckSummary } from "@/bindings/DeckSummary";
import { Confirmar } from "@/components/confirmar";
import { DialogoAdaptable } from "@/components/dialogo-adaptable";
import { FilaMenu } from "@/components/hoja-menu";
import { Icono } from "@/components/icono";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import { Dialog, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { megas, nombreSeguro } from "@/features/export/acciones";
import { usePasarUi } from "@/features/pasar/store";
import { abrirParaImportar, guardarParaExportar } from "@/lib/dialogos";
import { useFormato } from "@/lib/formato";
import { call } from "@/lib/ipc";
import { esMovil } from "@/lib/plataforma";
import { useNombreDeck } from "@/lib/use-nombre-deck";
import { cn } from "@/lib/utils";
import { abrirDeck, guardarAhora, useDeck } from "@/stores/deck-store";
import { useUi } from "@/stores/ui-store";
import { BarraHerramientas } from "./barra-herramientas";
import { BarraLateral } from "./barra-lateral";
import { useEtiquetasUi } from "./etiquetas-ui";
import { DialogoLiberar } from "./liberar-dialog";
import { type Acciones, AccionesDeck, BotonMenuDeck } from "./menu-deck";
import { alPulsarDeck, entrarEnSeleccion, seleccionar } from "./organizar";
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
	const movil = useFormato().tamano === "movil";
	const actual = useDeck((s) => s.deck?.id ?? null);
	const decks = useBiblioteca((s) => s.decks);
	const prefs = usePrefs();
	const { filtros, ponerFiltros } = useFiltros();
	const org = useOrganizados();
	const { enPantalla } = org;
	const etiquetasUi = useEtiquetasUi();
	const nombreDeck = useNombreDeck();

	const [seleccion, setSeleccion] = useState<string[]>([]);
	const ancla = useRef<string | null>(null);
	const [renombrar, setRenombrar] = useState<DeckSummary | null>(null);
	const [borrar, setBorrar] = useState<string[] | null>(null);
	const [propiedades, setPropiedades] = useState<string | null>(null);
	const [liberando, setLiberando] = useState(false);
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
		await call("deck_duplicate", { id: d.id, name: t("{n} (copia)", { n: nombreDeck(d.name) }) });
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
			// En móvil no hay carpeta que enseñar (es un `content://`): se comparte.
			action: (await esMovil())
				? {
						label: t("Compartir"),
						onClick: () => void call("compartir_fichero", { ruta: destino }),
					}
				: {
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
		// Enseña lo que trae y pregunta (con proxies o solo originales, y si ya
		// lo tienes): el mismo diálogo que al recibir de otro equipo.
		usePasarUi.getState().abrirImportar(ruta);
	});

	const liberar = () => setLiberando(true);

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
			const que = alPulsarDeck(seleccion.length, e);
			if (que === "tramo") {
				setSeleccion((s) => seleccionar(s, enPantalla, id, ancla.current, "tramo"));
			} else if (que === "alternar") {
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
		// Con el dedo: mantener pulsado un deck entra en el modo selección con él
		// elegido; después los toques suman o quitan (`pulsar`, con algo elegido).
		entrarSeleccion: (id) => {
			setSeleccion((s) => entrarEnSeleccion(s, id));
			ancla.current = id;
		},
		marcar: (ids, si) =>
			setSeleccion((s) => (si ? [...new Set([...s, ...ids])] : s.filter((x) => !ids.includes(x)))),
		abrir,
	};

	// Teclado: Ctrl+F busca, Ctrl+A elige todo lo visible, Esc suelta, Supr borra.
	useEffect(() => {
		const tecla = (e: KeyboardEvent) => {
			// El «atrás» de Android manda un Escape al `document` (`lib/atras.ts`): no es un elemento.
			const el = e.target instanceof Element ? e.target : null;
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
							movil ? (
								<Button size="sm" onClick={nuevo}>
									<Icono icon="lucide:plus" />
									{t("Nuevo deck")}
								</Button>
							) : (
								<>
									<Button size="sm" variant="ghost" onClick={liberar}>
										<Icono icon="lucide:hard-drive-download" />
										{t("Liberar espacio")}
									</Button>
									<Button size="sm" variant="outline" onClick={importarPaquete}>
										<Icono icon="lucide:import" />
										{t("Importar .pbdeck")}
									</Button>
									<Button size="sm" onClick={nuevo}>
										<Icono icon="lucide:plus" />
										{t("Nuevo deck")}
									</Button>
								</>
							)
						}
						menu={
							<>
								<FilaMenu
									icono="lucide:import"
									texto={t("Importar .pbdeck")}
									onClick={importarPaquete}
								/>
								<FilaMenu
									icono="lucide:hard-drive-download"
									texto={t("Liberar espacio")}
									onClick={liberar}
								/>
							</>
						}
					/>
					<div className="flex min-h-0 flex-1">
						{/* Móvil: la barra lateral es una hoja (se abre con «Etiquetas»). */}
						{(prefs.lateral || movil) && (
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
				<DialogoLiberar abierto={liberando} onCerrar={() => setLiberando(false)} />
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
	const movil = useFormato().tamano === "movil";
	const favorito = todosFavoritos ? t("Quitar de favoritos") : t("Añadir a favoritos");
	return (
		<div
			className={cn(
				"absolute z-10 flex rounded-xl border bg-popover text-xs shadow-lg",
				movil
					? "inset-x-2 bottom-9 flex-col gap-1 p-1.5 pl-3"
					: "bottom-9 left-1/2 -translate-x-1/2 items-center gap-1.5 py-1.5 pr-1.5 pl-3 whitespace-nowrap",
			)}
		>
			<div className="flex items-center gap-1.5">
				<span className="font-medium">
					{seleccion.length === 1
						? t("1 deck elegido")
						: t("{n} decks elegidos", { n: seleccion.length })}
				</span>
				{faltan && (
					<Button size="xs" variant="ghost" className={cn(movil && "ml-auto")} onClick={onTodos}>
						{t("Elegir todos")}
					</Button>
				)}
			</div>
			<div className={cn("flex items-center gap-1.5", movil ? "justify-around" : "ml-2")}>
				<Button
					size={movil ? "icon-xs" : "xs"}
					variant="ghost"
					aria-label={favorito}
					onClick={() => void useBiblioteca.getState().favorito(seleccion, !todosFavoritos)}
				>
					<Icono
						icon="lucide:star"
						className={cn(todosFavoritos && "fill-amber-400 text-amber-500")}
					/>
					{!movil && favorito}
				</Button>
				{primero && (
					<BotonMenuDeck deck={primero}>
						<Button size={movil ? "icon-xs" : "xs"} variant="outline" aria-label={t("Acciones")}>
							<Icono icon="lucide:more-horizontal" />
							{!movil && t("Acciones")}
						</Button>
					</BotonMenuDeck>
				)}
				<Button
					size={movil ? "icon-xs" : "xs"}
					variant="ghost"
					className="text-destructive"
					aria-label={t("Eliminar")}
					onClick={onBorrar}
				>
					<Icono icon="lucide:trash-2" />
					{!movil && t("Eliminar")}
				</Button>
				<Button
					size="icon-xs"
					variant="ghost"
					onClick={onSoltar}
					aria-label={t("Soltar la selección")}
				>
					<Icono icon="lucide:x" />
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
	const nombreDeck = useNombreDeck();
	const uno = ids?.length === 1 ? decks.find((d) => d.id === ids[0]) : undefined;
	return (
		<Confirmar
			abierto={ids !== null}
			onCerrar={onCerrar}
			titulo={
				uno
					? t("¿Eliminar «{n}»?", { n: nombreDeck(uno.name) })
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
	const nombreDeck = useNombreDeck();
	const [nombre, setNombre] = useState("");
	useEffect(() => setNombre(nombreDeck(deck?.name ?? "")), [deck, nombreDeck]);
	const hecho = () => {
		if (!deck || !nombre.trim()) return;
		onHecho(deck.id, nombre.trim());
		onCerrar();
	};
	return (
		<Dialog open={deck !== null} onOpenChange={(o) => !o && onCerrar()}>
			<DialogoAdaptable compacto className="sm:max-w-sm" cambiosSinGuardar={false}>
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
			</DialogoAdaptable>
		</Dialog>
	);
}
