import { Loader2, Plus, Save, Trash2, Upload, X } from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { toast } from "sonner";
import type { Collection } from "@/bindings/Collection";
import type { LetraInfo } from "@/bindings/LetraInfo";
import type { PackInfo } from "@/bindings/PackInfo";
import { Fila, FilaDeslizador, Selector } from "@/components/campos";
import { Confirmar } from "@/components/confirmar";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import { abrirParaImportar } from "@/lib/dialogos";
import { EDICION_NEUTRA, parametros, urlMiniatura } from "@/lib/image-url";
import { call } from "@/lib/ipc";
import { cn } from "@/lib/utils";
import { useDeck } from "@/stores/deck-store";
import { useUi } from "@/stores/ui-store";
import { EditorMarco } from "./formulario";
import { rehacerMiniaturas } from "./renumerar";
import { ESTILOS, NOMBRE_ORIGINAL } from "./textos";

/** La plantilla del pie de fábrica (la misma que `PIE_DEFECTO` en Rust). */
const PIE_DEFECTO = "{num}\t{rarity}\t{original}\n{set} • {lang}\t{brush} {artist}";

/** Las letras de la casa (siempre están). */
const LETRAS_DE_CASA = ["Spectral", "Montserrat"];

function nueva(nombre: string): Collection {
	return {
		id: "",
		name: nombre,
		defaultStyle: "borderless",
		frame: { pack: "builtin", color: { kind: "auto" }, crown: "auto", rulesBox: { kind: "auto" } },
		setCode: "",
		originalName: "auto",
		footerTemplate: { es: "", en: "" },
		titleSqueezeMin: 0.65,
	};
}

/**
 * **Colecciones**: lo que comparten las cartas de un mazo o una serie (marco,
 * emblema, pie, letras). Cambiar una colección cambia todas sus cartas.
 */
export function ColeccionesDialog() {
	const abierto = useUi((s) => s.dialogo?.tipo === "colecciones");
	return (
		<Dialog open={abierto} onOpenChange={(o) => !o && useUi.getState().abrir(null)}>
			{abierto && <Contenido />}
		</Dialog>
	);
}

function Contenido() {
	const { t, mensaje } = useLang();
	const [lista, setLista] = useState<Collection[] | null>(null);
	const [packs, setPacks] = useState<PackInfo[]>([]);
	const [letras, setLetras] = useState<LetraInfo[]>([]);
	const [elegida, setElegida] = useState<string | null>(null);
	const [borrador, setBorrador] = useState<Collection | null>(null);
	const [guardando, setGuardando] = useState(false);
	const [borrar, setBorrar] = useState(false);
	const [tocadas, setTocadas] = useState(false);

	const recargar = useCallback(async () => {
		try {
			const cs = await call("collection_list", {});
			setLista(cs);
			return cs;
		} catch (e) {
			toast.error(mensaje(e));
			setLista([]);
			return [];
		}
	}, [mensaje]);

	useEffect(() => {
		void recargar().then((cs) => {
			const primera = cs[0];
			if (primera) {
				setElegida(primera.id);
				setBorrador(primera);
			}
		});
		call("packs_list", {})
			.then(setPacks)
			.catch(() => {});
		call("fonts_list", {})
			.then(setLetras)
			.catch(() => {});
	}, [recargar]);

	// Al cerrar, si se guardó algo, se rehacen las miniaturas viejas.
	useEffect(() => {
		return () => {
			if (tocadas) void rehacerMiniaturas();
		};
	}, [tocadas]);

	const original = lista?.find((c) => c.id === elegida) ?? null;
	const cambiada = !!borrador && JSON.stringify(borrador) !== JSON.stringify(original ?? null);

	const elegir = (c: Collection) => {
		setElegida(c.id);
		setBorrador(c);
	};
	const crear = () => {
		setElegida("");
		setBorrador(nueva(t("Nueva colección")));
	};

	const guardar = async () => {
		if (!borrador?.name.trim()) return;
		setGuardando(true);
		try {
			const guardada = await call("collection_save", {
				collection: { ...borrador, name: borrador.name.trim() },
			});
			await recargar();
			setElegida(guardada.id);
			setBorrador(guardada);
			setTocadas(true);
			useUi.getState().anunciar(t("Colección guardada"));
		} catch (e) {
			toast.error(mensaje(e));
		} finally {
			setGuardando(false);
		}
	};

	const quitar = async () => {
		if (!original) return;
		try {
			await call("collection_delete", { id: original.id });
			const cs = await recargar();
			const primera = cs[0] ?? null;
			setElegida(primera?.id ?? null);
			setBorrador(primera);
			setTocadas(true);
		} catch (e) {
			toast.error(mensaje(e));
		}
	};

	return (
		<DialogContent
			className="flex h-[min(86vh,760px)] flex-col sm:max-w-4xl"
			cambiosSinGuardar={cambiada}
		>
			<DialogHeader>
				<DialogTitle>{t("Colecciones")}</DialogTitle>
				<DialogDescription className="text-xs">
					{t(
						"Lo que comparten las cartas de un mazo o una serie: marco, emblema, pie y letras. Cambiar una colección cambia todas sus cartas.",
					)}
				</DialogDescription>
			</DialogHeader>
			<div className="flex min-h-0 flex-1 gap-2.5">
				<div className="flex w-56 shrink-0 flex-col gap-2.5 rounded-md border p-2.5">
					<Button size="sm" variant="outline" onClick={crear}>
						<Plus />
						{t("Nueva colección")}
					</Button>
					<ScrollArea className="min-h-0 flex-1">
						<ul className="flex flex-col gap-1 pr-2">
							{lista === null && (
								<Loader2 className="mx-auto size-4 animate-spin text-muted-foreground" />
							)}
							{lista?.length === 0 && elegida !== "" && (
								<li className="text-xs text-muted-foreground">
									{t("Todavía no hay colecciones.")}
								</li>
							)}
							{elegida === "" && (
								<li className="rounded-md bg-muted px-2 py-1.5 text-xs font-medium italic ring-1 ring-primary/50">
									{borrador?.name || t("Nueva colección")}
								</li>
							)}
							{lista?.map((c) => (
								<li key={c.id}>
									<button
										type="button"
										onClick={() => elegir(c)}
										className={cn(
											"w-full truncate rounded-md px-2 py-1.5 text-left text-xs hover:bg-muted",
											elegida === c.id && "bg-muted font-medium ring-1 ring-primary/50",
										)}
									>
										{c.name}
									</button>
								</li>
							))}
						</ul>
					</ScrollArea>
				</div>
				<div className="flex min-w-0 flex-1 flex-col rounded-md border">
					{borrador ? (
						<>
							<ScrollArea className="min-h-0 flex-1">
								<Editor
									c={borrador}
									onChange={setBorrador}
									packs={packs}
									familias={familiasDe(letras)}
								/>
							</ScrollArea>
							<div className="flex shrink-0 items-center gap-1.5 border-t p-2.5">
								{original && (
									<Button size="sm" variant="ghost" onClick={() => setBorrar(true)}>
										<Trash2 />
										{t("Borrar")}
									</Button>
								)}
								<div className="ml-auto flex gap-1.5">
									<Button
										size="sm"
										variant="ghost"
										disabled={!cambiada}
										onClick={() => (original ? setBorrador(original) : crear())}
									>
										{t("Descartar")}
									</Button>
									<Button
										size="sm"
										onClick={guardar}
										disabled={!cambiada || guardando || !borrador.name.trim()}
									>
										{guardando ? <Loader2 className="animate-spin" /> : <Save />}
										{t("Guardar")}
									</Button>
								</div>
							</div>
						</>
					) : (
						<div className="flex flex-1 items-center justify-center p-6 text-center text-xs text-muted-foreground">
							{t("Crea una colección para compartir marco, emblema y pie entre cartas.")}
						</div>
					)}
				</div>
			</div>
			<Confirmar
				abierto={borrar}
				onCerrar={() => setBorrar(false)}
				titulo={t("¿Borrar la colección «{nombre}»?", { nombre: original?.name ?? "" })}
				descripcion={t(
					"Sus cartas se quedan con su diseño, pero vuelven al marco y al pie de fábrica.",
				)}
				accion={t("Borrar")}
				destructiva
				onConfirmar={quitar}
			/>
		</DialogContent>
	);
}

function familiasDe(letras: LetraInfo[]): string[] {
	const todas = new Set(LETRAS_DE_CASA);
	for (const l of letras) for (const f of l.familias) todas.add(f);
	return [...todas].sort((a, b) => a.localeCompare(b));
}

/** Un texto opcional: vacío es «sin valor». */
function CampoOpcional({
	etiqueta,
	valor,
	onChange,
	placeholder,
	ancho = "w-52",
}: {
	etiqueta: string;
	valor: string | undefined;
	onChange: (v: string | undefined) => void;
	placeholder?: string;
	ancho?: string;
}) {
	return (
		<Fila etiqueta={etiqueta}>
			<Input
				value={valor ?? ""}
				placeholder={placeholder}
				onChange={(e) => onChange(e.target.value === "" ? undefined : e.target.value)}
				aria-label={etiqueta}
				className={cn("h-7 text-xs", ancho)}
			/>
		</Fila>
	);
}

function Titulo({ children }: { children: React.ReactNode }) {
	return (
		<h3 className="pt-1 text-[11px] font-semibold tracking-wide text-muted-foreground uppercase">
			{children}
		</h3>
	);
}

function Editor({
	c,
	onChange,
	packs,
	familias,
}: {
	c: Collection;
	onChange: (c: Collection) => void;
	packs: PackInfo[];
	familias: string[];
}) {
	const { t, mensaje } = useLang();
	const ajustes = useDeck((s) => s.deck?.settings);
	const poner = (p: Partial<Collection>) => onChange({ ...c, ...p });
	const opcionesLetra = useMemo(
		() => [
			{ valor: "__pack", etiqueta: t("La del pack") },
			...familias.map((f) => ({ valor: f, etiqueta: f })),
		],
		[familias, t],
	);

	const subirEmblema = async () => {
		try {
			const r = await abrirParaImportar({
				multiple: false,
				filters: [{ name: t("Imágenes"), extensions: ["png", "webp", "jpg", "jpeg"] }],
			});
			if (!r?.[0]) return;
			const [img] = await call("blob_import_files", { paths: [r[0]] });
			if (img) poner({ emblem: img });
		} catch (e) {
			toast.error(mensaje(e));
		}
	};

	return (
		<div className="flex flex-col gap-2.5 p-3 pr-4">
			<Titulo>{t("General")}</Titulo>
			<Fila etiqueta={t("Nombre")}>
				<Input
					value={c.name}
					onChange={(e) => poner({ name: e.target.value })}
					aria-label={t("Nombre")}
					className="h-7 w-52 text-xs"
				/>
			</Fila>
			<Fila etiqueta={t("Estilo de las cartas nuevas")}>
				<Selector
					valor={c.defaultStyle}
					opciones={ESTILOS.map((e) => ({ valor: e.valor, etiqueta: t(e.etiqueta) }))}
					onChange={(defaultStyle) => poner({ defaultStyle })}
					ancho="w-52"
				/>
			</Fila>

			<Titulo>{t("Marco")}</Titulo>
			<EditorMarco marco={c.frame} onChange={(frame) => poner({ frame })} packs={packs} />

			<Titulo>{t("Emblema")}</Titulo>
			<Fila
				etiqueta={t("Emblema de la colección")}
				ayuda={t("El adorno a la derecha de la línea de tipo, igual en todas sus cartas.")}
			>
				{c.emblem && ajustes && (
					<img
						src={urlMiniatura(c.emblem.blob, parametros(ajustes, EDICION_NEUTRA, false, 0), 146)}
						alt=""
						className="size-7 rounded border bg-muted object-contain"
					/>
				)}
				<Button size="sm" variant="outline" onClick={subirEmblema}>
					<Upload />
					{c.emblem ? t("Cambiar…") : t("Subir…")}
				</Button>
				{c.emblem && (
					<Button
						size="sm"
						variant="ghost"
						onClick={() => poner({ emblem: undefined })}
						aria-label={t("Quitar emblema")}
					>
						<X />
					</Button>
				)}
			</Fila>

			<Titulo>{t("Letras")}</Titulo>
			<Fila etiqueta={t("Letra del título")}>
				<Selector
					valor={c.fontTitle ?? "__pack"}
					opciones={opcionesLetra}
					onChange={(v) => poner({ fontTitle: v === "__pack" ? undefined : v })}
					ancho="w-52"
				/>
			</Fila>
			<Fila etiqueta={t("Letra de las reglas")}>
				<Selector
					valor={c.fontRules ?? "__pack"}
					opciones={opcionesLetra}
					onChange={(v) => poner({ fontRules: v === "__pack" ? undefined : v })}
					ancho="w-52"
				/>
			</Fila>
			<FilaDeslizador
				etiqueta={t("Estrechar los títulos largos hasta")}
				valor={Math.round(c.titleSqueezeMin * 100)}
				min={55}
				max={75}
				sufijo="%"
				onChange={(v) => poner({ titleSqueezeMin: v / 100 })}
			/>

			<Titulo>{t("Pie")}</Titulo>
			<Fila etiqueta={t("Nombre original")}>
				<Selector
					valor={c.originalName}
					opciones={NOMBRE_ORIGINAL.map((o) => ({
						valor: o.valor,
						etiqueta: o.valor === "auto" ? t("Automático (en el pie)") : t(o.etiqueta),
					}))}
					onChange={(originalName) => poner({ originalName })}
					ancho="w-52"
				/>
			</Fila>
			<CampoOpcional
				etiqueta={t("Código del set")}
				valor={c.setCode || undefined}
				placeholder={t("El de la carta")}
				onChange={(v) => poner({ setCode: (v ?? "").toUpperCase() })}
			/>
			<CampoOpcional
				etiqueta={t("Rareza (letra)")}
				valor={c.rarityLetter}
				placeholder={t("La de la carta")}
				onChange={(rarityLetter) => poner({ rarityLetter })}
			/>
			<CampoOpcional
				etiqueta={t("Artista")}
				valor={c.artist}
				placeholder={t("El de la carta")}
				onChange={(artist) => poner({ artist })}
			/>
			<CampoOpcional
				etiqueta={t("Créditos")}
				valor={c.credit}
				onChange={(credit) => poner({ credit })}
			/>
			<CampoOpcional
				etiqueta={t("Universo")}
				valor={c.universe}
				onChange={(universe) => poner({ universe })}
			/>
			<CampoOpcional
				etiqueta={t("Año")}
				valor={c.year}
				onChange={(year) => poner({ year })}
				ancho="w-24"
			/>
			<PlantillaPie
				etiqueta={t("Plantilla del pie en español")}
				valor={c.footerTemplate.es}
				onChange={(es) => poner({ footerTemplate: { ...c.footerTemplate, es } })}
			/>
			<PlantillaPie
				etiqueta={t("Plantilla del pie en inglés")}
				valor={c.footerTemplate.en}
				onChange={(en) => poner({ footerTemplate: { ...c.footerTemplate, en } })}
			/>
			<p className="text-[11px] leading-relaxed text-muted-foreground">
				{t(
					"Fichas: {num} {rarity} {original} {set} {lang} {artist} {credit} {universe} {year} {brush} {star}. Un tabulador separa columnas; una línea nueva, filas. Vacía, la de fábrica.",
				)}
			</p>
		</div>
	);
}

/** La plantilla se escribe con «\t» visible para poder teclear columnas. */
function PlantillaPie({
	etiqueta,
	valor,
	onChange,
}: {
	etiqueta: string;
	valor: string;
	onChange: (v: string) => void;
}) {
	const visible = (s: string) => s.replaceAll("\t", "\\t");
	return (
		<div className="flex flex-col gap-1.5">
			<span className="text-xs text-foreground/90">{etiqueta}</span>
			<textarea
				value={visible(valor)}
				placeholder={visible(PIE_DEFECTO)}
				onChange={(e) => onChange(e.target.value.replaceAll("\\t", "\t"))}
				aria-label={etiqueta}
				rows={2}
				className="w-full resize-none rounded-md border border-input bg-transparent px-2.5 py-1.5 font-mono text-[11px] shadow-xs outline-none placeholder:text-muted-foreground focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/50"
			/>
		</div>
	);
}
