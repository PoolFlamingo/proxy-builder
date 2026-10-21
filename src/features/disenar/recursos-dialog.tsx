import {
	Check,
	Download,
	FolderOpen,
	Images,
	Loader2,
	Package,
	Plus,
	RefreshCw,
	SquareStack,
	Trash2,
	Type,
} from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { toast } from "sonner";
import type { DescargableInfo } from "@/bindings/DescargableInfo";
import type { LetraInfo } from "@/bindings/LetraInfo";
import type { LetraLibre } from "@/bindings/LetraLibre";
import type { PackInfo } from "@/bindings/PackInfo";
import type { SimbolosInfo } from "@/bindings/SimbolosInfo";
import type { TipoExtra } from "@/bindings/TipoExtra";
import { paraTraducir, useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { Progress } from "@/components/ui/progress";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { WithTooltip } from "@/components/ui/with-tooltip";
import { cartaPropia } from "@/lib/cartas";
import { insertar } from "@/lib/deck-ops";
import { abrirParaImportar } from "@/lib/dialogos";
import { urlExtra } from "@/lib/image-url";
import { call } from "@/lib/ipc";
import { cn } from "@/lib/utils";
import { useDeck } from "@/stores/deck-store";
import { useUi } from "@/stores/ui-store";
import { ID_ORIGINAL, useMarcosOriginales } from "./marcos-originales";
import { usePacksDescargables } from "./packs-descargables";
import { rehacerMiniaturas } from "./renumerar";
import { ESTILOS } from "./textos";

/**
 * **Letras, marcos y símbolos**: las letras y los packs de marcos importados
 * (las de la casa siempre están) y el estado de los símbolos de maná.
 */
export function RecursosDialog() {
	const abierto = useUi((s) => s.dialogo?.tipo === "recursosDiseno");
	return (
		<Dialog open={abierto} onOpenChange={(o) => !o && useUi.getState().abrir(null)}>
			{abierto && <Contenido />}
		</Dialog>
	);
}

function kb(bytes: bigint | number): string {
	return `${Math.max(1, Math.round(Number(bytes) / 1024))} KB`;
}

function Contenido() {
	const { t, lang, mensaje } = useLang();
	const [letras, setLetras] = useState<LetraInfo[] | null>(null);
	const [packs, setPacks] = useState<PackInfo[] | null>(null);
	const [simbolos, setSimbolos] = useState<SimbolosInfo | null>(null);
	const [ocupado, setOcupado] = useState<string | null>(null);
	const tocado = useRef(false);

	const recargar = useCallback(() => {
		call("fonts_list", {})
			.then(setLetras)
			.catch((e) => toast.error(mensaje(e)));
		call("packs_list", {})
			.then(setPacks)
			.catch((e) => toast.error(mensaje(e)));
	}, [mensaje]);

	useEffect(() => {
		recargar();
		call("symbols_status", { refresh: false })
			.then(setSimbolos)
			.catch(() => {});
		// Al cerrar, si algo cambió, se rehacen las miniaturas viejas.
		return () => {
			if (tocado.current) void rehacerMiniaturas();
		};
	}, [recargar]);

	/** Corre una acción marcando qué botón está ocupado. */
	const con = async (clave: string, f: () => Promise<void>) => {
		setOcupado(clave);
		try {
			await f();
			tocado.current = true;
			recargar();
		} catch (e) {
			toast.error(mensaje(e));
		} finally {
			setOcupado(null);
		}
	};

	const importarLetras = () =>
		con("letra", async () => {
			const rutas = await abrirParaImportar({
				multiple: true,
				filters: [{ name: t("Letras"), extensions: ["ttf", "otf"] }],
			});
			if (!rutas) return;
			for (const path of rutas) await call("font_import", { path });
			useUi.getState().anunciar(t("{n} letras importadas", { n: rutas.length }));
		});

	const importarPack = (carpeta: boolean) =>
		con("pack", async () => {
			const rutas = await abrirParaImportar(
				carpeta
					? { directory: true, multiple: false }
					: { multiple: false, filters: [{ name: t("Pack de marcos"), extensions: ["zip"] }] },
			);
			const ruta = rutas?.[0];
			if (!ruta) return;
			const p = await call("pack_import", { path: ruta });
			useUi.getState().anunciar(t("Pack «{nombre}» importado", { nombre: p.nombre }));
		});

	const refrescarSimbolos = async () => {
		setOcupado("simbolos");
		try {
			setSimbolos(await call("symbols_status", { refresh: true }));
			tocado.current = true;
		} catch (e) {
			toast.error(mensaje(e));
		} finally {
			setOcupado(null);
		}
	};

	const fecha = simbolos?.fecha
		? new Date(simbolos.fecha).toLocaleDateString(lang === "en" ? "en-GB" : "es-ES", {
				day: "numeric",
				month: "long",
				year: "numeric",
			})
		: null;

	return (
		<DialogContent className="flex h-[min(80vh,620px)] flex-col sm:max-w-2xl">
			<DialogHeader>
				<DialogTitle>{t("Letras, marcos y símbolos")}</DialogTitle>
				<DialogDescription className="text-xs">
					{t(
						"Lo que usa el editor para pintar las cartas. Lo de la casa siempre está; aquí se añade lo tuyo.",
					)}
				</DialogDescription>
			</DialogHeader>
			<Tabs defaultValue="letras" className="flex min-h-0 flex-1 flex-col">
				<TabsList>
					<TabsTrigger value="letras" className="text-xs">
						{t("Letras")}
					</TabsTrigger>
					<TabsTrigger value="packs" className="text-xs">
						{t("Packs de marcos")}
					</TabsTrigger>
					<TabsTrigger value="simbolos" className="text-xs">
						{t("Símbolos")}
					</TabsTrigger>
				</TabsList>

				<TabsContent value="letras" className="flex min-h-0 flex-1 flex-col gap-2.5">
					<div className="flex items-center gap-2.5">
						<p className="flex-1 text-xs text-muted-foreground">
							{t(
								"De la casa: Spectral (título y reglas) y Montserrat (pie). Importa .ttf u .otf para usarlas en una colección.",
							)}
						</p>
						<Button
							size="sm"
							variant="outline"
							onClick={importarLetras}
							disabled={ocupado !== null}
						>
							{ocupado === "letra" ? <Loader2 className="animate-spin" /> : <Type />}
							{t("Importar letras…")}
						</Button>
					</div>
					<ScrollArea className="min-h-0 flex-1 rounded-md border">
						<LetrasLibres
							ocupado={ocupado}
							descargar={(id) =>
								con(`libre-${id}`, () =>
									call("font_free_download", { id }).then((l) => {
										useUi.getState().anunciar(t("Letra «{n}» descargada", { n: l.familia }));
									}),
								)
							}
							version={letras}
						/>
						<div className="border-y bg-muted/40 px-3 py-1.5 text-[10px] font-semibold tracking-wider text-muted-foreground uppercase">
							{t("Tus letras")}
						</div>
						<ul className="flex flex-col divide-y">
							{letras === null && (
								<Loader2 className="m-3 size-4 animate-spin text-muted-foreground" />
							)}
							{letras?.length === 0 && (
								<li className="p-3 text-xs text-muted-foreground">
									{t("No has importado ninguna letra.")}
								</li>
							)}
							{letras?.map((l) => (
								<li key={l.id} className="flex items-center gap-2.5 px-3 py-2">
									<div className="min-w-0 flex-1">
										<div className="truncate text-xs font-medium">{l.familias.join(", ")}</div>
										<div className="font-mono text-[10px] text-muted-foreground">
											{l.id.slice(0, 12)} · {kb(l.bytes)}
										</div>
									</div>
									<WithTooltip content={t("Las cartas que la usen pasarán a la letra del pack.")}>
										<Button
											size="sm"
											variant="ghost"
											aria-label={t("Quitar letra")}
											disabled={ocupado !== null}
											onClick={() =>
												con(l.id, () => call("font_remove", { id: l.id }).then(() => {}))
											}
										>
											<Trash2 />
										</Button>
									</WithTooltip>
								</li>
							))}
						</ul>
					</ScrollArea>
				</TabsContent>

				<TabsContent value="packs" className="flex min-h-0 flex-1 flex-col gap-2.5">
					<div className="flex items-center gap-1.5">
						<p className="flex-1 text-xs text-muted-foreground">
							{t("Un pack es una carpeta (o un .zip) con su pack.json y sus capas en SVG.")}
						</p>
						<Button
							size="sm"
							variant="outline"
							onClick={() => importarPack(true)}
							disabled={ocupado !== null}
						>
							{ocupado === "pack" ? <Loader2 className="animate-spin" /> : <FolderOpen />}
							{t("Carpeta…")}
						</Button>
						<Button
							size="sm"
							variant="outline"
							onClick={() => importarPack(false)}
							disabled={ocupado !== null}
						>
							<Package />
							{t(".zip…")}
						</Button>
					</div>
					<ScrollArea className="min-h-0 flex-1 rounded-md border">
						<ul className="flex flex-col divide-y">
							{packs === null && (
								<Loader2 className="m-3 size-4 animate-spin text-muted-foreground" />
							)}
							{packs !== null && !packs.some((p) => p.id === ID_ORIGINAL) && (
								<FilaOriginales alTerminar={recargar} />
							)}
							{packs !== null && <FilasDescargables alTerminar={recargar} />}
							{packs?.map((p) => (
								<li key={p.id} className="flex items-center gap-2.5 px-3 py-2">
									<div className="min-w-0 flex-1">
										<div className="truncate text-xs font-medium">
											{p.importado ? p.nombre : t("Proxy Builder (de la casa)")}
										</div>
										<div className="truncate text-[10px] text-muted-foreground">
											{t("Estilos: {estilos}", {
												estilos: p.estilos
													.map((e) => {
														const conocido = ESTILOS.find((x) => x.valor === e);
														return conocido ? t(conocido.etiqueta) : e;
													})
													.join(", "),
											})}
										</div>
									</div>
									{p.importado && (
										<WithTooltip content={t("Las cartas que lo usen pasarán al pack de la casa.")}>
											<Button
												size="sm"
												variant="ghost"
												aria-label={t("Quitar pack")}
												disabled={ocupado !== null}
												onClick={() =>
													con(p.id, () => call("pack_remove", { id: p.id }).then(() => {}))
												}
											>
												<Trash2 />
											</Button>
										</WithTooltip>
									)}
								</li>
							))}
						</ul>
					</ScrollArea>
				</TabsContent>

				<TabsContent value="simbolos" className="flex flex-col gap-2.5">
					<p className="text-xs text-muted-foreground">
						{t(
							"Los símbolos de maná y de reglas vienen de Scryfall: se descargan la primera vez y se renuevan cada mes.",
						)}
					</p>
					<div className="flex items-center gap-2.5 rounded-md border p-3">
						<div className="flex-1 text-xs">
							{simbolos === null ? (
								<Loader2 className="size-4 animate-spin text-muted-foreground" />
							) : simbolos.cargados === 0 ? (
								t("No hay símbolos: hace falta conexión para descargarlos.")
							) : (
								<>
									<div className="font-medium">{t("{n} símbolos", { n: simbolos.cargados })}</div>
									{fecha && (
										<div className="text-[11px] text-muted-foreground">
											{t("Descargados el {fecha}", { fecha })}
										</div>
									)}
								</>
							)}
						</div>
						<Button
							size="sm"
							variant="outline"
							onClick={refrescarSimbolos}
							disabled={ocupado !== null}
						>
							{ocupado === "simbolos" ? <Loader2 className="animate-spin" /> : <RefreshCw />}
							{t("Descargar de nuevo")}
						</Button>
					</div>
				</TabsContent>
			</Tabs>
		</DialogContent>
	);
}

/** Los marcos «Original» sin descargar: qué son y el botón para bajarlos. */
function FilaOriginales({ alTerminar }: { alTerminar: () => void }) {
	const { t, mensaje } = useLang();
	const { info, progreso, cargar, descargar } = useMarcosOriginales();
	useEffect(() => {
		if (!info) void cargar();
	}, [info, cargar]);
	return (
		<li className="flex flex-col gap-2 px-3 py-2">
			<div className="flex items-center gap-2.5">
				<div className="min-w-0 flex-1">
					<div className="truncate text-xs font-medium">{t("Original (Card Conjurer)")}</div>
					<div className="text-[10px] text-muted-foreground">
						{t(
							"Marcos de imagen para Despejado, Extendido y Clásico (unos 37 MB). Se descargan de GitHub la primera vez.",
						)}
					</div>
				</div>
				<Button
					size="sm"
					variant="outline"
					disabled={progreso !== null}
					onClick={() =>
						descargar()
							.then(alTerminar)
							.catch((e) => toast.error(mensaje(e)))
					}
				>
					{progreso ? <Loader2 className="animate-spin" /> : <Download />}
					{t("Descargar")}
				</Button>
			</div>
			{progreso && <Progress value={(progreso.hechas * 100) / Math.max(1, progreso.total)} />}
		</li>
	);
}

/** Los packs descargables (Balatro): bajarlos y, ya bajados, su contenido extra. */
function FilasDescargables({ alTerminar }: { alTerminar: () => void }) {
	const { t, mensaje } = useLang();
	const { lista, progreso, cargar, descargar } = usePacksDescargables();
	const [abierto, setAbierto] = useState<string | null>(null);
	useEffect(() => {
		if (!lista) void cargar();
	}, [lista, cargar]);
	return (
		<>
			{(lista ?? []).map((d) => (
				<li key={d.id} className="flex flex-col gap-2 px-3 py-2">
					<div className="flex items-center gap-2.5">
						<div className="min-w-0 flex-1">
							<div className="truncate text-xs font-medium">{d.nombre}</div>
							<div className="text-[10px] text-muted-foreground">
								{d.listo
									? t(
											"Descargado: {n} imágenes extra (reversos, fichas, tierras y cartas hechas).",
											{
												n: d.extras.length,
											},
										)
									: t("Pack descargable (unos 45 MB con sus extras). Se descarga de GitHub.")}
							</div>
						</div>
						{d.listo ? (
							<Button
								size="sm"
								variant={abierto === d.id ? "secondary" : "outline"}
								onClick={() => setAbierto(abierto === d.id ? null : d.id)}
							>
								<Images />
								{t("Contenido")}
							</Button>
						) : (
							<Button
								size="sm"
								variant="outline"
								disabled={progreso !== null}
								onClick={() =>
									descargar(d.id)
										.then(alTerminar)
										.catch((e) => toast.error(mensaje(e)))
								}
							>
								{progreso?.id === d.id ? <Loader2 className="animate-spin" /> : <Download />}
								{t("Descargar")}
							</Button>
						)}
					</div>
					{progreso?.id === d.id && (
						<Progress value={(progreso.hechas * 100) / Math.max(1, progreso.total)} />
					)}
					{abierto === d.id && d.listo && <ContenidoPack pack={d} />}
				</li>
			))}
		</>
	);
}

const TIPOS_EXTRA: { valor: TipoExtra; etiqueta: string }[] = [
	{ valor: "reverso", etiqueta: paraTraducir("Reversos") },
	{ valor: "ficha", etiqueta: paraTraducir("Fichas") },
	{ valor: "tierra", etiqueta: paraTraducir("Tierras básicas") },
	{ valor: "carta", etiqueta: paraTraducir("Cartas hechas") },
];

/** Las imágenes extra de un pack, para ponerlas de reverso o añadirlas al deck. */
function ContenidoPack({ pack }: { pack: DescargableInfo }) {
	const { t, mensaje } = useLang();
	const [tipo, setTipo] = useState<TipoExtra>("reverso");
	const [elegido, setElegido] = useState<number | null>(null);
	const hayDeck = useDeck((s) => s.deck !== null);
	const extras = pack.extras.map((e, i) => ({ ...e, indice: i })).filter((e) => e.tipo === tipo);
	const actual = elegido !== null ? pack.extras[elegido] : null;
	const usar = async (como: "reverso" | "carta") => {
		if (!actual) return;
		try {
			const img = await call("pack_extra_use", { id: pack.id, ruta: actual.ruta });
			if (como === "reverso") {
				useDeck.getState().aplicar(() => ({ defaultBack: img }));
				useUi.getState().anunciar(t("Reverso del deck cambiado"));
			} else {
				useDeck.getState().setCartas((c) => insertar(c, [cartaPropia(img, actual.nombre)]));
				useUi.getState().anunciar(t("«{nombre}» añadida al deck", { nombre: actual.nombre }));
			}
		} catch (e) {
			toast.error(mensaje(e));
		}
	};
	return (
		<div className="flex flex-col gap-2 rounded-md border bg-muted/30 p-2">
			<div className="flex flex-wrap gap-1">
				{TIPOS_EXTRA.map((x) => (
					<Button
						key={x.valor}
						size="xs"
						variant={tipo === x.valor ? "secondary" : "ghost"}
						onClick={() => {
							setTipo(x.valor);
							setElegido(null);
						}}
					>
						{t(x.etiqueta)} ({pack.extras.filter((e) => e.tipo === x.valor).length})
					</Button>
				))}
			</div>
			<div className="grid grid-cols-[repeat(auto-fill,minmax(64px,1fr))] gap-1.5 p-1">
				{extras.map((e) => (
					<WithTooltip key={e.indice} content={e.nombre}>
						<button
							type="button"
							onClick={() => setElegido(e.indice)}
							className={cn(
								"overflow-hidden rounded-[6%] ring-offset-1 transition-shadow",
								elegido === e.indice ? "ring-2 ring-primary" : "hover:ring-1 hover:ring-border",
							)}
						>
							<img
								src={urlExtra(pack.id, e.indice)}
								alt=""
								loading="lazy"
								decoding="async"
								className="aspect-[63/88] w-full object-cover"
							/>
						</button>
					</WithTooltip>
				))}
			</div>
			<div className="flex flex-wrap items-center justify-end gap-1.5">
				{!hayDeck && (
					<span className="mr-auto text-[11px] text-muted-foreground">
						{t("Abre un deck para usarlas.")}
					</span>
				)}
				<Button
					size="sm"
					variant="outline"
					disabled={!actual || !hayDeck}
					onClick={() => usar("reverso")}
				>
					<SquareStack />
					{t("Usar como reverso del deck")}
				</Button>
				<Button size="sm" disabled={!actual || !hayDeck} onClick={() => usar("carta")}>
					<Plus />
					{t("Añadir al deck")}
				</Button>
			</div>
		</div>
	);
}

/** Las muestras de cada letra libre (su nombre escrito con ella). */
const MUESTRAS = import.meta.glob<string>("@/assets/letras/*.png", {
	eager: true,
	query: "?url",
	import: "default",
});
const muestra = (id: string) =>
	Object.entries(MUESTRAS).find(([ruta]) => ruta.endsWith(`/${id}.png`))?.[1];

/**
 * El catálogo de **letras libres** (OFL, de Google Fonts): de título, de
 * aire clásico, medieval o de fantasía, y de reglas, legibles y sin trazos
 * finos. Rust las descarga de una versión fija y las importa como las tuyas.
 */
function LetrasLibres({
	ocupado,
	descargar,
	version,
}: {
	ocupado: string | null;
	descargar: (id: string) => void;
	/** Cambia cuando cambian las letras importadas: se relee el catálogo. */
	version: unknown;
}) {
	const { t } = useLang();
	const [lista, setLista] = useState<LetraLibre[] | null>(null);
	useEffect(() => {
		void version;
		call("fonts_free_list", {})
			.then(setLista)
			.catch(() => setLista([]));
	}, [version]);
	if (!lista || lista.length === 0) return null;
	const grupos: { titulo: string; letras: LetraLibre[] }[] = [
		{ titulo: t("Para el título"), letras: lista.filter((l) => l.uso !== "reglas") },
		{ titulo: t("Para las reglas"), letras: lista.filter((l) => l.uso === "reglas") },
	];
	return (
		<div className="flex flex-col">
			<p className="px-3 pt-2.5 pb-1.5 text-[11px] text-muted-foreground">
				{t(
					"Letras libres (licencia OFL) para descargar. Se usan en las colecciones como cualquier letra tuya.",
				)}
			</p>
			{grupos.map((g) => (
				<div key={g.titulo} className="flex flex-col">
					<div className="border-y bg-muted/40 px-3 py-1.5 text-[10px] font-semibold tracking-wider text-muted-foreground uppercase">
						{g.titulo}
					</div>
					<ul className="flex flex-col divide-y">
						{g.letras.map((l) => {
							const src = muestra(l.id);
							return (
								<li key={l.id} className="flex items-center gap-2.5 px-3 py-2">
									<div className="flex min-w-0 flex-1 flex-col gap-1">
										{src ? (
											<img
												src={src}
												alt={l.familia}
												className="h-5 max-w-full self-start object-contain object-left dark:invert"
											/>
										) : (
											<span className="text-xs font-medium">{l.familia}</span>
										)}
										<span className="text-[10px] text-muted-foreground">
											{[
												l.uso === "ambos" ? t("También para las reglas") : null,
												!l.cursiva ? t("Sin cursiva") : null,
												kb(l.bytes),
											]
												.filter(Boolean)
												.join(" · ")}
										</span>
									</div>
									{l.descargada ? (
										<span className="flex items-center gap-1 text-[11px] text-muted-foreground">
											<Check className="size-3.5" />
											{t("Descargada")}
										</span>
									) : (
										<Button
											size="sm"
											variant="outline"
											disabled={ocupado !== null}
											onClick={() => descargar(l.id)}
										>
											{ocupado === `libre-${l.id}` ? (
												<Loader2 className="animate-spin" />
											) : (
												<Download />
											)}
											{t("Descargar")}
										</Button>
									)}
								</li>
							);
						})}
					</ul>
				</div>
			))}
		</div>
	);
}
