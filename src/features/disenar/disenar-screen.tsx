import {
	ChevronLeft,
	ChevronRight,
	Hash,
	ImageDown,
	Loader2,
	Save,
	SlidersHorizontal,
	Trash2,
	Undo2,
	Wand2,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { useDefaultLayout } from "react-resizable-panels";
import { toast } from "sonner";
import type { ArtFraming } from "@/bindings/ArtFraming";
import type { Collection } from "@/bindings/Collection";
import type { DesignLang } from "@/bindings/DesignLang";
import type { PackInfo } from "@/bindings/PackInfo";
import { Confirmar } from "@/components/confirmar";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from "@/components/ui/resizable";
import { WithTooltip } from "@/components/ui/with-tooltip";
import { call } from "@/lib/ipc";
import { useDeck } from "@/stores/deck-store";
import { useUi } from "@/stores/ui-store";
import { Formulario } from "./formulario";
import { agrupar, type Grupo } from "./grupos";
import { ListaCartas } from "./lista-cartas";
import { useMarcosOriginales } from "./marcos-originales";
import { usePacksDescargables } from "./packs-descargables";
import { renumerar } from "./renumerar";
import { useBorrador } from "./use-borrador";
import { useResumen } from "./use-resumen";
import { VistaPrevia } from "./vista-previa";

/**
 * **Diseñar**: la tercera vista. A la izquierda las cartas del deck y su
 * estado; en el centro la carta tal como saldrá (pintada por Rust); a la
 * derecha el formulario. `Alt+←/→` pasa de carta.
 */
export function DisenarScreen() {
	const { t, mensaje } = useLang();
	const cartas = useDeck((s) => s.deck?.cards);
	const langDeck = useDeck((s) => s.deck?.settings.designLang ?? "es");
	const disenando = useUi((s) => s.disenando);
	const layout = useDefaultLayout({ id: "proxybuilder.paneles.disenar", storage: localStorage });
	const grupos = useMemo(() => agrupar(cartas ?? []), [cartas]);
	const grupo = useMemo(
		() => grupos.find((g) => g.cartas.some((c) => c.id === disenando)) ?? null,
		[grupos, disenando],
	);
	const [lang, setLang] = useState<DesignLang>(langDeck);
	const [cara, setCara] = useState(0);
	const [soloEsta, setSoloEsta] = useState(false);
	const [colecciones, setColecciones] = useState<Collection[]>([]);
	const [packs, setPacks] = useState<PackInfo[]>([]);
	const [simbolos, setSimbolos] = useState<number | null>(null);
	const recursos = useMemo(
		() => ({ colecciones, packs, simbolos }),
		[colecciones, packs, simbolos],
	);
	const b = useBorrador(grupo, cara, lang, recursos);
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
		setSoloEsta(false);
	}, [grupo?.clave]);

	const elegibles = useMemo(() => grupos.filter((g) => g.scryfallId), [grupos]);
	// Sin carta elegida (se entra por la barra), la primera que se puede diseñar.
	useEffect(() => {
		if (!grupo && elegibles[0]?.cartas[0]) useUi.getState().disenar(elegibles[0].cartas[0].id);
	}, [grupo, elegibles]);
	const pasar = useCallback(
		(dir: 1 | -1) => {
			if (elegibles.length === 0) return;
			const i = grupo ? elegibles.findIndex((g) => g.clave === grupo.clave) : -1;
			const sig = elegibles[(i + dir + elegibles.length) % elegibles.length];
			if (sig?.cartas[0]) useUi.getState().disenar(sig.cartas[0].id);
		},
		[elegibles, grupo],
	);
	useEffect(() => {
		const tecla = (e: KeyboardEvent) => {
			if (!e.altKey) return;
			if (e.key === "ArrowRight") pasar(1);
			if (e.key === "ArrowLeft") pasar(-1);
		};
		window.addEventListener("keydown", tecla);
		return () => window.removeEventListener("keydown", tecla);
	}, [pasar]);

	const [pendiente, setPendiente] = useState<Grupo | null>(null);
	const elegir = (g: Grupo) => {
		if (b.cambiado) return setPendiente(g);
		if (g.cartas[0]) useUi.getState().disenar(g.cartas[0].id);
	};

	const guardar = async (siguiente: boolean) => {
		const ok = await b.guardar(!soloEsta);
		if (!ok) return;
		useUi.getState().anunciar(t("Diseño guardado"));
		if (siguiente) pasar(1);
	};

	const encuadre = (framing: ArtFraming) =>
		b.cambiar((d) => {
			const arte = d.faces[cara]?.art;
			if (!arte) return d;
			return {
				...d,
				faces: d.faces.map((x, i) => (i === cara ? { ...x, art: { ...arte, framing } } : x)),
			};
		});

	const copias = grupo?.cartas.length ?? 0;
	const tieneDiseno = !!grupo?.cartas[0]?.design;

	return (
		<div className="flex h-full flex-col">
			<div className="flex shrink-0 items-center gap-2 border-b px-3 py-1.5">
				<Button
					size="sm"
					variant="ghost"
					onClick={() => pasar(-1)}
					aria-label={t("Carta anterior")}
				>
					<ChevronLeft />
				</Button>
				<Button
					size="sm"
					variant="ghost"
					onClick={() => pasar(1)}
					aria-label={t("Carta siguiente")}
				>
					<ChevronRight />
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
							<Hash />
							{t("Renumerar")}
						</Button>
					</WithTooltip>
					<Button
						size="sm"
						variant="ghost"
						onClick={() => useUi.getState().abrir({ tipo: "disenarDeck" })}
					>
						<Wand2 />
						{t("Diseñar el deck")}
					</Button>
					<Button
						size="sm"
						variant="ghost"
						onClick={() => useUi.getState().abrir({ tipo: "exportarPng" })}
						disabled={!cartas?.some((c) => c.design)}
					>
						<ImageDown />
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
						<SlidersHorizontal />
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
					<ListaCartas grupos={grupos} elegido={grupo?.clave ?? null} onElegir={elegir} />
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
									colecciones={colecciones}
									packs={packs}
									onColeccion={(c) =>
										setColecciones((cs) => cs.map((x) => (x.id === c.id ? c : x)))
									}
									resumen={resumen}
								/>
							</div>
							<div className="flex shrink-0 flex-col gap-2 border-t p-2.5">
								{copias > 1 && (
									<label className="flex items-center gap-2 text-xs">
										<Checkbox checked={soloEsta} onCheckedChange={(v) => setSoloEsta(v === true)} />
										{t("Solo esta copia (hay {n})", { n: copias })}
									</label>
								)}
								<div className="flex items-center gap-1.5">
									{tieneDiseno && (
										<WithTooltip content={t("Quita el diseño y deja la carta como estaba.")}>
											<Button
												size="sm"
												variant="ghost"
												onClick={b.quitar}
												aria-label={t("Quitar diseño")}
											>
												<Trash2 />
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
											<Undo2 />
										</Button>
									</WithTooltip>
									<div className="ml-auto flex gap-1.5">
										<Button
											size="sm"
											variant="outline"
											onClick={() => guardar(false)}
											disabled={b.guardando}
										>
											{b.guardando ? <Loader2 className="animate-spin" /> : <Save />}
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
								<Loader2 className="size-5 animate-spin" />
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
					if (pendiente?.cartas[0]) useUi.getState().disenar(pendiente.cartas[0].id);
				}}
			/>
		</div>
	);
}
