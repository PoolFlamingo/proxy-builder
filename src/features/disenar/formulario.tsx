import {
	Clipboard,
	Crop,
	ImagePlus,
	Lock,
	RotateCcw,
	Save,
	Sparkles,
	Trash2,
	Upload,
} from "lucide-react";
import { useEffect, useState } from "react";
import { toast } from "sonner";
import type { ArtRef } from "@/bindings/ArtRef";
import type { CardDesign } from "@/bindings/CardDesign";
import type { Collection } from "@/bindings/Collection";
import type { DesignLang } from "@/bindings/DesignLang";
import type { EmblemChoice } from "@/bindings/EmblemChoice";
import type { FaceTexts } from "@/bindings/FaceTexts";
import type { FineTuning } from "@/bindings/FineTuning";
import type { FooterOptions } from "@/bindings/FooterOptions";
import type { FrameOptions } from "@/bindings/FrameOptions";
import type { PackInfo } from "@/bindings/PackInfo";
import type { Parte } from "@/bindings/Parte";
import type { Resumen } from "@/bindings/Resumen";
import { Fila, FilaDeslizador, FilaInterruptor, Selector } from "@/components/campos";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Progress } from "@/components/ui/progress";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { abrirParaImportar } from "@/lib/dialogos";
import { idioma } from "@/lib/idiomas";
import { call } from "@/lib/ipc";
import { conIdioma, porIdioma, textosDe } from "@/lib/por-idioma";
import { cn } from "@/lib/utils";
import { useDeck } from "@/stores/deck-store";
import { useUi } from "@/stores/ui-store";
import { AJUSTES_CERO, AjustesFinos, hayAjustes } from "./ajustes-finos";
import { ZOOM_MAX, ZOOM_MIN } from "./encuadre";
import {
	elegirEstilo,
	nombreEnIdioma,
	opcionesEstilo,
	packEfectivo,
	propiosDe,
	valorEstilo,
} from "./estilos";
import { ID_ORIGINAL, useMarcosOriginales } from "./marcos-originales";
import { usePacksDescargables } from "./packs-descargables";
import { RecortarDialog } from "./recortar-dialog";
import { rehacerMiniaturas } from "./renumerar";
import { letraRareza, MAQUETACIONES, NOMBRE_ORIGINAL, PALETAS, RAREZAS } from "./textos";
import { TraduccionSugerida } from "./traduccion";

type Cambiar = (f: (d: CardDesign) => CardDesign) => void;

const EXTENSIONES = ["png", "jpg", "jpeg", "webp"];

/** Un área de texto con el aspecto de los `Input` de la casa. */
function AreaTexto({
	valor,
	onChange,
	placeholder,
	filas = 3,
	ariaLabel,
}: {
	valor: string;
	onChange: (v: string) => void;
	placeholder?: string;
	filas?: number;
	ariaLabel?: string;
}) {
	return (
		<textarea
			value={valor}
			onChange={(e) => onChange(e.target.value)}
			placeholder={placeholder}
			rows={filas}
			aria-label={ariaLabel}
			className="w-full resize-y rounded-md border border-input bg-transparent px-2.5 py-1.5 text-xs shadow-xs outline-none placeholder:text-muted-foreground focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/50"
		/>
	);
}

function Etiqueta({ children }: { children: React.ReactNode }) {
	return (
		<span className="text-[10px] font-semibold tracking-wider text-muted-foreground uppercase">
			{children}
		</span>
	);
}

/** Cambia los textos de una cara en un idioma. */
function conTextos(
	d: CardDesign,
	cara: number,
	lang: DesignLang,
	c: Partial<FaceTexts>,
): CardDesign {
	const faces = d.faces.map((f, i) =>
		i === cara
			? { ...f, texts: conIdioma(f.texts, lang, { ...textosDe(f.texts, lang), ...c }) }
			: f,
	);
	return { ...d, faces };
}

/** Cambia la línea de tipo de una cara en un idioma (la de la carta, no un texto del diseño). */
function conTipo(d: CardDesign, cara: number, lang: DesignLang, tipo: string): CardDesign {
	const faces = d.card.faces.map((f, i) =>
		i === cara ? { ...f, typeLine: conIdioma(f.typeLine, lang, tipo) } : f,
	);
	return { ...d, card: { ...d.card, faces } };
}

function conArte(d: CardDesign, cara: number, art: ArtRef | undefined): CardDesign {
	return { ...d, faces: d.faces.map((f, i) => (i === cara ? { ...f, art } : f)) };
}

/** El formulario de la derecha, en pestañas. */
export function Formulario({
	design,
	cambiar,
	cara,
	colecciones,
	packs,
	resumen,
	onColeccion,
}: {
	design: CardDesign;
	cambiar: Cambiar;
	cara: number;
	colecciones: Collection[];
	packs: PackInfo[];
	resumen: Resumen | null;
	/** Una colección guardada desde aquí (para tenerla al día). */
	onColeccion?: (c: Collection) => void;
}) {
	const { t } = useLang();
	return (
		<Tabs defaultValue="carta" className="flex h-full min-h-0 flex-col gap-0">
			<div className="shrink-0 border-b px-2.5 py-2">
				<TabsList className="w-full">
					<TabsTrigger value="carta">{t("Carta")}</TabsTrigger>
					<TabsTrigger value="textos">{t("Textos")}</TabsTrigger>
					<TabsTrigger value="arte">{t("Arte")}</TabsTrigger>
					<TabsTrigger value="marco">{t("Marco")}</TabsTrigger>
					<TabsTrigger value="pie">{t("Pie")}</TabsTrigger>
					<TabsTrigger value="ajustes">{t("Ajustes finos")}</TabsTrigger>
				</TabsList>
			</div>
			<ScrollArea className="min-h-0 flex-1">
				<div className="p-2.5">
					<TabsContent value="carta">
						<PestanaCarta
							design={design}
							cambiar={cambiar}
							cara={cara}
							colecciones={colecciones}
							packs={packs}
							resumen={resumen}
						/>
					</TabsContent>
					<TabsContent value="textos">
						<PestanaTextos design={design} cambiar={cambiar} cara={cara} />
					</TabsContent>
					<TabsContent value="arte">
						<PestanaArte design={design} cambiar={cambiar} cara={cara} />
					</TabsContent>
					<TabsContent value="marco">
						<PestanaMarco
							design={design}
							cambiar={cambiar}
							colecciones={colecciones}
							packs={packs}
						/>
					</TabsContent>
					<TabsContent value="pie">
						<PestanaPie design={design} cambiar={cambiar} colecciones={colecciones} />
					</TabsContent>
					<TabsContent value="ajustes">
						<PestanaAjustes
							design={design}
							cambiar={cambiar}
							colecciones={colecciones}
							onColeccion={onColeccion}
						/>
					</TabsContent>
				</div>
			</ScrollArea>
		</Tabs>
	);
}

// ── Carta ──────────────────────────────────────────────────────────────────

/**
 * Un dato de la carta: etiqueta y valor, dentro de la rejilla de `Datos`. La
 * columna de las etiquetas mide lo que la más larga (como mucho la mitad):
 * con un ancho fijo, en otro idioma o con una letra más ancha se montaban
 * sobre el valor.
 */
function Dato({ etiqueta, children }: { etiqueta: string; children: React.ReactNode }) {
	return (
		<>
			<dt className="break-words text-muted-foreground">{etiqueta}</dt>
			<dd className="min-w-0 break-words">{children}</dd>
		</>
	);
}

/** Cómo ha partido el motor el texto: habilidades, capítulos, niveles… */
function Partes({ partes }: { partes: Parte[] }) {
	const { t } = useLang();
	return (
		<div className="flex flex-col gap-1.5 rounded-lg border p-2.5">
			<div className="text-[11px] font-semibold text-muted-foreground">
				{t("Cómo se reparte el texto en la carta")}
			</div>
			<ol className="flex flex-col gap-1.5">
				{partes.map((p, i) => (
					<li key={i} className="flex gap-2 text-[11px] leading-snug">
						{p.marca && (
							<span className="h-fit shrink-0 rounded bg-foreground/85 px-1.5 py-px font-mono text-[10px] text-background">
								{p.marca}
							</span>
						)}
						<span className="min-w-0 flex-1">
							{p.cabecera && <span className="block font-semibold">{p.cabecera}</span>}
							{p.texto}
						</span>
						{p.fr && (
							<span className="shrink-0 font-mono text-[10px] text-muted-foreground">{p.fr}</span>
						)}
					</li>
				))}
			</ol>
		</div>
	);
}

function PestanaCarta({
	design,
	cambiar,
	cara,
	colecciones,
	packs,
	resumen,
}: {
	design: CardDesign;
	cambiar: Cambiar;
	cara: number;
	colecciones: Collection[];
	packs: PackInfo[];
	resumen: Resumen | null;
}) {
	const { t, lang } = useLang();
	const langDiseno = useDeck((s) => s.deck?.settings.designLang ?? "es");
	const propios = propiosDe(packs, packEfectivo(design, colecciones));
	const valor = valorEstilo(design.style, design.estiloPack);
	const variantes = propios.find((e) => e.id === design.estiloPack)?.variantes ?? [];
	const sinVariante = "__carta";
	const f = design.card.faces[cara] ?? design.card.faces[0];
	const fr =
		f?.power != null && f.toughness != null
			? `${f.power}/${f.toughness}`
			: (f?.loyalty ?? f?.defense);
	const ninguna = "__ninguna";
	return (
		<div className="flex flex-col gap-3">
			<div className="flex flex-col gap-1.5 rounded-lg border bg-muted/40 p-2.5">
				<div className="flex items-center gap-1.5 text-[11px] font-semibold text-muted-foreground">
					<Lock className="size-3.5" />
					{t("Datos de la carta original (no se tocan)")}
				</div>
				<dl className="grid grid-cols-[fit-content(50%)_minmax(0,1fr)] gap-x-2.5 gap-y-1.5 text-xs">
					<Dato etiqueta={t("Nombre original")}>
						{design.card.faces.map((x) => x.name).join(" // ")}
					</Dato>
					<Dato etiqueta={t("Coste")}>
						<span className="font-mono">{f?.manaCost || "—"}</span>
					</Dato>
					<Dato etiqueta={t("Tipo")}>
						{f && porIdioma(f.typeLine, langDiseno)}
						{f?.typeLine.en && f.typeLine.en !== porIdioma(f.typeLine, langDiseno) && (
							<span className="block text-muted-foreground">{f.typeLine.en}</span>
						)}
					</Dato>
					{fr && <Dato etiqueta={t("Fuerza/resistencia")}>{fr}</Dato>}
					<Dato etiqueta={t("Rareza")}>{t(RAREZAS[design.card.rarity] ?? design.card.rarity)}</Dato>
					<Dato etiqueta={t("Edición")}>
						{design.card.set.toUpperCase()} {design.card.collectorNumber}
					</Dato>
					{resumen && (
						<Dato etiqueta={t("Maquetación")}>{t(MAQUETACIONES[resumen.maquetacion])}</Dato>
					)}
				</dl>
			</div>
			{resumen && resumen.partes.length > 0 && <Partes partes={resumen.partes} />}
			<Fila etiqueta={t("Estilo")}>
				<Selector
					valor={valor}
					opciones={opcionesEstilo(t, lang, propios, valor)}
					onChange={(v) =>
						cambiar((d) => ({
							...d,
							...elegirEstilo(v, propios, d.style),
							variantePack: undefined,
						}))
					}
					ancho="w-48"
				/>
			</Fila>
			{variantes.length > 0 && (
				<Fila etiqueta={t("Variante")}>
					<Selector
						valor={design.variantePack ?? sinVariante}
						opciones={[
							{ valor: sinVariante, etiqueta: t("Según la carta") },
							...variantes.map((v) => ({
								valor: v.id,
								etiqueta: nombreEnIdioma(v.nombres, lang, v.id),
							})),
						]}
						onChange={(v) =>
							cambiar((d) => ({ ...d, variantePack: v === sinVariante ? undefined : v }))
						}
						ancho="w-48"
					/>
				</Fila>
			)}
			<Fila etiqueta={t("Colección")}>
				<Selector
					valor={design.collection ?? ninguna}
					opciones={[
						{ valor: ninguna, etiqueta: t("Ninguna") },
						...colecciones.map((c) => ({ valor: c.id, etiqueta: c.name })),
					]}
					onChange={(v) => cambiar((d) => ({ ...d, collection: v === ninguna ? undefined : v }))}
					ancho="w-48"
				/>
			</Fila>
			<Button
				size="sm"
				variant="outline"
				className="self-end"
				onClick={() => useUi.getState().abrir({ tipo: "colecciones" })}
			>
				{t("Gestionar colecciones…")}
			</Button>
			<FilaInterruptor
				etiqueta={t("Lista para imprimir")}
				ayuda={t("Las cartas en borrador se ven en el deck, pero así sabes cuáles has revisado.")}
				valor={design.status === "ready"}
				onChange={(v) => cambiar((d) => ({ ...d, status: v ? "ready" : "draft" }))}
			/>
		</div>
	);
}

// ── Textos ─────────────────────────────────────────────────────────────────

function PestanaTextos({
	design,
	cambiar,
	cara,
}: {
	design: CardDesign;
	cambiar: Cambiar;
	cara: number;
}) {
	const { t } = useLang();
	const langDiseno = useDeck((s) => s.deck?.settings.designLang ?? "es");
	const textos = design.faces[cara]?.texts;
	const datos = design.card.faces[cara];
	if (!textos || !datos) return null;
	// El idioma de diseño del deck y, debajo, el inglés (el original); con el
	// inglés, el español. Los textos de los demás idiomas se guardan igual.
	const langs: DesignLang[] = langDiseno === "en" ? ["en", "es"] : [langDiseno, "en"];
	const idiomas = langs.map((lang) => ({ lang, nombre: idioma(lang).nativo }));
	return (
		<div className="flex flex-col gap-3">
			{idiomas.map(({ lang, nombre }) => {
				const tx = textosDe(textos, lang);
				const aMano = tx.rulesOverride !== undefined;
				return (
					<div key={lang} className="flex flex-col gap-1.5 rounded-lg border p-2.5">
						<Etiqueta>{nombre}</Etiqueta>
						<Input
							value={tx.name}
							onChange={(e) => cambiar((d) => conTextos(d, cara, lang, { name: e.target.value }))}
							placeholder={t("Nombre temático")}
							aria-label={t("Nombre en {idioma}", { idioma: nombre })}
							className="h-8 text-xs"
						/>
						{lang !== "en" && (
							<Input
								value={porIdioma(datos.typeLine, lang)}
								onChange={(e) => cambiar((d) => conTipo(d, cara, lang, e.target.value))}
								// Vacío no vale: la línea de tipo vuelve a la original en inglés.
								onBlur={(e) => {
									if (!e.target.value.trim())
										cambiar((d) => conTipo(d, cara, lang, datos.typeLine.en));
								}}
								placeholder={datos.typeLine.en}
								aria-label={t("Línea de tipo en {idioma}", { idioma: nombre })}
								className="h-8 text-xs"
							/>
						)}
						<AreaTexto
							valor={tx.flavor}
							onChange={(flavor) => cambiar((d) => conTextos(d, cara, lang, { flavor }))}
							placeholder={t("Texto de ambientación (opcional)")}
							ariaLabel={t("Ambientación en {idioma}", { idioma: nombre })}
						/>
						<div className="flex items-center justify-between">
							<span className="text-[11px] text-muted-foreground">{t("Texto de reglas")}</span>
							{aMano ? (
								<Button
									size="xs"
									variant="ghost"
									onClick={() =>
										cambiar((d) => conTextos(d, cara, lang, { rulesOverride: undefined }))
									}
								>
									<RotateCcw />
									{t("Restaurar el oficial")}
								</Button>
							) : (
								<Button
									size="xs"
									variant="ghost"
									onClick={() =>
										cambiar((d) =>
											conTextos(d, cara, lang, { rulesOverride: porIdioma(datos.rules, lang) }),
										)
									}
								>
									{t("Escribir a mano")}
								</Button>
							)}
						</div>
						{aMano ? (
							<>
								<AreaTexto
									valor={tx.rulesOverride ?? ""}
									onChange={(v) => cambiar((d) => conTextos(d, cara, lang, { rulesOverride: v }))}
									filas={5}
									ariaLabel={t("Reglas en {idioma}", { idioma: nombre })}
								/>
								<p className="text-[11px] text-amber-700 dark:text-amber-400">
									{t("Reglas escritas a mano: cuida que digan exactamente lo mismo que la carta.")}
								</p>
								<p className="text-[11px] text-muted-foreground">
									{t(
										"[[así]] resalta unas palabras con el color del pack (si lo trae, como el de Balatro); sin él, salen normales.",
									)}
								</p>
							</>
						) : (
							<p className="whitespace-pre-line rounded-md bg-muted/50 px-2.5 py-1.5 text-[11px] text-muted-foreground">
								{porIdioma(datos.rules, lang) || t("(sin texto)")}
							</p>
						)}
					</div>
				);
			})}
			<TraduccionSugerida design={design} cara={cara} cambiar={cambiar} />
			<p className="text-[11px] text-muted-foreground">
				{t(
					"En las reglas, el nombre original se cambia solo por el tuyo («Si Veldanava, Dragón de la Creación fuera a ir…»).",
				)}
			</p>
			<FilaInterruptor
				etiqueta={t("Poner el texto de ambientación")}
				valor={design.options.showFlavor}
				onChange={(v) => cambiar((d) => ({ ...d, options: { ...d.options, showFlavor: v } }))}
			/>
		</div>
	);
}

// ── Arte ───────────────────────────────────────────────────────────────────

function PestanaArte({
	design,
	cambiar,
	cara,
}: {
	design: CardDesign;
	cambiar: Cambiar;
	cara: number;
}) {
	const { t, mensaje } = useLang();
	const [ocupado, setOcupado] = useState(false);
	const [recortar, setRecortar] = useState(false);
	const arte = design.faces[cara]?.art;
	const poner = (a: ArtRef) => cambiar((d) => conArte(d, cara, a));
	const intentar = async (f: () => Promise<ArtRef | null>) => {
		setOcupado(true);
		try {
			const a = await f();
			if (a) poner(a);
		} catch (e) {
			toast.error(mensaje(e));
		} finally {
			setOcupado(false);
		}
	};
	const subir = () =>
		intentar(async () => {
			const r = await abrirParaImportar({
				multiple: false,
				filters: [{ name: t("Imágenes"), extensions: EXTENSIONES }],
			});
			if (!r?.[0]) return null;
			const [img] = await call("blob_import_files", { paths: [r[0]] });
			return img ? call("art_from_blob", { blob: img.blob }) : null;
		});
	const pegar = () =>
		intentar(async () => {
			// El portapapeles lo lee Rust: WebKitGTK no deja usar `navigator.clipboard.read()`.
			const a = await call("art_from_clipboard", {});
			if (!a) toast.info(t("No hay ninguna imagen en el portapapeles."));
			return a;
		});
	const scryfall = () =>
		intentar(() => call("art_from_scryfall", { scryfallId: design.card.scryfallId, cara }));
	const encuadre = (c: Partial<ArtRef["framing"]>) =>
		arte && poner({ ...arte, framing: { ...arte.framing, ...c } });
	const origen =
		arte?.source.kind === "scryfallArtCrop"
			? t("Ilustración de Scryfall")
			: arte?.source.kind === "cardCrop"
				? t("Recortada de una carta")
				: t("Imagen subida");

	return (
		<div className="flex flex-col gap-3">
			<div className="grid grid-cols-2 gap-1.5">
				<Button size="sm" variant="outline" onClick={subir} disabled={ocupado}>
					<Upload />
					{t("Subir imagen…")}
				</Button>
				<Button size="sm" variant="outline" onClick={pegar} disabled={ocupado}>
					<Clipboard />
					{t("Pegar")}
				</Button>
				<Button size="sm" variant="outline" onClick={scryfall} disabled={ocupado}>
					<Sparkles />
					{t("Arte de Scryfall")}
				</Button>
				<Button size="sm" variant="outline" onClick={() => setRecortar(true)} disabled={ocupado}>
					<Crop />
					{t("Recortar de una carta…")}
				</Button>
			</div>
			{arte ? (
				<>
					<div className="flex items-center gap-2 text-[11px] text-muted-foreground">
						<ImagePlus className="size-3.5" />
						<span className="flex-1">
							{origen} · {arte.image.width}×{arte.image.height}
						</span>
						<Button
							size="xs"
							variant="ghost"
							onClick={() => cambiar((d) => conArte(d, cara, undefined))}
							aria-label={t("Quitar la ilustración")}
						>
							<Trash2 />
						</Button>
					</div>
					{arte.image.width < 900 && (
						<p className="text-[11px] text-amber-700 dark:text-amber-400">
							{t(
								"Esta imagen es pequeña para imprimir con nitidez (mejor 1200 px de ancho o más).",
							)}
						</p>
					)}
					<FilaDeslizador
						etiqueta={t("Zoom")}
						valor={arte.framing.zoom}
						min={ZOOM_MIN}
						max={ZOOM_MAX}
						paso={0.01}
						sufijo="×"
						onChange={(zoom) => encuadre({ zoom })}
					/>
					<FilaDeslizador
						etiqueta={t("Encuadre horizontal")}
						valor={arte.framing.offsetX}
						min={-1}
						max={1}
						paso={0.01}
						onChange={(offsetX) => encuadre({ offsetX })}
					/>
					<FilaDeslizador
						etiqueta={t("Encuadre vertical")}
						valor={arte.framing.offsetY}
						min={-1}
						max={1}
						paso={0.01}
						onChange={(offsetY) => encuadre({ offsetY })}
					/>
					<FilaDeslizador
						etiqueta={t("Giro")}
						valor={arte.framing.rotation}
						min={-180}
						max={180}
						paso={0.5}
						sufijo="°"
						onChange={(rotation) => encuadre({ rotation })}
					/>
					<Button
						size="sm"
						variant="ghost"
						className="self-start"
						onClick={() => encuadre({ zoom: 1, offsetX: 0, offsetY: 0, rotation: 0 })}
					>
						<RotateCcw />
						{t("Encuadre inicial")}
					</Button>
				</>
			) : (
				<p className="text-xs text-muted-foreground">
					{t(
						"Sin ilustración. Sube una, pégala, usa la de Scryfall o recórtala de una carta ya hecha.",
					)}
				</p>
			)}
			<RecortarDialog
				abierto={recortar}
				onCerrar={() => setRecortar(false)}
				onRecortado={(a) => {
					poner(a);
					setRecortar(false);
				}}
			/>
		</div>
	);
}

// ── Marco ──────────────────────────────────────────────────────────────────

const FABRICA: FrameOptions = {
	pack: "builtin",
	color: { kind: "auto" },
	crown: "auto",
	rulesBox: { kind: "auto" },
};

export function EditorMarco({
	marco,
	onChange,
	packs,
}: {
	marco: FrameOptions;
	onChange: (m: FrameOptions) => void;
	packs: PackInfo[];
}) {
	const { t, mensaje } = useLang();
	const paletas = PALETAS.map((p) => ({ valor: p.valor, etiqueta: t(p.etiqueta) }));
	const color = marco.color;
	const originales = useMarcosOriginales();
	useEffect(() => {
		if (!originales.info) void originales.cargar();
	}, [originales.info, originales.cargar]);
	const hayOriginal = packs.some((p) => p.id === ID_ORIGINAL);
	const opciones = packs.map((p) => ({
		valor: p.id,
		etiqueta: p.importado ? p.nombre : t("Proxy Builder (de la casa)"),
	}));
	// Los originales se ofrecen siempre; si no están, elegirlos los descarga.
	if (!hayOriginal) opciones.push({ valor: ID_ORIGINAL, etiqueta: t("Original (descargar)") });
	// Y los del repositorio de packs (Balatro), igual.
	const descargables = usePacksDescargables();
	useEffect(() => {
		if (!descargables.lista) void descargables.cargar();
	}, [descargables.lista, descargables.cargar]);
	const porBajar = (descargables.lista ?? []).filter((d) => !packs.some((p) => p.id === d.id));
	for (const d of porBajar) {
		opciones.push({ valor: d.id, etiqueta: t("{nombre} (descargar)", { nombre: d.nombre }) });
	}
	const elegirPack = (pack: string) => {
		if (porBajar.some((d) => d.id === pack)) {
			descargables
				.descargar(pack)
				.then(() => onChange({ ...marco, pack }))
				.catch((e) => toast.error(mensaje(e)));
			return;
		}
		if (pack !== ID_ORIGINAL || hayOriginal) return onChange({ ...marco, pack });
		originales
			.descargar()
			.then(() => onChange({ ...marco, pack }))
			.catch((e) => toast.error(mensaje(e)));
	};
	return (
		<div className="flex flex-col gap-2.5">
			<Fila etiqueta={t("Pack de marcos")}>
				<Selector valor={marco.pack} opciones={opciones} onChange={elegirPack} ancho="w-52" />
			</Fila>
			{descargables.progreso && (
				<div className="flex flex-col gap-1.5">
					<Progress
						value={(descargables.progreso.hechas * 100) / Math.max(1, descargables.progreso.total)}
					/>
					<span className="text-[11px] text-muted-foreground">
						{t("Descargando el pack: {hechas} de {total}", {
							hechas: descargables.progreso.hechas,
							total: descargables.progreso.total,
						})}
					</span>
				</div>
			)}
			{originales.progreso ? (
				<div className="flex flex-col gap-1.5">
					<Progress
						value={(originales.progreso.hechas * 100) / Math.max(1, originales.progreso.total)}
					/>
					<span className="text-[11px] text-muted-foreground">
						{t("Descargando los marcos originales: {hechas} de {total}", {
							hechas: originales.progreso.hechas,
							total: originales.progreso.total,
						})}
					</span>
				</div>
			) : (
				marco.pack === ID_ORIGINAL && (
					<p className="text-[11px] text-muted-foreground">
						{t(
							"Marcos de imagen de Card Conjurer para los estilos Despejado, Extendido y Clásico; el paisaje usa el de la casa.",
						)}
					</p>
				)
			)}
			<Fila
				etiqueta={t("Color del marco")}
				ayuda={t("Automático: un color, el suyo; dos colores, degradado; tres o más, oro.")}
			>
				<Selector
					valor={color.kind}
					opciones={[
						{ valor: "auto", etiqueta: t("Automático") },
						{ valor: "palette", etiqueta: t("Un color") },
						{ valor: "gradient", etiqueta: t("Degradado") },
						{ valor: "custom", etiqueta: t("Personalizado") },
					]}
					onChange={(k) =>
						onChange({
							...marco,
							color:
								k === "palette"
									? { kind: "palette", palette: "gold" }
									: k === "gradient"
										? { kind: "gradient", a: "G", b: "U" }
										: k === "custom"
											? { kind: "custom", hex: "#7a4fd0" }
											: { kind: "auto" },
						})
					}
					ancho="w-44"
				/>
			</Fila>
			{color.kind === "palette" && (
				<Fila etiqueta={t("Paleta")}>
					<Selector
						valor={color.palette}
						opciones={paletas}
						onChange={(palette) => onChange({ ...marco, color: { kind: "palette", palette } })}
						ancho="w-44"
					/>
				</Fila>
			)}
			{color.kind === "gradient" && (
				<>
					<Fila etiqueta={t("De")}>
						<Selector
							valor={color.a}
							opciones={paletas}
							onChange={(a) => onChange({ ...marco, color: { ...color, a } })}
							ancho="w-44"
						/>
					</Fila>
					<Fila etiqueta={t("A")}>
						<Selector
							valor={color.b}
							opciones={paletas}
							onChange={(b) => onChange({ ...marco, color: { ...color, b } })}
							ancho="w-44"
						/>
					</Fila>
				</>
			)}
			{color.kind === "custom" && (
				<Fila etiqueta={t("Color")}>
					<input
						type="color"
						value={color.hex}
						onChange={(e) => onChange({ ...marco, color: { kind: "custom", hex: e.target.value } })}
						aria-label={t("Color")}
						className="h-7 w-12 cursor-pointer rounded border bg-transparent"
					/>
				</Fila>
			)}
			<Fila
				etiqueta={t("Corona de legendaria")}
				ayuda={t("Automática: solo en el estilo despejado y en las legendarias.")}
			>
				<Selector
					valor={marco.crown}
					opciones={[
						{ valor: "auto", etiqueta: t("Automática") },
						{ valor: "on", etiqueta: t("Siempre") },
						{ valor: "off", etiqueta: t("Nunca") },
					]}
					onChange={(crown) => onChange({ ...marco, crown })}
					ancho="w-44"
				/>
			</Fila>
			<Fila etiqueta={t("Caja de reglas")} ayuda={t("Automática: teñida del color de la carta.")}>
				<Selector
					valor={marco.rulesBox.kind}
					opciones={[
						{ valor: "auto", etiqueta: t("Automática") },
						{ valor: "dark", etiqueta: t("Oscura") },
						{ valor: "light", etiqueta: t("Clara") },
						{ valor: "tint", etiqueta: t("Otro tinte") },
					]}
					onChange={(k) =>
						onChange({
							...marco,
							rulesBox: k === "tint" ? { kind: "tint", hex: "#2e2412" } : { kind: k },
						})
					}
					ancho="w-44"
				/>
			</Fila>
			{marco.rulesBox.kind === "tint" && (
				<Fila etiqueta={t("Tinte")}>
					<input
						type="color"
						value={marco.rulesBox.hex}
						onChange={(e) =>
							onChange({ ...marco, rulesBox: { kind: "tint", hex: e.target.value } })
						}
						aria-label={t("Tinte")}
						className="h-7 w-12 cursor-pointer rounded border bg-transparent"
					/>
				</Fila>
			)}
		</div>
	);
}

function PestanaMarco({
	design,
	cambiar,
	colecciones,
	packs,
}: {
	design: CardDesign;
	cambiar: Cambiar;
	colecciones: Collection[];
	packs: PackInfo[];
}) {
	const { t, mensaje } = useLang();
	const col = colecciones.find((c) => c.id === design.collection);
	const propio = design.frame !== undefined;
	const marco = design.frame ?? col?.frame ?? FABRICA;
	const emblema = design.emblem;
	const subirEmblema = async () => {
		try {
			const r = await abrirParaImportar({
				multiple: false,
				filters: [{ name: t("Imágenes"), extensions: ["png", "webp", "jpg", "jpeg"] }],
			});
			if (!r?.[0]) return;
			const [img] = await call("blob_import_files", { paths: [r[0]] });
			if (img) cambiar((d) => ({ ...d, emblem: { kind: "own", image: img } }));
		} catch (e) {
			toast.error(mensaje(e));
		}
	};
	const tipoEmblema = (e: EmblemChoice["kind"]) => {
		if (e === "own") void subirEmblema();
		else cambiar((d) => ({ ...d, emblem: { kind: e } as EmblemChoice }));
	};
	return (
		<div className="flex flex-col gap-3">
			<FilaInterruptor
				etiqueta={t("Marco propio para esta carta")}
				ayuda={t("Apagado, usa el de la colección: cambiarlo allí cambia todas sus cartas.")}
				valor={propio}
				onChange={(v) => cambiar((d) => ({ ...d, frame: v ? { ...marco } : undefined }))}
			/>
			<div className={cn(!propio && "pointer-events-none opacity-60")}>
				<EditorMarco
					marco={marco}
					onChange={(m) =>
						cambiar((d) => ({
							...d,
							frame: m,
							// Un estilo propio que el pack nuevo no tiene: al de serie.
							estiloPack: propiosDe(packs, m.pack).some((e) => e.id === d.estiloPack)
								? d.estiloPack
								: undefined,
						}))
					}
					packs={packs}
				/>
			</div>
			<div className="h-px bg-border" />
			<Fila
				etiqueta={t("Emblema")}
				ayuda={t(
					"El adorno a la derecha de la línea de tipo: el de la colección para que todas las cartas vayan iguales.",
				)}
			>
				<Selector
					valor={emblema.kind}
					opciones={[
						{ valor: "inherit", etiqueta: t("El de la colección") },
						{ valor: "none", etiqueta: t("Ninguno") },
						{ valor: "own", etiqueta: t("Uno propio…") },
					]}
					onChange={tipoEmblema}
					ancho="w-44"
				/>
			</Fila>
			{emblema.kind === "own" && (
				<Button size="sm" variant="outline" className="self-end" onClick={subirEmblema}>
					<Upload />
					{t("Cambiar emblema…")}
				</Button>
			)}
			<FilaInterruptor
				etiqueta={t("Estrechado del título propio")}
				ayuda={t(
					"Cuánto se estrechan el título y el tipo: siempre (el mínimo) y como mucho antes de reducir la letra (el límite).",
				)}
				valor={design.options.titleSqueezeMin !== undefined}
				onChange={(v) =>
					cambiar((d) => ({
						...d,
						options: {
							...d.options,
							titleSqueezeMin: v ? (col?.titleSqueezeMin ?? 0.65) : undefined,
							titleSqueezeBase: v ? (col?.titleSqueezeBase ?? 1) : undefined,
						},
					}))
				}
			/>
			{design.options.titleSqueezeMin !== undefined && (
				<>
					<FilaDeslizador
						etiqueta={t("Estrechar hasta")}
						valor={Math.round(design.options.titleSqueezeMin * 100)}
						min={55}
						max={75}
						sufijo="%"
						onChange={(v) =>
							cambiar((d) => ({
								...d,
								options: {
									...d.options,
									titleSqueezeMin: v / 100,
									titleSqueezeBase: Math.max(d.options.titleSqueezeBase ?? 1, v / 100),
								},
							}))
						}
					/>
					<FilaDeslizador
						etiqueta={t("Estrechar siempre al menos")}
						valor={Math.round((design.options.titleSqueezeBase ?? 1) * 100)}
						min={Math.round(design.options.titleSqueezeMin * 100)}
						max={100}
						sufijo="%"
						onChange={(v) =>
							cambiar((d) => ({ ...d, options: { ...d.options, titleSqueezeBase: v / 100 } }))
						}
					/>
				</>
			)}
		</div>
	);
}

// ── Ajustes finos ──────────────────────────────────────────────────────────

function PestanaAjustes({
	design,
	cambiar,
	colecciones,
	onColeccion,
}: {
	design: CardDesign;
	cambiar: Cambiar;
	colecciones: Collection[];
	onColeccion?: (c: Collection) => void;
}) {
	const { t, mensaje } = useLang();
	const col = colecciones.find((c) => c.id === design.collection);
	const propios = design.options.fineTuning;
	const poner = (fineTuning: FineTuning | undefined) =>
		cambiar((d) => ({ ...d, options: { ...d.options, fineTuning } }));
	const [guardando, setGuardando] = useState(false);
	// Los ajustes de esta carta pasan a ser los de su colección (y la carta
	// deja de llevar los suyos: sigue a la colección).
	const pasarAColeccion = async () => {
		if (!col || !propios) return;
		setGuardando(true);
		try {
			const guardada = await call("collection_save", {
				collection: { ...col, fineTuning: hayAjustes(propios) ? propios : undefined },
			});
			onColeccion?.(guardada);
			poner(undefined);
			void rehacerMiniaturas();
			useUi.getState().anunciar(t("Ajustes guardados en la colección"));
		} catch (e) {
			toast.error(mensaje(e));
		} finally {
			setGuardando(false);
		}
	};
	return (
		<div className="flex flex-col gap-2.5">
			<FilaInterruptor
				etiqueta={t("Ajustes propios para esta carta")}
				ayuda={
					col
						? t("Sin ellos, la carta usa los de su colección («{c}»).", { c: col.name })
						: t("Sin ellos, la carta va como viene en el marco.")
				}
				valor={propios !== undefined}
				onChange={(v) => poner(v ? (col?.fineTuning ?? AJUSTES_CERO) : undefined)}
			/>
			{propios !== undefined ? (
				<>
					<AjustesFinos valor={propios} onChange={poner} />
					{col && (
						<Button
							size="sm"
							variant="outline"
							className="self-start"
							disabled={guardando}
							onClick={pasarAColeccion}
						>
							<Save />
							{t("Usar estos ajustes en toda la colección «{c}»", { c: col.name })}
						</Button>
					)}
				</>
			) : (
				<p className="text-[11px] text-muted-foreground">
					{t("Para mover a la vez todas las cartas de una colección, ajústalo en «Colecciones».")}
				</p>
			)}
		</div>
	);
}

// ── Pie ────────────────────────────────────────────────────────────────────

function CampoTexto({
	etiqueta,
	valor,
	defecto,
	onChange,
}: {
	etiqueta: string;
	valor: string | undefined;
	defecto?: string;
	onChange: (v: string | undefined) => void;
}) {
	// Se aplica al escribir (la carta se repinta en tiempo real); vacío es «el
	// de la colección», que se ve en gris.
	return (
		<Fila etiqueta={etiqueta}>
			<Input
				value={valor ?? ""}
				placeholder={defecto}
				onChange={(e) => onChange(e.target.value.trim() ? e.target.value : undefined)}
				aria-label={etiqueta}
				className="h-7 w-44 text-xs"
			/>
		</Fila>
	);
}

function PestanaPie({
	design,
	cambiar,
	colecciones,
}: {
	design: CardDesign;
	cambiar: Cambiar;
	colecciones: Collection[];
}) {
	const { t } = useLang();
	const col = colecciones.find((c) => c.id === design.collection);
	const pie = design.footer;
	const poner = (c: Partial<FooterOptions>) =>
		cambiar((d) => ({ ...d, footer: { ...d.footer, ...c } }));
	return (
		<div className="flex flex-col gap-2.5">
			<Fila
				etiqueta={t("Número de colección")}
				ayuda={t(
					"Se pone solo por el orden del deck («Renumerar»); si lo escribes, se queda el tuyo.",
				)}
			>
				<Input
					value={pie.number}
					onChange={(e) => poner({ number: e.target.value, numberManual: true })}
					aria-label={t("Número de colección")}
					className="h-7 w-44 text-xs"
				/>
			</Fila>
			{pie.numberManual && (
				<Button
					size="xs"
					variant="ghost"
					className="self-end"
					onClick={() => poner({ numberManual: false })}
				>
					{t("Volver a numerar solo")}
				</Button>
			)}
			<CampoTexto
				etiqueta={t("Rareza (letra)")}
				valor={pie.rarity}
				defecto={col?.rarityLetter ?? letraRareza(design.card.rarity)}
				onChange={(rarity) => poner({ rarity })}
			/>
			<CampoTexto
				etiqueta={t("Código del set")}
				valor={pie.setCode}
				defecto={col?.setCode || design.card.set.toUpperCase()}
				onChange={(setCode) => poner({ setCode })}
			/>
			<CampoTexto
				etiqueta={t("Artista")}
				valor={pie.artist}
				defecto={col?.artist ?? design.card.artist ?? undefined}
				onChange={(artist) => poner({ artist })}
			/>
			<CampoTexto
				etiqueta={t("Créditos")}
				valor={pie.credit}
				defecto={col?.credit ?? undefined}
				onChange={(credit) => poner({ credit })}
			/>
			<CampoTexto
				etiqueta={t("Universo")}
				valor={pie.universe}
				defecto={col?.universe ?? undefined}
				onChange={(universe) => poner({ universe })}
			/>
			<CampoTexto
				etiqueta={t("Año")}
				valor={pie.year}
				defecto={col?.year ?? undefined}
				onChange={(year) => poner({ year })}
			/>
			<Fila etiqueta={t("Nombre original")}>
				<Selector
					valor={design.options.originalName}
					opciones={NOMBRE_ORIGINAL.map((o) => ({ valor: o.valor, etiqueta: t(o.etiqueta) }))}
					onChange={(originalName) =>
						cambiar((d) => ({ ...d, options: { ...d.options, originalName } }))
					}
					ancho="w-52"
				/>
			</Fila>
			<p className="text-[11px] text-muted-foreground">
				{t("Lo que dejes vacío sale de la colección; la forma del pie se cambia en la colección.")}
			</p>
		</div>
	);
}
