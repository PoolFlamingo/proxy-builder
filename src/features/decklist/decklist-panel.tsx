import {
	Download,
	Eraser,
	FileText,
	Grid3x3,
	ImagePlus,
	Languages,
	Link,
	Link2,
	ListPlus,
	Loader2,
	Plus,
	RefreshCw,
	SquareStack,
	Trash2,
	Upload,
	X,
} from "lucide-react";
import { useEffect, useState } from "react";
import { useDefaultLayout } from "react-resizable-panels";
import { toast } from "sonner";
import type { CacheStats } from "@/bindings/CacheStats";
import type { ImagePolicy } from "@/bindings/ImagePolicy";
import type { ParsedDecklist } from "@/bindings/ParsedDecklist";
import { Fila, FilaInterruptor, type Opcion, Seccion, Selector } from "@/components/campos";
import { Confirmar } from "@/components/confirmar";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { Progress } from "@/components/ui/progress";
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from "@/components/ui/resizable";
import { ScrollArea } from "@/components/ui/scroll-area";
import { WithTooltip } from "@/components/ui/with-tooltip";
import type { AnchoMiniatura } from "@/lib/image-url";
import { call } from "@/lib/ipc";
import { useDeck } from "@/stores/deck-store";
import { usePlan } from "@/stores/plan-store";
import { useTrabajo } from "@/stores/trabajo-store";
import { useUi } from "@/stores/ui-store";
import {
	cancelarTrabajo,
	cargarLista,
	reaplicarIdioma,
	reversoGenerico,
	subirCartasPropias,
	subirReverso,
} from "./acciones";
import { EditorLista } from "./editor-lista";
import { EJEMPLO } from "./ejemplo";

/** Idiomas de Scryfall, con su nombre propio (no se traducen). */
const IDIOMAS: Opcion<string>[] = [
	{ valor: "en", etiqueta: "English" },
	{ valor: "es", etiqueta: "Español" },
	{ valor: "fr", etiqueta: "Français" },
	{ valor: "de", etiqueta: "Deutsch" },
	{ valor: "it", etiqueta: "Italiano" },
	{ valor: "pt", etiqueta: "Português" },
	{ valor: "ja", etiqueta: "日本語" },
	{ valor: "ko", etiqueta: "한국어" },
	{ valor: "ru", etiqueta: "Русский" },
	{ valor: "zhs", etiqueta: "简体中文" },
	{ valor: "zht", etiqueta: "繁體中文" },
];

function megas(b: number) {
	return `${(b / 1024 / 1024).toFixed(b > 100 * 1024 * 1024 ? 0 : 1)} MB`;
}

/**
 * **Panel izquierdo**: la lista de cartas y todo lo que mete cartas en el
 * deck (cargar, añadir, importar, subir imágenes propias).
 */
export function DecklistPanel() {
	const { t, mensaje } = useLang();
	const texto = useDeck((s) => s.deck?.decklistText ?? "");
	const hayCartas = useDeck((s) => (s.deck?.cards.length ?? 0) > 0);
	const setSinHistorial = useDeck((s) => s.setSinHistorial);
	const trabajo = useTrabajo((s) => s.actual);
	const ocupado = trabajo !== null;
	const ui = useUi();
	const [leida, setLeida] = useState<ParsedDecklist | null>(null);
	const [confirmarCarga, setConfirmarCarga] = useState(false);
	const [noPreguntar, setNoPreguntar] = useState(false);
	const [confirmarCache, setConfirmarCache] = useState<CacheStats | null>(null);
	const [url, setUrl] = useState("");
	const [conBanquillo, setConBanquillo] = useState(false);
	const [importando, setImportando] = useState(false);
	const layout = useDefaultLayout({ id: "proxybuilder.panel-lista", storage: localStorage });

	const intentar = (f: () => Promise<unknown>) => async () => {
		try {
			const aviso = await f();
			if (typeof aviso === "string") toast.warning(aviso);
		} catch (e) {
			toast.error(mensaje(e));
		}
	};

	const cargar = intentar(() => cargarLista(texto, "reemplazar", t));
	const pulsarCargar = () => {
		if (hayCartas && ui.pedirConfirmacionAlCargar) setConfirmarCarga(true);
		else void cargar();
	};

	const importar = intentar(async () => {
		setImportando(true);
		try {
			const r = await call("import_archidekt", { url: url.trim(), includeSideboard: conBanquillo });
			setSinHistorial({ decklistText: r.text });
			useUi
				.getState()
				.anunciar(t("Lista importada de Archidekt ({n} cartas)", { n: r.list.cardCount }));
			setUrl("");
		} finally {
			setImportando(false);
		}
	});

	const pedirLimpiarCache = intentar(async () => {
		setConfirmarCache(await call("cache_stats", {}));
	});

	const politicas: Opcion<ImagePolicy>[] = [
		{ valor: "withImage", etiqueta: t("Con imagen") },
		{ valor: "bestAvailable", etiqueta: t("La mejor disponible") },
		{ valor: "highresOnly", etiqueta: t("Solo alta resolución") },
	];
	const calidades: Opcion<string>[] = [
		{ valor: "244", etiqueta: t("Baja (244 px)") },
		{ valor: "488", etiqueta: t("Normal (488 px)") },
		{ valor: "672", etiqueta: t("Alta (672 px)") },
		{ valor: "745", etiqueta: t("Máxima (745 px)") },
	];

	return (
		<section className="flex h-full min-h-0 flex-col [&_[data-slot=button]]:text-xs [&_button[data-slot=tooltip-trigger]]:text-xs">
			<div className="flex items-center gap-1.5 px-2.5 pt-2.5">
				<h2 className="flex-1 text-[11px] font-semibold uppercase tracking-widest text-muted-foreground">
					{t("Lista de cartas")}
				</h2>
				<WithTooltip content={t("Ver un set entero y elegir cartas")}>
					<Button
						size="xs"
						variant="ghost"
						onClick={() => useUi.getState().abrir({ tipo: "sets" })}
						disabled={ocupado}
					>
						<Grid3x3 />
						{t("Sets")}
					</Button>
				</WithTooltip>
				<WithTooltip content={t("Poner una lista de ejemplo")}>
					<Button
						size="xs"
						variant="ghost"
						onClick={() => setSinHistorial({ decklistText: EJEMPLO })}
						disabled={ocupado}
					>
						<FileText />
						{t("Ejemplo")}
					</Button>
				</WithTooltip>
				<WithTooltip content={t("Vaciar el cuadro (las cartas del deck no se tocan)")}>
					<Button
						size="xs"
						variant="ghost"
						onClick={() => setSinHistorial({ decklistText: "" })}
						disabled={ocupado || texto === ""}
					>
						<Eraser />
						{t("Limpiar")}
					</Button>
				</WithTooltip>
			</div>

			{/* El cuadro de la lista y lo de debajo, con un asa entre medias para
			    darle al cuadro el alto que se quiera (se recuerda). */}
			<ResizablePanelGroup
				orientation="vertical"
				className="min-h-0 flex-1"
				defaultLayout={layout.defaultLayout}
				onLayoutChanged={layout.onLayoutChanged}
			>
				<ResizablePanel id="lista" defaultSize="45" minSize={72}>
					<div className="flex h-full min-h-0 flex-col p-2.5 pb-1.5">
						<EditorLista
							valor={texto}
							onChange={(v) => setSinHistorial({ decklistText: v })}
							onLeida={setLeida}
							deshabilitado={ocupado}
						/>
					</div>
				</ResizablePanel>
				<ResizableHandle withHandle />
				<ResizablePanel id="opciones" minSize={160}>
					<div className="flex h-full min-h-0 flex-col">
						<div className="flex shrink-0 flex-col gap-2.5 p-2.5">
							<div className="flex items-center justify-between text-[11px] text-muted-foreground">
								<span>{leida ? t("{n} cartas en la lista", { n: leida.cardCount }) : " "}</span>
								{leida && leida.invalidCount > 0 && (
									<span className="text-destructive">
										{t("{n} líneas sin entender", { n: leida.invalidCount })}
									</span>
								)}
							</div>
							<div className="grid grid-cols-[1fr_auto] gap-1.5">
								<Button size="sm" onClick={pulsarCargar} disabled={ocupado || !leida?.cardCount}>
									<Download />
									{t("Cargar lista")}
								</Button>
								<WithTooltip content={t("Añadir estas cartas al final del deck")}>
									<Button
										size="sm"
										variant="outline"
										onClick={intentar(() => cargarLista(texto, "anadir", t))}
										disabled={ocupado || !leida?.cardCount}
									>
										<Plus />
										{t("Añadir")}
									</Button>
								</WithTooltip>
							</div>
							<label className="flex items-center gap-2 text-xs">
								<Checkbox
									checked={ui.addTokens}
									onCheckedChange={(v) => ui.setPreferencia("addTokens", v === true)}
								/>
								{t("Añadir tokens automáticamente")}
							</label>
						</div>

						<ScrollArea className="min-h-0 flex-1 border-t">
							<div className="flex flex-col gap-2.5 p-2.5">
								<Seccion
									id="importar"
									titulo={t("Importar desde URL")}
									icono={<Link className="size-3.5" />}
								>
									<div className="flex gap-1.5">
										<Input
											value={url}
											onChange={(e) => setUrl(e.target.value)}
											placeholder="https://archidekt.com/decks/…"
											className="h-7 text-xs"
											data-filtro=""
											onKeyDown={(e) => e.key === "Enter" && url.trim() && void importar()}
										/>
										<Button
											size="sm"
											variant="outline"
											className="h-7"
											disabled={!url.trim() || importando}
											onClick={importar}
										>
											{importando ? <Loader2 className="animate-spin" /> : <ListPlus />}
											{t("Importar")}
										</Button>
									</div>
									<label className="flex items-center gap-2 text-xs">
										<Checkbox
											checked={conBanquillo}
											onCheckedChange={(v) => setConBanquillo(v === true)}
										/>
										{t("Incluir el banquillo")}
									</label>
									<p className="text-[11px] text-muted-foreground">
										{t("Archidekt. De Moxfield no se puede: copia la lista y pégala arriba.")}
									</p>
								</Seccion>

								<Seccion
									id="propias"
									titulo={t("Imágenes propias")}
									icono={<ImagePlus className="size-3.5" />}
								>
									<div className="grid grid-cols-2 gap-1.5">
										<Button
											size="sm"
											variant="outline"
											onClick={intentar(() => subirCartasPropias(t))}
										>
											<Upload />
											{t("Subir cartas")}
										</Button>
										<Button size="sm" variant="outline" onClick={intentar(() => subirReverso(t))}>
											<SquareStack />
											{t("Subir reverso")}
										</Button>
									</div>
									<Button size="xs" variant="ghost" onClick={intentar(() => reversoGenerico(t))}>
										<RefreshCw />
										{t("Volver al reverso genérico")}
									</Button>
									<div className="flex flex-col gap-1.5">
										<WithTooltip
											content={t(
												"Busca en Scryfall la carta real de cada imagen propia (por el nombre del fichero o leyendo la carta: el título, el nombre original del pie o cualquier línea) y te enseña lo que ha encontrado para que lo revises antes de vincular. Si hay cartas seleccionadas, solo esas.",
											)}
										>
											<Button
												size="sm"
												variant="outline"
												onClick={() => {
													const sel = useUi.getState().seleccion;
													useUi
														.getState()
														.abrir({ tipo: "vincularLote", ids: sel.length > 0 ? sel : undefined });
												}}
												disabled={ocupado}
											>
												<Link2 />
												{t("Vincular con Scryfall")}
											</Button>
										</WithTooltip>
									</div>
									<p className="text-[11px] text-muted-foreground">
										{t(
											"También puedes arrastrar imágenes a la vista previa: sobre una carta la reemplazan; fuera, se añaden.",
										)}
									</p>
								</Seccion>

								<Seccion
									id="opciones"
									titulo={t("Opciones")}
									icono={<Languages className="size-3.5" />}
								>
									<Fila etiqueta={t("Idioma de las cartas")}>
										<Selector
											valor={ui.lang}
											opciones={IDIOMAS}
											onChange={(v) => ui.setPreferencia("lang", v)}
											ariaLabel={t("Idioma de las cartas")}
										/>
									</Fila>
									<Fila
										etiqueta={t("Imágenes")}
										ayuda={t("Qué impresión se elige cuando la lista no dice el set")}
									>
										<Selector
											valor={ui.imagePolicy}
											opciones={politicas}
											onChange={(v) => ui.setPreferencia("imagePolicy", v)}
											ariaLabel={t("Imágenes")}
										/>
									</Fila>
									<Fila
										etiqueta={t("Calidad de la vista previa")}
										ayuda={t(
											"Solo la pantalla. El PDF siempre sale a la resolución de exportación.",
										)}
									>
										<Selector
											valor={String(ui.calidadVista)}
											opciones={calidades}
											onChange={(v) =>
												ui.setPreferencia("calidadVista", Number(v) as AnchoMiniatura)
											}
											ariaLabel={t("Calidad de la vista previa")}
										/>
									</Fila>
									<FilaInterruptor
										etiqueta={t("Preguntar antes de reemplazar las cartas")}
										valor={ui.pedirConfirmacionAlCargar}
										onChange={(v) => ui.setPreferencia("pedirConfirmacionAlCargar", v)}
									/>
									<WithTooltip
										content={t("Vuelve a pedir las cartas no fijadas en el idioma elegido")}
									>
										<Button
											size="sm"
											variant="outline"
											onClick={intentar(() => reaplicarIdioma(t))}
											disabled={ocupado || !hayCartas}
										>
											<Languages />
											{t("Reaplicar idioma")}
										</Button>
									</WithTooltip>
									<Button size="xs" variant="ghost" onClick={pedirLimpiarCache}>
										<Trash2 />
										{t("Limpiar caché de miniaturas")}
									</Button>
								</Seccion>
							</div>
						</ScrollArea>
					</div>
				</ResizablePanel>
			</ResizablePanelGroup>

			<PiePanel />

			<Confirmar
				abierto={confirmarCarga}
				onCerrar={() => setConfirmarCarga(false)}
				titulo={t("¿Reemplazar las cartas del deck?")}
				descripcion={t(
					"Las cartas actuales se cambian por las de la lista. Se puede deshacer con Ctrl+Z.",
				)}
				accion={t("Reemplazar")}
				onConfirmar={() => {
					if (noPreguntar) ui.setPreferencia("pedirConfirmacionAlCargar", false);
					void cargar();
				}}
			>
				<label className="flex items-center gap-2 text-xs">
					<Checkbox checked={noPreguntar} onCheckedChange={(v) => setNoPreguntar(v === true)} />
					{t("No volver a preguntar")}
				</label>
			</Confirmar>

			<Confirmar
				abierto={confirmarCache !== null}
				onCerrar={() => setConfirmarCache(null)}
				titulo={t("¿Limpiar la caché de miniaturas?")}
				descripcion={t(
					"Ocupa {mb} en {n} ficheros. Se vuelven a generar al verlas. Las imágenes de tus decks no se tocan.",
					{ mb: megas(confirmarCache?.bytes ?? 0), n: confirmarCache?.files ?? 0 },
				)}
				accion={t("Limpiar")}
				onConfirmar={intentar(async () => {
					const r = await call("cache_clear", {});
					useUi.getState().anunciar(t("Caché limpiada ({mb})", { mb: megas(r.bytes) }));
				})}
			/>
		</section>
	);
}

/** El pie: el trabajo en marcha (con «Cancelar») o el resumen y la última acción. */
function PiePanel() {
	const { t } = useLang();
	const trabajo = useTrabajo((s) => s.actual);
	const cartas = useDeck((s) => s.deck?.cards.length ?? 0);
	const hojas = usePlan((s) => s.plan?.sheets ?? 0);
	const ultima = useUi((s) => s.ultimaAccion);
	const [visible, setVisible] = useState(ultima);

	useEffect(() => {
		setVisible(ultima);
		if (!ultima) return;
		const id = setTimeout(() => useUi.getState().anunciar(null), 8000);
		return () => clearTimeout(id);
	}, [ultima]);

	return (
		<footer className="flex min-h-11 shrink-0 flex-col justify-center gap-1 border-t px-2.5 py-1.5 text-[11px]">
			{trabajo ? (
				<>
					<div className="flex items-center gap-2">
						<Loader2 className="size-3.5 animate-spin text-primary" />
						<span className="flex-1 truncate">{trabajo.fase}</span>
						{trabajo.total > 0 && (
							<span className="font-mono text-muted-foreground">
								{trabajo.hechas}/{trabajo.total}
							</span>
						)}
						<Button size="xs" variant="ghost" onClick={cancelarTrabajo} disabled={!trabajo.jobId}>
							<X />
							{t("Cancelar")}
						</Button>
					</div>
					<Progress
						value={trabajo.total ? (trabajo.hechas / trabajo.total) * 100 : 0}
						className="h-1"
					/>
				</>
			) : (
				<div className="flex items-center gap-2">
					<span className="font-medium">
						{t("{n} cartas · {p} hojas", { n: cartas, p: hojas })}
					</span>
					{visible && (
						<span className="min-w-0 flex-1 truncate text-right text-muted-foreground">
							{visible}
						</span>
					)}
				</div>
			)}
		</footer>
	);
}
