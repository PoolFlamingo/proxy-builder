import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useDefaultLayout } from "react-resizable-panels";
import { toast } from "sonner";
import type { ArtFraming } from "@/bindings/ArtFraming";
import type { Collection } from "@/bindings/Collection";
import type { DesignLang } from "@/bindings/DesignLang";
import type { PackInfo } from "@/bindings/PackInfo";
import { Confirmar } from "@/components/confirmar";
import { Icono } from "@/components/icono";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from "@/components/ui/resizable";
import { WithTooltip } from "@/components/ui/with-tooltip";
import { nuevoId } from "@/lib/deck-ops";
import { call } from "@/lib/ipc";
import { useDeck } from "@/stores/deck-store";
import { useEdicion } from "@/stores/edicion-store";
import { useUi } from "@/stores/ui-store";
import { Formulario } from "./formulario";
import { agrupar, conVariante, vecina } from "./grupos";
import { ListaCartas } from "./lista-cartas";
import { useMarcosOriginales } from "./marcos-originales";
import { usePacksDescargables } from "./packs-descargables";
import { ponerIdiomaDiseno, renumerar } from "./renumerar";
import { useBorrador } from "./use-borrador";
import { useResumen } from "./use-resumen";
import { VistaPrevia } from "./vista-previa";

/**
 * **Diseñar**: la tercera vista. A la izquierda las cartas del deck y su
 * estado; en el centro la carta tal como saldrá (pintada por Rust); a la
 * derecha el formulario. `Alt+←/→` pasa de carta (y, si se elige, de copia:
 * cada copia de una carta puede llevar su propio diseño).
 */
export function DisenarScreen() {
	const { t, mensaje } = useLang();
	const cartas = useDeck((s) => s.deck?.cards);
	const lang = useDeck((s) => s.deck?.settings.designLang ?? "es");
	// El idioma de la vista es el del deck: lo que se ve aquí es lo que sale
	// en el PDF.
	// Sube al acabar de cambiar de idioma: si se bajaron letras (japonés,
	// coreano, chino), la vista previa se vuelve a pedir.
	const [versionLetras, setVersionLetras] = useState(0);
	const setLang = (l: DesignLang) =>
		void ponerIdiomaDiseno(l, t, mensaje).then(() => setVersionLetras((v) => v + 1));
	const disenando = useUi((s) => s.disenando);
	const layout = useDefaultLayout({ id: "proxybuilder.paneles.disenar", storage: localStorage });
	const grupos = useMemo(() => agrupar(cartas ?? []), [cartas]);
	const grupo = useMemo(
		() => grupos.find((g) => g.cartas.some((c) => c.id === disenando)) ?? null,
		[grupos, disenando],
	);
	const [cara, setCara] = useState(0);
	const [soloEsta, setSoloEsta] = useState(false);
	const [colecciones, setColecciones] = useState<Collection[]>([]);
	const [packs, setPacks] = useState<PackInfo[]>([]);
	const [simbolos, setSimbolos] = useState<number | null>(null);
	const recursos = useMemo(
		() => ({ colecciones, packs, simbolos, versionLetras }),
		[colecciones, packs, simbolos, versionLetras],
	);
	const porCopias = useUi((s) => s.disenarCopias);
	// La copia que se diseña es la carta que dice `disenando`.
	const copia = Math.max(0, grupo?.cartas.findIndex((c) => c.id === disenando) ?? 0);
	const b = useBorrador(grupo, cara, lang, copia, recursos);
	const resumen = useResumen(b.design, cara, lang);
	const dialogo = useUi((s) => s.dialogo);

	// Colecciones y packs (se releen al cerrar sus diálogos y al descargar los
	// marcos originales).
	const versionOriginales = useMarcosOriginales((s) => s.version);
	const versionDescargables = usePacksDescargables((s) => s.version);
	useEffect(() => {
		void versionDescargables;
		call("collection_list", {})
			.then(setColecciones)
			.catch(() => {});
		call("packs_list", {})
			.then(setPacks)
			.catch(() => {});
	}, [dialogo, versionOriginales, versionDescargables]);

	// La primera vez, los símbolos de maná (se descargan si no están).
	useEffect(() => {
		call("symbols_status", { refresh: false })
			.then((s) => {
				// Recién descargados: la vista se pinta otra vez con ellos.
				setSimbolos(s.fecha);
				if (s.cargados === 0)
					toast.warning(t("Sin símbolos de maná: hace falta conexión la primera vez."));
			})
			.catch(() => {});
	}, [t]);

	// Al cambiar de carta, cara 0.
	useEffect(() => {
		setCara(0);
		// Con las copias por separado, lo que se guarda es solo la copia de ahora.
		setSoloEsta(useUi.getState().disenarCopias);
	}, [grupo?.clave]);

	const elegibles = useMemo(() => grupos.filter((g) => g.scryfallId), [grupos]);
	// Sin carta elegida (se entra por la barra), la primera que se puede diseñar.
	useEffect(() => {
		if (!grupo && elegibles[0]?.cartas[0]) useUi.getState().disenar(elegibles[0].cartas[0].id);
	}, [grupo, elegibles]);
	const ir = useCallback(
		(id: string) => {
			// Dentro de la misma carta es otra copia: se guarda solo esa.
			if (grupo?.cartas.some((c) => c.id === id)) setSoloEsta(true);
			useUi.getState().disenar(id);
		},
		[grupo],
	);
	// Pasar a la carta de al lado sin más: solo tras guardar («Guardar y
	// siguiente»).
	const pasar = useCallback(
		(dir: 1 | -1) => {
			const id = vecina(elegibles, disenando, dir, porCopias);
			if (id) ir(id);
		},
		[elegibles, disenando, porCopias, ir],
	);

	// Lo que se hará, tras guardar o descartar, si se sale de una carta con
	// cambios sin guardar.
	const [pendiente, setPendiente] = useState<(() => void) | null>(null);
	const saltar = (id: string) => {
		if (b.cambiado) return setPendiente(() => () => ir(id));
		ir(id);
	};
	// Las flechas (la barra y Alt+←/→) también preguntan.
	const pasarConAviso = useCallback(
		(dir: 1 | -1) => {
			const id = vecina(elegibles, disenando, dir, porCopias);
			if (!id) return;
			if (b.cambiado) setPendiente(() => () => ir(id));
			else ir(id);
		},
		[elegibles, disenando, porCopias, ir, b.cambiado],
	);
	useEffect(() => {
		const tecla = (e: KeyboardEvent) => {
			if (!e.altKey) return;
			if (e.key === "ArrowRight") pasarConAviso(1);
			if (e.key === "ArrowLeft") pasarConAviso(-1);
		};
		window.addEventListener("keydown", tecla);
		return () => window.removeEventListener("keydown", tecla);
	}, [pasarConAviso]);

	// Guarda la carta (y las copias que toque); `false` si no se pudo.
	const guardarCarta = async () => {
		const ok = await b.guardar(!soloEsta);
		if (ok) useUi.getState().anunciar(t("Diseño guardado"));
		return ok;
	};
	const guardar = async (siguiente: boolean) => {
		if ((await guardarCarta()) && siguiente) pasar(1);
	};
	// Con cambios sin guardar, las demás pantallas (cambiar de vista, cerrar la
	// app) saben guardar esta carta por aquí.
	const guardarAhora = useRef<() => Promise<boolean>>(async () => false);
	guardarAhora.current = guardarCarta;
	const hayCambios = b.cambiado;
	useEffect(() => {
		if (!hayCambios) return;
		useEdicion.getState().poner({ guardar: () => guardarAhora.current() });
		return () => useEdicion.getState().poner(null);
	}, [hayCambios]);

	// La segunda parte de una aventura o una flip enseña la ilustración de la
	// primera: moverla es mover esa.
	const caraArte = resumen?.artePropio === false ? 0 : cara;
	const encuadre = (framing: ArtFraming) =>
		b.cambiar((d) => {
			const arte = d.faces[caraArte]?.art;
			if (!arte) return d;
			return {
				...d,
				faces: d.faces.map((x, i) => (i === caraArte ? { ...x, art: { ...arte, framing } } : x)),
			};
		});

	// Otra copia de la carta con el diseño de esta, marcada como variante de
	// diseño (se imprime, pero no cuenta para el mazo), y a diseñarla.
	const crearVariante = async () => {
		const actual = grupo?.cartas[copia];
		if (!actual) return;
		// Lo que se lleva hecho se guarda antes, para que la variante parta de ello.
		if (b.cambiado && !(await b.guardar(false))) return;
		const id = nuevoId();
		useDeck.getState().setCartas((cs) => conVariante(cs, actual.id, id));
		setSoloEsta(true);
		useUi.getState().disenar(id);
		useUi
			.getState()
			.anunciar(t("Variante creada (copia {n})", { n: (grupo?.cartas.length ?? 0) + 1 }));
	};

	const copias = grupo?.cartas.length ?? 0;
	const tieneDiseno = !!grupo?.cartas[copia]?.design;

	return (
		<div className="flex h-full flex-col">
			<div
				className="flex shrink-0 items-center gap-2 border-b px-3 py-1.5"
				data-tutorial="disenar-barra"
			>
				<Button
					size="sm"
					variant="ghost"
					onClick={() => pasarConAviso(-1)}
					aria-label={t("Carta anterior")}
				>
					<Icono icon="lucide:chevron-left" />
				</Button>
				<Button
					size="sm"
					variant="ghost"
					onClick={() => pasarConAviso(1)}
					aria-label={t("Carta siguiente")}
				>
					<Icono icon="lucide:chevron-right" />
				</Button>
				<span className="text-[11px] text-muted-foreground">Alt+← / Alt+→</span>
				<div className="ml-auto flex items-center gap-1.5">
					<WithTooltip
						content={t(
							"Vuelve a numerar las cartas por el orden del deck (las escritas a mano no cambian).",
						)}
					>
						<Button
							size="sm"
							variant="ghost"
							onClick={() => renumerar(t, mensaje)}
							disabled={!cartas?.some((c) => c.design)}
						>
							<Icono icon="lucide:hash" />
							{t("Renumerar")}
						</Button>
					</WithTooltip>
					<Button
						size="sm"
						variant="ghost"
						onClick={() => useUi.getState().abrir({ tipo: "disenarDeck" })}
					>
						<Icono icon="lucide:wand-2" />
						{t("Diseñar el deck")}
					</Button>
					<Button
						size="sm"
						variant="ghost"
						onClick={() => useUi.getState().abrir({ tipo: "exportarPng" })}
						disabled={!cartas?.some((c) => c.design)}
					>
						<Icono icon="lucide:image-down" />
						{t("Exportar imágenes")}
					</Button>
					<Button
						size="sm"
						variant="ghost"
						onClick={() => useUi.getState().abrir({ tipo: "colecciones" })}
					>
						{t("Colecciones")}
					</Button>
					<Button
						size="sm"
						variant="ghost"
						onClick={() => useUi.getState().abrir({ tipo: "recursosDiseno" })}
					>
						<Icono icon="lucide:sliders-horizontal" />
						{t("Letras, marcos y símbolos")}
					</Button>
				</div>
			</div>
			<ResizablePanelGroup
				orientation="horizontal"
				defaultLayout={layout.defaultLayout}
				onLayoutChanged={layout.onLayoutChanged}
				className="min-h-0 flex-1"
			>
				<ResizablePanel id="lista" defaultSize="22" minSize={220}>
					<ListaCartas
						grupos={grupos}
						elegido={grupo?.clave ?? null}
						onElegir={(g) => g.cartas[0] && saltar(g.cartas[0].id)}
					/>
				</ResizablePanel>
				<ResizableHandle withHandle />
				<ResizablePanel id="vista" minSize={320}>
					<VistaPrevia
						design={b.design}
						huella={b.huella}
						alDia={b.alDia}
						recursos={recursos}
						avisos={b.avisos}
						lang={lang}
						setLang={setLang}
						cara={cara}
						setCara={setCara}
						caraArte={caraArte}
						onEncuadre={encuadre}
						cargando={b.cargando}
						carasImpresas={resumen?.carasImpresas ?? 1}
					/>
				</ResizablePanel>
				<ResizableHandle withHandle />
				<ResizablePanel id="formulario" defaultSize="30" minSize={300}>
					{b.design ? (
						<div className="flex h-full flex-col">
							<div className="min-h-0 flex-1">
								<Formulario
									design={b.design}
									cambiar={b.cambiar}
									cara={cara}
									setCara={setCara}
									colecciones={colecciones}
									packs={packs}
									onColeccion={(c) =>
										setColecciones((cs) => cs.map((x) => (x.id === c.id ? c : x)))
									}
									resumen={resumen}
								/>
							</div>
							<div
								className="flex shrink-0 flex-col gap-2 border-t p-2.5"
								data-tutorial="disenar-pie"
							>
								{grupo && (
									<div className="flex flex-wrap items-center gap-1.5">
										{copias > 1 && (
											<>
												<span className="text-xs text-muted-foreground">{t("Copia:")}</span>
												{grupo.cartas.map((c, i) => (
													<Button
														key={c.id}
														size="sm"
														variant={i === copia ? "secondary" : "ghost"}
														className="h-6 min-w-6 px-1.5"
														onClick={() => i !== copia && saltar(c.id)}
														aria-label={t("Copia {n}", { n: i + 1 })}
														aria-pressed={i === copia}
													>
														{i + 1}
													</Button>
												))}
											</>
										)}
										<WithTooltip
											content={t(
												"Añade otra copia de esta carta con una copia de este diseño, marcada como variante de diseño: se imprime pero no cuenta para el mazo. Antes se guarda lo que llevas hecho.",
											)}
										>
											<Button
												size="sm"
												variant="outline"
												className="ml-auto"
												onClick={() => void crearVariante()}
												disabled={b.guardando}
											>
												<Icono icon="lucide:copy-plus" />
												{t("Crear variante")}
											</Button>
										</WithTooltip>
									</div>
								)}
								{copias > 1 && (
									<>
										<label className="flex items-center gap-2 text-xs">
											<Checkbox
												checked={soloEsta}
												onCheckedChange={(v) => setSoloEsta(v === true)}
											/>
											{t("Solo esta copia (hay {n})", { n: copias })}
										</label>
										<label className="flex items-center gap-2 text-xs">
											<Checkbox
												checked={porCopias}
												onCheckedChange={(v) => {
													useUi.getState().setPreferencia("disenarCopias", v === true);
													setSoloEsta(v === true);
												}}
											/>
											{t("Recorrer también las copias con las flechas")}
										</label>
									</>
								)}
								<div className="flex items-center gap-1.5">
									{tieneDiseno && (
										<WithTooltip content={t("Quita el diseño y deja la carta como estaba.")}>
											<Button size="sm" variant="ghost" onClick={() => b.quitar(!soloEsta)}>
												<Icono icon="lucide:trash-2" />
												{t("Quitar diseño")}
											</Button>
										</WithTooltip>
									)}
									<WithTooltip content={t("Descartar los cambios")}>
										<Button
											size="sm"
											variant="ghost"
											onClick={b.descartar}
											disabled={!b.cambiado}
											aria-label={t("Descartar los cambios")}
										>
											<Icono icon="lucide:undo-2" />
										</Button>
									</WithTooltip>
									<div className="ml-auto flex gap-1.5">
										<Button
											size="sm"
											variant="outline"
											onClick={() => guardar(false)}
											disabled={b.guardando}
										>
											{b.guardando ? (
												<Icono icon="lucide:loader-2" className="animate-spin" />
											) : (
												<Icono icon="lucide:save" />
											)}
											{t("Guardar")}
										</Button>
										<Button size="sm" onClick={() => guardar(true)} disabled={b.guardando}>
											{t("Guardar y siguiente")}
										</Button>
									</div>
								</div>
							</div>
						</div>
					) : (
						<div className="flex h-full items-center justify-center p-6 text-center text-xs text-muted-foreground">
							{b.cargando ? (
								<Icono icon="lucide:loader-2" className="size-5 animate-spin" />
							) : (
								t("Elige una carta de la lista para diseñarla.")
							)}
						</div>
					)}
				</ResizablePanel>
			</ResizablePanelGroup>
			<Confirmar
				abierto={!!pendiente}
				onCerrar={() => setPendiente(null)}
				titulo={t("¿Descartar los cambios?")}
				descripcion={t("Hay cambios sin guardar en esta carta.")}
				accion={t("Descartar")}
				destructiva
				onConfirmar={() => {
					pendiente?.();
				}}
				accionAlternativa={{
					texto: t("Guardar y continuar"),
					onClick: async () => {
						const ok = await guardarAhora.current();
						if (ok) pendiente?.();
						return ok;
					},
				}}
			/>
		</div>
	);
}
