import {
	Bookmark,
	Contrast,
	Crop,
	Crosshair,
	FileDown,
	FileOutput,
	Layers,
	Loader2,
	Printer,
	Rows3,
	Ruler,
	Scissors,
	X,
} from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";
import type { BleedMode } from "@/bindings/BleedMode";
import type { Duplex } from "@/bindings/Duplex";
import type { EdgeMode } from "@/bindings/EdgeMode";
import type { ExportEstimate } from "@/bindings/ExportEstimate";
import type { PaperFormat } from "@/bindings/PaperFormat";
import type { PrintSettings } from "@/bindings/PrintSettings";
import type { Quality } from "@/bindings/Quality";
import {
	CampoNumero,
	Fila,
	FilaDeslizador,
	FilaInterruptor,
	type Opcion,
	Seccion,
	Selector,
} from "@/components/campos";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { Progress } from "@/components/ui/progress";
import { ScrollArea } from "@/components/ui/scroll-area";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { cancelarTrabajo } from "@/features/decklist/acciones";
import { call } from "@/lib/ipc";
import { useDeck } from "@/stores/deck-store";
import { usePlan } from "@/stores/plan-store";
import { useTrabajo } from "@/stores/trabajo-store";
import { AVISO_BYTES, exportarA, exportarCalibracion, megas, pedirDestino } from "./acciones";
import { AlinearFilas } from "./alinear-filas";
import { PresetsImpresion } from "./presets";

const TAMANOS = {
	mtg: { widthMm: 63, heightMm: 88 },
	poker: { widthMm: 63.5, heightMm: 88.9 },
};

/**
 * **Panel derecho**: todo lo que decide cómo sale el papel. Cada cambio va
 * al deck (se guarda con él y se deshace con Ctrl+Z) y la vista se recalcula.
 */
export function ExportPanel() {
	const { t, mensaje } = useLang();
	const s = useDeck((st) => st.deck?.settings);
	const setAjustes = useDeck((st) => st.setAjustes);
	const filas = usePlan((st) => st.plan?.grid.rows ?? 3);
	const trabajo = useTrabajo((st) => st.actual);
	const [aviso, setAviso] = useState<{ destino: string; estimacion: ExportEstimate } | null>(null);
	if (!s) return <section className="h-full" />;

	const poner = <K extends keyof PrintSettings>(k: K, v: PrintSettings[K]) =>
		setAjustes((a) => ({ ...a, [k]: v }));
	const intentar = (f: () => Promise<unknown>) => () => {
		f().catch((e) => toast.error(mensaje(e)));
	};

	const exportando = trabajo?.tipo === "exportar";
	const exportar = intentar(async () => {
		const destino = await pedirDestino(t);
		if (!destino) return;
		const deck = useDeck.getState().deck;
		if (!deck) return;
		const estimacion = await call("export_estimate", { deck });
		if (estimacion.bytes > AVISO_BYTES) {
			setAviso({ destino, estimacion });
			return;
		}
		await exportarA(destino, t);
	});

	const papeles: Opcion<PaperFormat>[] = [
		{ valor: "A3", etiqueta: "A3", grupo: "DIN / ISO" },
		{ valor: "A4", etiqueta: "A4", grupo: "DIN / ISO" },
		{ valor: "A5", etiqueta: "A5", grupo: "DIN / ISO" },
		{ valor: "Letter", etiqueta: "Letter", grupo: t("EE. UU.") },
		{ valor: "Legal", etiqueta: "Legal", grupo: t("EE. UU.") },
		{ valor: "Tabloid", etiqueta: "Tabloid", grupo: t("EE. UU.") },
		{ valor: "Photo4x6", etiqueta: "4×6″", grupo: t("Foto") },
		{ valor: "Photo5x7", etiqueta: "5×7″", grupo: t("Foto") },
		{ valor: "Photo8x10", etiqueta: "8×10″", grupo: t("Foto") },
		{ valor: "Photo8x12", etiqueta: "8×12″", grupo: t("Foto") },
	];
	const duplex: Opcion<Duplex>[] = [
		{ valor: "off", etiqueta: t("Una cara") },
		{ valor: "longEdge", etiqueta: t("Dúplex, borde largo") },
		{ valor: "shortEdge", etiqueta: t("Dúplex, borde corto") },
	];
	const modos: Opcion<EdgeMode>[] = [
		{ valor: "contrastEdges", etiqueta: t("Contraste") },
		{ valor: "solidBlack", etiqueta: t("Negro sólido") },
		{ valor: "threshold", etiqueta: t("Umbral") },
	];
	const rellenos: Opcion<BleedMode>[] = [
		{ valor: "clamp", etiqueta: t("Repetir el borde") },
		{ valor: "mirror", etiqueta: t("En espejo") },
	];
	const tamano = s.cardSize.widthMm === 63.5 ? "poker" : "mtg";
	const edge = s.edgeDarkening;
	const guias = s.cutGuides;

	return (
		<section className="flex h-full min-h-0 flex-col [&_[data-slot=button]]:text-xs">
			<div className="flex flex-col gap-2 border-b p-2.5">
				<h2 className="text-[11px] font-semibold tracking-widest text-muted-foreground uppercase">
					{t("Exportar")}
				</h2>
				{exportando ? (
					<div className="flex flex-col gap-1.5">
						<div className="flex items-center gap-2 text-xs">
							<Loader2 className="size-3.5 animate-spin text-primary" />
							<span className="flex-1 truncate">{trabajo.fase}</span>
							{trabajo.total > 0 && (
								<span className="font-mono text-[11px] text-muted-foreground">
									{trabajo.hechas}/{trabajo.total}
								</span>
							)}
						</div>
						<Progress
							value={trabajo.total ? (trabajo.hechas / trabajo.total) * 100 : 0}
							className="h-1.5"
						/>
						<Button size="sm" variant="outline" onClick={cancelarTrabajo} disabled={!trabajo.jobId}>
							<X />
							{t("Cancelar exportación")}
						</Button>
					</div>
				) : (
					<Button onClick={exportar} disabled={trabajo !== null} className="!text-sm">
						<FileDown />
						{t("Exportar PDF")}
					</Button>
				)}
			</div>

			<ScrollArea className="min-h-0 flex-1">
				<div className="flex flex-col gap-2.5 p-2.5">
					<Seccion id="exp-pdf" titulo={t("PDF")} icono={<FileOutput className="size-3.5" />}>
						<Fila etiqueta={t("Calidad")}>
							<ToggleGroup
								type="single"
								variant="outline"
								size="sm"
								value={s.quality}
								onValueChange={(v) => v && poner("quality", v as Quality)}
							>
								{(["dpi300", "dpi600", "dpi900"] as const).map((q) => (
									<ToggleGroupItem key={q} value={q} className="h-7 px-2 text-[11px]">
										{q.slice(3)}
									</ToggleGroupItem>
								))}
							</ToggleGroup>
						</Fila>
						<Fila etiqueta={t("Imágenes")}>
							<ToggleGroup
								type="single"
								variant="outline"
								size="sm"
								value={s.encoding.format}
								onValueChange={(v) =>
									(v === "jpeg" || v === "png") && poner("encoding", { ...s.encoding, format: v })
								}
							>
								<ToggleGroupItem value="jpeg" className="h-7 px-2 text-[11px]">
									JPEG
								</ToggleGroupItem>
								<ToggleGroupItem value="png" className="h-7 px-2 text-[11px]">
									PNG
								</ToggleGroupItem>
							</ToggleGroup>
						</Fila>
						{s.encoding.format === "jpeg" && (
							<FilaDeslizador
								etiqueta={t("Calidad JPEG")}
								valor={s.encoding.jpegQuality}
								min={60}
								max={100}
								onChange={(v) => poner("encoding", { ...s.encoding, jpegQuality: v })}
							/>
						)}
						<Fila
							etiqueta={t("Partir cada")}
							ayuda={t("Un PDF por cada N hojas. Vacío: un solo fichero.")}
						>
							<CampoNumero
								valor={s.splitEvery}
								onChange={(v) => poner("splitEvery", v === null || v < 1 ? null : Math.round(v))}
								min={1}
								max={500}
								paso={1}
								sufijo={t("hojas")}
								ancho="w-28"
								vacioEsNulo
								placeholder="—"
							/>
						</Fila>
					</Seccion>

					<Seccion id="exp-duplex" titulo={t("Doble cara")} icono={<Layers className="size-3.5" />}>
						<Fila etiqueta={t("Impresión")}>
							<Selector
								valor={s.duplex}
								opciones={duplex}
								onChange={(v) => poner("duplex", v)}
								ancho="w-44"
							/>
						</Fila>
						<FilaInterruptor
							etiqueta={t("Pares DFC intercalados")}
							ayuda={t("Las dos caras de cada carta de doble cara, en casillas seguidas")}
							valor={s.interleaveDfc}
							onChange={(v) => poner("interleaveDfc", v)}
						/>
						<Button size="sm" variant="outline" onClick={intentar(() => exportarCalibracion(t))}>
							<Crosshair />
							{t("Hoja de calibración")}
						</Button>
					</Seccion>

					<Seccion id="exp-pagina" titulo={t("Página")} icono={<Printer className="size-3.5" />}>
						<Fila etiqueta={t("Papel")}>
							<Selector
								valor={s.paper}
								opciones={papeles}
								onChange={(v) => poner("paper", v)}
								ancho="w-28"
							/>
						</Fila>
						<FilaInterruptor
							etiqueta={t("Apaisado")}
							valor={s.landscape}
							onChange={(v) => poner("landscape", v)}
						/>
						<Fila etiqueta={t("Tamaño de carta")}>
							<Selector
								valor={tamano}
								opciones={[
									{ valor: "mtg", etiqueta: "63 × 88 mm" },
									{ valor: "poker", etiqueta: "63,5 × 88,9 mm" },
								]}
								onChange={(v) => poner("cardSize", TAMANOS[v])}
								ancho="w-36"
							/>
						</Fila>
						<Fila etiqueta={t("Margen mínimo")}>
							<CampoNumero
								valor={s.minMarginMm}
								onChange={(v) => poner("minMarginMm", v ?? 0)}
								min={0}
								max={40}
								sufijo="mm"
							/>
						</Fila>
					</Seccion>

					<Seccion
						id="exp-sangrado"
						titulo={t("Separación y sangrado")}
						icono={<Crop className="size-3.5" />}
					>
						<Fila etiqueta={t("Separación")} ayuda={t("Espacio entre cartas")}>
							<CampoNumero
								valor={s.gapMm}
								onChange={(v) => poner("gapMm", v ?? 0)}
								min={0}
								max={20}
								sufijo="mm"
							/>
						</Fila>
						<Fila
							etiqueta={t("Sangrado")}
							ayuda={t(
								"Imagen extra fuera del corte, para que no queden filos blancos. Entre cartas se limita a la mitad de la separación.",
							)}
						>
							<CampoNumero
								valor={s.bleedMm}
								onChange={(v) => poner("bleedMm", v ?? 0)}
								min={0}
								max={5}
								sufijo="mm"
							/>
						</Fila>
						<Fila etiqueta={t("Relleno del sangrado")}>
							<Selector
								valor={s.bleedMode}
								opciones={rellenos}
								onChange={(v) => poner("bleedMode", v)}
							/>
						</Fila>
						<FilaInterruptor
							etiqueta={t("Sangrado en los reversos")}
							valor={s.bleedOnBacks}
							onChange={(v) => poner("bleedOnBacks", v)}
						/>
					</Seccion>

					<Seccion
						id="exp-guias"
						titulo={t("Guías de corte")}
						icono={<Scissors className="size-3.5" />}
					>
						<FilaInterruptor
							etiqueta={t("Dibujar guías")}
							valor={guias.enabled}
							onChange={(v) => poner("cutGuides", { ...guias, enabled: v })}
						/>
						{guias.enabled && (
							<>
								<Fila etiqueta={t("Color")}>
									<input
										type="color"
										value={guias.color}
										onChange={(e) => poner("cutGuides", { ...guias, color: e.target.value })}
										aria-label={t("Color")}
										className="h-7 w-12 cursor-pointer rounded-md border bg-transparent p-0.5"
									/>
								</Fila>
								<Fila etiqueta={t("Grosor")}>
									<CampoNumero
										valor={guias.widthPx}
										onChange={(v) => poner("cutGuides", { ...guias, widthPx: v ?? 0.5 })}
										min={0.1}
										max={5}
										sufijo="px"
									/>
								</Fila>
								<Fila etiqueta={t("Separación del borde")}>
									<CampoNumero
										valor={guias.offsetMm}
										onChange={(v) => poner("cutGuides", { ...guias, offsetMm: v ?? 0 })}
										min={0}
										max={20}
										sufijo="mm"
									/>
								</Fila>
								<FilaDeslizador
									etiqueta={t("Longitud")}
									valor={guias.lengthMm}
									min={1}
									max={30}
									paso={0.5}
									sufijo=" mm"
									onChange={(v) => poner("cutGuides", { ...guias, lengthMm: v })}
								/>
							</>
						)}
					</Seccion>

					<Seccion
						id="exp-bordes"
						titulo={t("Oscurecer bordes")}
						icono={<Contrast className="size-3.5" />}
						abiertaAlPrincipio={false}
					>
						<FilaInterruptor
							etiqueta={t("Oscurecer los bordes claros")}
							ayuda={t("Para escaneos viejos con el borde negro agrisado")}
							valor={edge.enabled}
							onChange={(v) => poner("edgeDarkening", { ...edge, enabled: v })}
						/>
						{edge.enabled && (
							<>
								<Fila etiqueta={t("Modo")}>
									<Selector
										valor={edge.mode}
										opciones={modos}
										onChange={(v) => poner("edgeDarkening", { ...edge, mode: v })}
									/>
								</Fila>
								<FilaDeslizador
									etiqueta={t("Cantidad")}
									valor={edge.amount}
									min={0}
									max={100}
									sufijo=" %"
									onChange={(v) => poner("edgeDarkening", { ...edge, amount: v })}
								/>
								<FilaDeslizador
									etiqueta={t("Ancho de la banda")}
									valor={edge.widthPct}
									min={1}
									max={20}
									paso={0.5}
									sufijo=" %"
									onChange={(v) => poner("edgeDarkening", { ...edge, widthPct: v })}
								/>
								<FilaInterruptor
									etiqueta={t("Detectar el tipo de borde")}
									ayuda={t("No toca las cartas de borde blanco, sin borde o de arte completo")}
									valor={edge.autoDetect}
									onChange={(v) => poner("edgeDarkening", { ...edge, autoDetect: v })}
								/>
								<FilaInterruptor
									etiqueta={t("En cartas de Scryfall")}
									valor={edge.applyScryfall}
									onChange={(v) => poner("edgeDarkening", { ...edge, applyScryfall: v })}
								/>
								<FilaInterruptor
									etiqueta={t("En imágenes propias")}
									valor={edge.applyUploads}
									onChange={(v) => poner("edgeDarkening", { ...edge, applyUploads: v })}
								/>
								<FilaInterruptor
									etiqueta={t("En los reversos")}
									valor={edge.applyBacks}
									onChange={(v) => poner("edgeDarkening", { ...edge, applyBacks: v })}
								/>
							</>
						)}
					</Seccion>

					<Seccion
						id="exp-filas"
						titulo={t("Alinear filas")}
						icono={<Rows3 className="size-3.5" />}
						abiertaAlPrincipio={false}
					>
						<AlinearFilas filas={filas} />
					</Seccion>

					<Seccion
						id="exp-presets"
						titulo={t("Presets de impresión")}
						icono={<Bookmark className="size-3.5" />}
						abiertaAlPrincipio={false}
					>
						<p className="text-[11px] text-muted-foreground">
							{t(
								"Guarda todos los ajustes de esta columna con un nombre para usarlos en otros decks.",
							)}
						</p>
						<PresetsImpresion />
					</Seccion>

					<p className="flex items-start gap-1.5 px-1 text-[10px] text-muted-foreground">
						<Ruler className="mt-px size-3 shrink-0" />
						{t("Imprime al 100 % (sin «ajustar a la página») para que las medidas salgan exactas.")}
					</p>
				</div>
			</ScrollArea>

			<Dialog open={aviso !== null} onOpenChange={(o) => !o && setAviso(null)}>
				<DialogContent className="sm:max-w-md" cambiosSinGuardar={false}>
					<DialogHeader>
						<DialogTitle>{t("El PDF va a ser muy grande")}</DialogTitle>
						<DialogDescription>
							{t("Unos {mb} con {n} imágenes. Puedes bajar la calidad o exportar igualmente.", {
								mb: megas(aviso?.estimacion.bytes ?? 0),
								n: aviso?.estimacion.images ?? 0,
							})}
						</DialogDescription>
					</DialogHeader>
					<DialogFooter className="sm:flex-wrap">
						<Button
							size="sm"
							variant="outline"
							onClick={() => {
								const d = aviso?.destino;
								setAviso(null);
								setAjustes((a) => ({ ...a, encoding: { format: "jpeg", jpegQuality: 90 } }));
								if (d) void intentar(() => exportarA(d, t))();
							}}
						>
							{t("Bajar a JPEG 90")}
						</Button>
						<Button
							size="sm"
							variant="outline"
							onClick={() => {
								const d = aviso?.destino;
								setAviso(null);
								setAjustes((a) => ({ ...a, quality: "dpi300" }));
								if (d) void intentar(() => exportarA(d, t))();
							}}
						>
							{t("Bajar a 300 PPP")}
						</Button>
						<Button
							size="sm"
							onClick={() => {
								const d = aviso?.destino;
								setAviso(null);
								if (d) void intentar(() => exportarA(d, t))();
							}}
						>
							{t("Exportar igualmente")}
						</Button>
					</DialogFooter>
				</DialogContent>
			</Dialog>
		</section>
	);
}
