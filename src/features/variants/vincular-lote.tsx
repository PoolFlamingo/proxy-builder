import { Link2, Loader2, ScanText, X } from "lucide-react";
import { memo, useMemo, useRef, useState } from "react";
import { toast } from "sonner";
import type { CardIdentify } from "@/bindings/CardIdentify";
import type { Confidence } from "@/bindings/Confidence";
import type { DeckCard } from "@/bindings/DeckCard";
import { type Opcion, Selector } from "@/components/campos";
import { paraTraducir, useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
	Dialog,
	DialogContent,
	DialogCuerpo,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { Progress } from "@/components/ui/progress";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { RaizScroll, useVisible } from "@/features/preview/visibilidad";
import { EDICION_NEUTRA, parametros, urlBusqueda, urlMiniatura } from "@/lib/image-url";
import { call } from "@/lib/ipc";
import { lanzarTrabajo } from "@/lib/trabajos";
import { cn } from "@/lib/utils";
import { useDeck } from "@/stores/deck-store";
import { useUi } from "@/stores/ui-store";
import { estaVinculada, propiasPorImagen, vincular } from "./vincular";

/**
 * **Vincular con Scryfall en bloque.** Rust lee (nombre del fichero y, si se
 * pide, OCR de la carta) y busca cada imagen propia; aquí se revisa lo que ha
 * propuesto y se vinculan de una vez las marcadas. Las exactas y probables
 * vienen marcadas; el resto se corrige en la lista o se deja sin vincular.
 *
 * Con cartas seleccionadas deja elegir entre ellas y todo el deck (de
 * partida, lo que diga quien lo abre: el menú de varias cartas, la
 * selección; el botón del panel, todo el deck salvo que haya varias
 * seleccionadas). Las copias con la misma imagen van juntas (una fila por
 * imagen).
 */
export function VincularLoteDialog() {
	const dialogo = useUi((s) => s.dialogo);
	const abierto = dialogo?.tipo === "vincularLote";
	const ids = dialogo?.tipo === "vincularLote" ? dialogo.ids : undefined;
	const soloSeleccion = dialogo?.tipo === "vincularLote" && !!dialogo.soloSeleccion;
	return (
		<Dialog open={abierto} onOpenChange={(o) => !o && useUi.getState().abrir(null)}>
			{abierto && <Contenido ids={ids} soloSeleccion={soloSeleccion} />}
		</Dialog>
	);
}

type Fila = {
	carta: DeckCard;
	resultado: CardIdentify | null;
	elegida: string | null;
	marcada: boolean;
};

type Fase = "elegir" | "leyendo" | "revisar" | "aplicando";

const NINGUNA = "__ninguna";

const CONFIANZA: Record<Confidence, { texto: string; clase: string }> = {
	exact: { texto: paraTraducir("Exacta"), clase: "bg-green-600/90 text-white" },
	probable: { texto: paraTraducir("Probable"), clase: "bg-amber-500/90 text-black" },
	none: { texto: paraTraducir("Sin propuesta"), clase: "bg-muted text-muted-foreground" },
};

/** «~45 s», «~3 min». */
function restante(
	segundos: number,
	t: (es: string, v?: Record<string, string | number>) => string,
) {
	if (!Number.isFinite(segundos) || segundos <= 0) return "";
	if (segundos < 60) return t("quedan ~{n} s", { n: Math.max(1, Math.round(segundos)) });
	return t("quedan ~{n} min", { n: Math.round(segundos / 60) });
}

function Contenido({ ids, soloSeleccion }: { ids?: string[]; soloSeleccion: boolean }) {
	const { t, mensaje } = useLang();
	const haySeleccion = !!ids && ids.length > 0;
	const [enSeleccion, setEnSeleccion] = useState(haySeleccion && soloSeleccion);
	const delDeck = useMemo(() => propiasPorImagen(), []);
	const deSeleccion = useMemo(() => (ids ? propiasPorImagen(ids) : []), [ids]);
	const propias = enSeleccion ? deSeleccion : delDeck;
	const faltan = useMemo(() => propias.filter((c) => !estaVinculada(c)), [propias]);
	const [eleccion, setAlcance] = useState<"faltan" | "todas">("faltan");
	// «Solo las que faltan» sin ninguna que falte no tiene sentido.
	const alcance = eleccion === "faltan" && faltan.length === 0 ? "todas" : eleccion;
	const [ocr, setOcr] = useState(true);
	const [fase, setFase] = useState<Fase>("elegir");
	const [filas, setFilas] = useState<Fila[]>([]);
	const [progreso, setProgreso] = useState({ hechas: 0, total: 0 });
	const [filtro, setFiltro] = useState<"todas" | "revisar">("todas");
	const [viewport, setViewport] = useState<HTMLDivElement | null>(null);
	const inicio = useRef(0);
	const jobId = useRef<number | null>(null);
	const parar = useRef(false);

	const elegidas = alcance === "faltan" ? faltan : propias;

	const empezar = async () => {
		const porBlob = new Map(elegidas.map((c) => [c.front?.blob ?? "", c]));
		setFilas([]);
		setProgreso({ hechas: 0, total: elegidas.length });
		setFase("leyendo");
		inicio.current = performance.now();
		try {
			const { jobId: id, fin } = await lanzarTrabajo(
				"identify://progress",
				"identify://done",
				() =>
					call("identify_batch", {
						items: elegidas.flatMap((c) =>
							c.front ? [{ blob: c.front.blob, hint: c.meta.name || null }] : [],
						),
						ocr,
					}),
				(p) => {
					setProgreso({ hechas: p.done, total: p.total });
					const carta = porBlob.get(p.blob);
					if (!carta) return;
					const r = p.result;
					const segura = r?.confidence === "exact" || r?.confidence === "probable";
					setFilas((fs) => [
						...fs,
						{ carta, resultado: r, elegida: r?.best ?? null, marcada: !!(segura && r?.best) },
					]);
				},
			);
			jobId.current = id;
			const d = await fin;
			if (d.error && d.error.code !== "Cancelled") toast.error(mensaje(d.error));
		} catch (e) {
			toast.error(mensaje(e));
		} finally {
			jobId.current = null;
			setFase("revisar");
		}
	};

	const cancelarLectura = () => {
		if (jobId.current !== null) void call("job_cancel", { jobId: jobId.current });
	};

	const aVincular = filas.filter((f) => f.marcada && f.elegida);
	const aplicar = async () => {
		setFase("aplicando");
		parar.current = false;
		setProgreso({ hechas: 0, total: aVincular.length });
		let hechas = 0;
		try {
			for (const f of aVincular) {
				if (parar.current) break;
				if (f.elegida) await vincular(f.carta.id, f.elegida);
				hechas++;
				setProgreso({ hechas, total: aVincular.length });
			}
			useUi.getState().anunciar(t("{n} cartas propias vinculadas con Scryfall", { n: hechas }));
			useUi.getState().abrir(null);
		} catch (e) {
			toast.error(mensaje(e));
			setFase("revisar");
		}
	};

	const cambiar = (blob: string, cambio: Partial<Fila>) =>
		setFilas((fs) => fs.map((f) => (f.carta.front?.blob === blob ? { ...f, ...cambio } : f)));

	const cuenta = (c: Confidence) => filas.filter((f) => f.resultado?.confidence === c).length;
	const visibles = filtro === "todas" ? filas : filas.filter((f) => !f.marcada);
	const segundos =
		progreso.hechas > 0
			? ((performance.now() - inicio.current) / 1000 / progreso.hechas) *
				(progreso.total - progreso.hechas)
			: Number.NaN;
	const trabajando = fase === "leyendo" || fase === "aplicando";

	return (
		<DialogContent
			className="flex max-h-[90vh] flex-col sm:max-w-4xl"
			ocupado={trabajando}
			alInterrumpir={() => {
				cancelarLectura();
				parar.current = true;
			}}
			cambiosSinGuardar={fase === "revisar" && filas.length > 0}
		>
			<DialogHeader>
				<DialogTitle>{t("Vincular con Scryfall en bloque")}</DialogTitle>
				<DialogDescription>
					{enSeleccion
						? t("{n} imágenes propias distintas entre las cartas seleccionadas.", {
								n: propias.length,
							})
						: t("{n} imágenes propias distintas en el deck.", { n: propias.length })}
				</DialogDescription>
			</DialogHeader>

			{fase === "elegir" && (
				<div className="flex flex-col gap-3">
					{haySeleccion && (
						<ToggleGroup
							type="single"
							variant="outline"
							size="sm"
							value={enSeleccion ? "seleccion" : "deck"}
							onValueChange={(v) => v && setEnSeleccion(v === "seleccion")}
							className="self-start"
						>
							<ToggleGroupItem value="deck" className="h-7 px-2.5 text-[11px]">
								{t("Todo el deck ({n})", { n: delDeck.length })}
							</ToggleGroupItem>
							<ToggleGroupItem value="seleccion" className="h-7 px-2.5 text-[11px]">
								{t("Solo las seleccionadas ({n})", { n: deSeleccion.length })}
							</ToggleGroupItem>
						</ToggleGroup>
					)}
					<p className="text-xs font-medium">{t("¿Qué cartas quieres leer?")}</p>
					<div className="grid gap-2 sm:grid-cols-2">
						<Eleccion
							activa={alcance === "faltan"}
							onClick={() => setAlcance("faltan")}
							titulo={t("Solo las que faltan")}
							detalle={t("{n} sin vincular", { n: faltan.length })}
							deshabilitada={faltan.length === 0}
						/>
						<Eleccion
							activa={alcance === "todas"}
							onClick={() => setAlcance("todas")}
							titulo={t("Todas otra vez")}
							detalle={t("{n} imágenes, también las ya vinculadas", { n: propias.length })}
							deshabilitada={propias.length === 0}
						/>
					</div>
					<label className="flex items-center gap-2 text-xs">
						<Checkbox checked={ocr} onCheckedChange={(v) => setOcr(v === true)} />
						<ScanText className="size-3.5" />
						{t("Leer la carta entera con OCR (más fiable, unos 2 s por carta)")}
					</label>
					<p className="text-[11px] text-muted-foreground">
						{ocr
							? t(
									"Sin OCR solo se usa el nombre del fichero. La primera vez, el OCR descarga sus modelos (unos 12 MB).",
								)
							: t("Sin OCR solo se usa el nombre del fichero: va rápido, pero acierta menos.")}
					</p>
				</div>
			)}

			{(fase === "leyendo" || fase === "aplicando") && (
				<div className="flex flex-col gap-1.5">
					<div className="flex items-center gap-2 text-xs">
						<Loader2 className="size-3.5 animate-spin text-primary" />
						<span className="flex-1">
							{fase === "leyendo"
								? t("Leyendo {h} de {n}", { h: progreso.hechas, n: progreso.total })
								: t("Vinculando {h} de {n}", { h: progreso.hechas, n: progreso.total })}
						</span>
						{fase === "leyendo" && (
							<span className="font-mono text-[11px] text-muted-foreground">
								{restante(segundos, t)}
							</span>
						)}
					</div>
					<Progress
						value={progreso.total ? (progreso.hechas / progreso.total) * 100 : 0}
						className="h-1.5"
					/>
				</div>
			)}

			{(fase === "revisar" || (fase === "leyendo" && filas.length > 0)) && (
				<div className="flex flex-wrap items-center gap-2.5 text-[11px]">
					<span className="rounded bg-green-600/15 px-1.5 py-0.5">
						{t("Exactas: {n}", { n: cuenta("exact") })}
					</span>
					<span className="rounded bg-amber-500/15 px-1.5 py-0.5">
						{t("Probables: {n}", { n: cuenta("probable") })}
					</span>
					<span className="rounded bg-muted px-1.5 py-0.5">
						{t("Sin propuesta: {n}", {
							n: cuenta("none") + filas.filter((f) => !f.resultado).length,
						})}
					</span>
					<ToggleGroup
						type="single"
						variant="outline"
						size="sm"
						className="ml-auto"
						value={filtro}
						onValueChange={(v) => (v === "todas" || v === "revisar") && setFiltro(v)}
					>
						<ToggleGroupItem value="todas" className="h-6 px-2 text-[11px]">
							{t("Todas")}
						</ToggleGroupItem>
						<ToggleGroupItem value="revisar" className="h-6 px-2 text-[11px]">
							{t("Sin marcar")}
						</ToggleGroupItem>
					</ToggleGroup>
				</div>
			)}

			{fase !== "elegir" && (
				<DialogCuerpo className="min-h-48 flex-1" onViewportChange={setViewport}>
					<RaizScroll.Provider value={viewport}>
						<div className="flex flex-col divide-y">
							{visibles.map((f) => (
								<FilaVinculo
									key={f.carta.front?.blob ?? f.carta.id}
									fila={f}
									bloqueada={fase === "aplicando"}
									onCambiar={(c) => f.carta.front && cambiar(f.carta.front.blob, c)}
								/>
							))}
						</div>
					</RaizScroll.Provider>
				</DialogCuerpo>
			)}

			<DialogFooter className="items-center">
				{fase === "elegir" && (
					<Button size="sm" onClick={empezar} disabled={elegidas.length === 0}>
						<ScanText />
						{t("Empezar ({n})", { n: elegidas.length })}
					</Button>
				)}
				{fase === "leyendo" && (
					<Button size="sm" variant="outline" onClick={cancelarLectura}>
						<X />
						{t("Parar y revisar lo leído")}
					</Button>
				)}
				{fase === "revisar" && (
					<Button size="sm" onClick={aplicar} disabled={aVincular.length === 0}>
						<Link2 />
						{t("Vincular {n}", { n: aVincular.length })}
					</Button>
				)}
				{fase === "aplicando" && (
					<Button size="sm" variant="outline" onClick={() => (parar.current = true)}>
						<X />
						{t("Parar")}
					</Button>
				)}
			</DialogFooter>
		</DialogContent>
	);
}

function Eleccion({
	activa,
	onClick,
	titulo,
	detalle,
	deshabilitada,
}: {
	activa: boolean;
	onClick: () => void;
	titulo: string;
	detalle: string;
	deshabilitada?: boolean;
}) {
	return (
		<button
			type="button"
			onClick={onClick}
			disabled={deshabilitada}
			aria-pressed={activa}
			className={cn(
				"flex flex-col gap-0.5 rounded-lg border p-2.5 text-left transition-colors hover:bg-accent disabled:pointer-events-none disabled:opacity-50",
				activa && "border-primary bg-primary/10",
			)}
		>
			<span className="text-xs font-medium">{titulo}</span>
			<span className="text-[11px] text-muted-foreground">{detalle}</span>
		</button>
	);
}

const FilaVinculo = memo(function FilaVinculo({
	fila,
	bloqueada,
	onCambiar,
}: {
	fila: Fila;
	bloqueada: boolean;
	onCambiar: (c: Partial<Fila>) => void;
}) {
	const { t } = useLang();
	const ajustes = useDeck((s) => s.deck?.settings);
	const [el, setEl] = useState<HTMLDivElement | null>(null);
	const visible = useVisible(el);
	const { carta, resultado: r } = fila;
	const confianza = r?.confidence ?? "none";
	const elegida = r?.candidates.find((c) => c.id === fila.elegida) ?? null;
	const opciones: Opcion<string>[] = [
		{ valor: NINGUNA, etiqueta: t("No vincular") },
		...(r?.candidates ?? []).map((c) => ({
			valor: c.id,
			etiqueta: `${c.printedName ?? c.name} · ${c.set.toUpperCase()} ${c.collectorNumber}${c.lang !== "en" ? ` · ${c.lang}` : ""}`,
		})),
	];
	const leido = [carta.meta.name, ...(r?.lines ?? [])].filter(Boolean).join(" · ");

	return (
		<div ref={setEl} className="flex items-center gap-2.5 py-1.5">
			<Checkbox
				checked={fila.marcada}
				disabled={bloqueada || !fila.elegida}
				onCheckedChange={(v) => onCambiar({ marcada: v === true })}
				aria-label={t("Vincular esta")}
			/>
			<div className="h-14 w-10 shrink-0 overflow-hidden rounded-[3px] bg-muted">
				{visible && carta.front && ajustes && (
					<img
						src={urlMiniatura(carta.front.blob, parametros(ajustes, EDICION_NEUTRA, false, 0), 146)}
						alt=""
						loading="lazy"
						className="size-full object-cover"
					/>
				)}
			</div>
			<div className="min-w-0 flex-1">
				<div className="truncate text-[11px] text-muted-foreground">{leido || "—"}</div>
				<div className="mt-1 flex items-center gap-1.5">
					<Selector
						valor={fila.elegida ?? NINGUNA}
						opciones={opciones}
						onChange={(v) =>
							onCambiar(
								v === NINGUNA ? { elegida: null, marcada: false } : { elegida: v, marcada: true },
							)
						}
						ancho="w-full max-w-md"
						ariaLabel={t("Carta de Scryfall")}
					/>
					<span
						className={cn(
							"shrink-0 rounded-[3px] px-1.5 py-0.5 text-[10px] font-semibold",
							CONFIANZA[confianza].clase,
						)}
					>
						{t(CONFIANZA[confianza].texto)}
					</span>
				</div>
			</div>
			<div className="h-14 w-10 shrink-0 overflow-hidden rounded-[3px] bg-muted">
				{visible && elegida?.thumb && (
					<img
						src={urlBusqueda(elegida.thumb)}
						alt=""
						loading="lazy"
						className="size-full object-cover"
					/>
				)}
			</div>
		</div>
	);
});
