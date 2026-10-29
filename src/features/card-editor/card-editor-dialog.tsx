import { useMemo, useRef, useState } from "react";
import type { Adjustments } from "@/bindings/Adjustments";
import type { CardEdits } from "@/bindings/CardEdits";
import type { DeckCard } from "@/bindings/DeckCard";
import type { Rotation } from "@/bindings/Rotation";
import { FilaDeslizador } from "@/components/campos";
import { Icono } from "@/components/icono";
import { paraTraducir, useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import {
	Dialog,
	DialogContent,
	DialogCuerpo,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { WithTooltip } from "@/components/ui/with-tooltip";
import { cartaPorId } from "@/features/preview/acciones-carta";
import { EDICION_NEUTRA, parametros, urlMiniatura } from "@/lib/image-url";
import { useAlRitmo, useImagenViva, useRafaga } from "@/lib/imagen-viva";
import { useDeck } from "@/stores/deck-store";
import { useUi } from "@/stores/ui-store";
import { type Alcance, aplicarEdicion, copiasDe } from "./propagar";

/** Los once retoques, en el orden de fabricard. */
const RETOQUES: { k: keyof Adjustments; nombre: string; min: number }[] = [
	{ k: "brightness", nombre: paraTraducir("Brillo"), min: -100 },
	{ k: "exposure", nombre: paraTraducir("Exposición"), min: -100 },
	{ k: "contrast", nombre: paraTraducir("Contraste"), min: -100 },
	{ k: "saturation", nombre: paraTraducir("Saturación"), min: -100 },
	{ k: "warmth", nombre: paraTraducir("Calidez"), min: -100 },
	{ k: "vibrance", nombre: paraTraducir("Intensidad"), min: -100 },
	{ k: "sharpness", nombre: paraTraducir("Nitidez"), min: 0 },
	{ k: "clarity", nombre: paraTraducir("Claridad"), min: 0 },
	{ k: "noiseReduction", nombre: paraTraducir("Reducción de ruido"), min: 0 },
	{ k: "vignette", nombre: paraTraducir("Viñeta"), min: 0 },
	{ k: "fade", nombre: paraTraducir("Desvanecido"), min: 0 },
];

const GIROS: Rotation[] = ["r0", "r90", "r180", "r270"];

/**
 * «Editar carta»: giro, encuadre y los once retoques. Mientras se arrastra un
 * deslizador se aproxima con un filtro CSS; al soltar (o tras 150 ms quieto)
 * se pide a Rust la imagen de verdad, que es lo que acaba viéndose: el mismo
 * render que irá al PDF.
 */
export function CardEditorDialog() {
	const dialogo = useUi((s) => s.dialogo);
	const id = dialogo?.tipo === "editar" ? dialogo.id : null;
	const carta = id ? cartaPorId(id) : undefined;
	return (
		<Dialog open={!!carta?.front} onOpenChange={(o) => !o && useUi.getState().abrir(null)}>
			{carta?.front && <Contenido carta={carta} />}
		</Dialog>
	);
}

/** Filtro CSS provisional: la diferencia entre el borrador y lo ya pintado por Rust. */
function filtroAproximado(borrador: Adjustments, pintado: Adjustments): string {
	const d = (k: keyof Adjustments) => (borrador[k] - pintado[k]) / 100;
	const brillo = 1 + d("brightness") * 0.6 + d("exposure") * 0.8;
	return `brightness(${brillo.toFixed(3)}) contrast(${(1 + d("contrast")).toFixed(3)}) saturate(${(1 + d("saturation") + d("vibrance") * 0.5).toFixed(3)})`;
}

function Contenido({ carta }: { carta: DeckCard }) {
	const { t } = useLang();
	const ajustes = useDeck((s) => s.deck?.settings);
	const copias = useDeck((s) => (s.deck ? copiasDe(s.deck.cards, carta).length : 0));
	const [borrador, setBorrador] = useState<CardEdits>(carta.edits);
	const arrastre = useRef<{ x: number; y: number; ox: number; oy: number } | null>(null);
	const marco = useRef<HTMLDivElement>(null);

	// Lo pintado sigue al borrador en tiempo real: como mucho cada 60 ms
	// mientras se arrastra o se mueve un deslizador, y más pequeño entretanto.
	const pintado = useAlRitmo(borrador, 60);
	const rafaga = useRafaga(pintado);

	const front = carta.front;
	const url = useMemo(
		() =>
			front && ajustes
				? urlMiniatura(front.blob, parametros(ajustes, pintado, false, 0), rafaga ? 488 : 745)
				: null,
		[front, ajustes, pintado, rafaga],
	);
	const { mostrada, cargando } = useImagenViva(url);
	// Los retoques que aún no se ven se aproximan con CSS sobre la imagen que
	// hay: para eso se recuerda con qué retoques se pidió cada URL.
	const porUrl = useRef(new Map<string, CardEdits>());
	if (url && !porUrl.current.has(url)) {
		if (porUrl.current.size > 64) porUrl.current.clear();
		porUrl.current.set(url, pintado);
	}
	const enPantalla = (mostrada && porUrl.current.get(mostrada)) || pintado;

	if (!ajustes || !url) return null;

	const ajustar = (k: keyof Adjustments, v: number) =>
		setBorrador((b) => ({ ...b, adjust: { ...b.adjust, [k]: v } }));
	const encuadre = (c: Partial<CardEdits["crop"]>) =>
		setBorrador((b) => ({ ...b, crop: { ...b.crop, ...c } }));
	const girar = (paso: 1 | -1) =>
		setBorrador((b) => ({ ...b, rotation: GIROS[(GIROS.indexOf(b.rotation) + paso + 4) % 4] }));

	const guardar = (alcance: Alcance) => {
		useDeck.getState().setCartas((cs) => aplicarEdicion(cs, carta, borrador, alcance));
		useUi
			.getState()
			.anunciar(
				alcance === "todas"
					? t("Retoques aplicados a todas las cartas")
					: alcance === "copias"
						? t("Carta y sus {n} copias retocadas", { n: copias })
						: t("Carta retocada"),
			);
		useUi.getState().abrir(null);
	};

	// Arrastrar la imagen mueve el encuadre (si hay zoom).
	const alBajar = (e: React.PointerEvent) => {
		if (borrador.crop.zoom <= 1) return;
		(e.target as HTMLElement).setPointerCapture(e.pointerId);
		arrastre.current = {
			x: e.clientX,
			y: e.clientY,
			ox: borrador.crop.offsetX,
			oy: borrador.crop.offsetY,
		};
	};
	const alMover = (e: React.PointerEvent) => {
		const a = arrastre.current;
		const m = marco.current;
		if (!a || !m) return;
		const r = m.getBoundingClientRect();
		const margen = 1 - 1 / borrador.crop.zoom;
		const dx = ((e.clientX - a.x) / r.width / Math.max(margen, 0.01)) * -2;
		const dy = ((e.clientY - a.y) / r.height / Math.max(margen, 0.01)) * -2;
		encuadre({
			offsetX: Math.max(-1, Math.min(1, a.ox + dx)),
			offsetY: Math.max(-1, Math.min(1, a.oy + dy)),
		});
	};
	const alSoltar = () => {
		arrastre.current = null;
	};

	const esScryfall = carta.source.kind === "scryfall" || carta.source.kind === "token";
	const recortaElPie = esScryfall && borrador.crop.zoom > 1.05;
	const cambiado = JSON.stringify(borrador) !== JSON.stringify(carta.edits);

	return (
		<DialogContent className="flex max-h-[92vh] flex-col sm:max-w-4xl">
			<DialogHeader>
				<DialogTitle>{t("Editar {carta}", { carta: carta.meta.name || t("carta") })}</DialogTitle>
			</DialogHeader>
			<div className="flex min-h-0 flex-1 gap-4">
				<div className="flex w-[46%] shrink-0 flex-col items-center gap-2">
					<div
						ref={marco}
						className="relative aspect-[63/88] w-full max-w-sm touch-none overflow-hidden rounded-[4.5%] bg-muted shadow-md"
						style={{ cursor: borrador.crop.zoom > 1 ? "grab" : "default" }}
						onPointerDown={alBajar}
						onPointerMove={alMover}
						onPointerUp={alSoltar}
						onPointerCancel={alSoltar}
					>
						{mostrada && (
							<img
								src={mostrada}
								alt=""
								draggable={false}
								className="size-full select-none object-cover"
								style={{ filter: filtroAproximado(borrador.adjust, enPantalla.adjust) }}
							/>
						)}
						{cargando && (
							<Icono
								icon="lucide:loader-2"
								className="absolute top-2 right-2 size-4 animate-spin text-white drop-shadow"
							/>
						)}
					</div>
					{recortaElPie && (
						<p className="flex items-center gap-1.5 text-[11px] text-amber-600 dark:text-amber-400">
							<Icono icon="lucide:alert-triangle" className="size-3.5" />
							{t("Con este zoom se recorta el pie de la carta (artista y copyright).")}
						</p>
					)}
				</div>
				<DialogCuerpo className="flex-1">
					<div className="flex flex-col gap-3 pr-1">
						<div className="flex items-center gap-1.5">
							<span className="flex-1 text-xs font-medium">{t("Giro")}</span>
							<Button
								size="icon-sm"
								variant="outline"
								onClick={() => girar(-1)}
								aria-label={t("Girar a la izquierda")}
							>
								<Icono icon="lucide:rotate-ccw" />
							</Button>
							<span className="w-10 text-center font-mono text-[11px]">
								{GIROS.indexOf(borrador.rotation) * 90}°
							</span>
							<Button
								size="icon-sm"
								variant="outline"
								onClick={() => girar(1)}
								aria-label={t("Girar a la derecha")}
							>
								<Icono icon="lucide:rotate-cw" />
							</Button>
						</div>
						<FilaDeslizador
							etiqueta={t("Zoom")}
							valor={borrador.crop.zoom}
							min={1}
							max={2}
							paso={0.01}
							sufijo="×"
							onChange={(v) => encuadre({ zoom: v })}
						/>
						<FilaDeslizador
							etiqueta={t("Encuadre horizontal")}
							valor={borrador.crop.offsetX}
							min={-1}
							max={1}
							paso={0.01}
							onChange={(v) => encuadre({ offsetX: v })}
						/>
						<FilaDeslizador
							etiqueta={t("Encuadre vertical")}
							valor={borrador.crop.offsetY}
							min={-1}
							max={1}
							paso={0.01}
							onChange={(v) => encuadre({ offsetY: v })}
						/>
						<div className="h-px bg-border" />
						{RETOQUES.map((r) => (
							<FilaDeslizador
								key={r.k}
								etiqueta={t(r.nombre)}
								valor={borrador.adjust[r.k]}
								min={r.min}
								max={100}
								onChange={(v) => ajustar(r.k, v)}
							/>
						))}
					</div>
				</DialogCuerpo>
			</div>
			<DialogFooter className="sm:justify-between">
				<Button
					size="sm"
					variant="ghost"
					onClick={() => setBorrador(EDICION_NEUTRA)}
					disabled={JSON.stringify(borrador) === JSON.stringify(EDICION_NEUTRA)}
				>
					<Icono icon="lucide:undo-2" />
					{t("Restablecer todo")}
				</Button>
				<div className="flex flex-wrap justify-end gap-2">
					<WithTooltip
						content={t(
							"Solo los retoques de color; el giro y el encuadre se quedan en esta carta.",
						)}
					>
						<Button size="sm" variant="outline" onClick={() => guardar("todas")}>
							{t("Aplicar a todas las cartas")}
						</Button>
					</WithTooltip>
					{copias > 0 && (
						<WithTooltip
							content={t("Las que llevan la misma imagen: giro, encuadre y retoques, todo igual.")}
						>
							<Button size="sm" variant="outline" onClick={() => guardar("copias")}>
								{t("Aplicar también a sus copias ({n})", { n: copias })}
							</Button>
						</WithTooltip>
					)}
					<Button size="sm" onClick={() => guardar("carta")} disabled={!cambiado}>
						{t("Aplicar")}
					</Button>
				</div>
			</DialogFooter>
		</DialogContent>
	);
}
