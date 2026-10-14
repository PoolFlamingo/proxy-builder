import { AlertTriangle, Info, Loader2, Move } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import type { Aviso } from "@/bindings/Aviso";
import type { CardDesign } from "@/bindings/CardDesign";
import type { DesignLang } from "@/bindings/DesignLang";
import { useLang } from "@/components/language-provider";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Switch } from "@/components/ui/switch";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { WithTooltip } from "@/components/ui/with-tooltip";
import { type AnchoMiniatura, urlDiseno } from "@/lib/image-url";
import { cn } from "@/lib/utils";
import { esGrave, textoAviso } from "./textos";

/**
 * La carta tal como saldrá, pintada por Rust (el mismo motor que el PDF).
 * Arrastrar sobre ella mueve la ilustración; la rueda la acerca o aleja.
 */
export function VistaPrevia({
	design,
	huella,
	avisos,
	lang,
	setLang,
	cara,
	setCara,
	onEncuadre,
	cargando,
	carasImpresas,
}: {
	design: CardDesign | null;
	huella: string | null;
	avisos: Aviso[];
	lang: DesignLang;
	setLang: (l: DesignLang) => void;
	cara: number;
	setCara: (c: number) => void;
	onEncuadre: (cambio: { dx?: number; dy?: number; zoom?: number }) => void;
	cargando: boolean;
	/** 2 en las de doble cara; en las partidas, aventuras… las caras son
	 *  partes de una sola cara impresa. */
	carasImpresas: number;
}) {
	const { t } = useLang();
	const [seguro, setSeguro] = useState(false);
	const [pintando, setPintando] = useState(false);
	const marco = useRef<HTMLDivElement>(null);
	const arrastre = useRef<{ x: number; y: number } | null>(null);
	const [ancho, setAncho] = useState<AnchoMiniatura>(745);

	// La resolución de la vista según el tamaño en pantalla.
	useEffect(() => {
		const m = marco.current;
		if (!m) return;
		const ro = new ResizeObserver(() => {
			const w = m.clientWidth * (window.devicePixelRatio || 1);
			setAncho(w > 760 ? 1000 : w > 500 ? 745 : 488);
		});
		ro.observe(m);
		return () => ro.disconnect();
	}, []);

	const url = huella ? urlDiseno(huella, ancho) : null;
	useEffect(() => {
		if (url) setPintando(true);
	}, [url]);

	const tieneArte = !!design?.faces[cara]?.art;
	const alBajar = (e: React.PointerEvent) => {
		if (!tieneArte) return;
		(e.target as HTMLElement).setPointerCapture(e.pointerId);
		arrastre.current = { x: e.clientX, y: e.clientY };
	};
	const alMover = (e: React.PointerEvent) => {
		const a = arrastre.current;
		const m = marco.current;
		if (!a || !m) return;
		const r = m.getBoundingClientRect();
		onEncuadre({ dx: (e.clientX - a.x) / r.width, dy: (e.clientY - a.y) / r.height });
		arrastre.current = { x: e.clientX, y: e.clientY };
	};
	const alSoltar = () => {
		arrastre.current = null;
	};
	const alRodar = (e: React.WheelEvent) => {
		if (!tieneArte || e.ctrlKey) return;
		onEncuadre({ zoom: e.deltaY < 0 ? 0.04 : -0.04 });
	};

	const caras = design?.faces.length ?? 0;
	return (
		<div className="flex h-full flex-col">
			<div className="flex shrink-0 items-center gap-2.5 border-b px-3 py-2">
				<ToggleGroup
					type="single"
					size="sm"
					value={lang}
					onValueChange={(v) => (v === "es" || v === "en") && setLang(v)}
				>
					<ToggleGroupItem value="es" className="px-2.5 text-xs">
						ES
					</ToggleGroupItem>
					<ToggleGroupItem value="en" className="px-2.5 text-xs">
						EN
					</ToggleGroupItem>
				</ToggleGroup>
				{caras > 1 && (
					<ToggleGroup
						type="single"
						size="sm"
						value={String(cara)}
						onValueChange={(v) => v && setCara(Number(v))}
					>
						{design?.faces.map((_, i) => (
							<ToggleGroupItem key={i} value={String(i)} className="px-2.5 text-xs">
								{carasImpresas > 1 ? t("Cara {n}", { n: i + 1 }) : t("Parte {n}", { n: i + 1 })}
							</ToggleGroupItem>
						))}
					</ToggleGroup>
				)}
				<label className="ml-auto flex items-center gap-1.5 text-xs text-muted-foreground">
					<Switch checked={seguro} onCheckedChange={setSeguro} />
					{t("Margen de seguridad")}
				</label>
			</div>
			<div className="flex min-h-0 flex-1 items-center justify-center p-4">
				<div
					ref={marco}
					className={cn(
						"relative aspect-[63/88] h-full max-h-full max-w-full touch-none select-none",
						tieneArte && "cursor-grab active:cursor-grabbing",
					)}
					onPointerDown={alBajar}
					onPointerMove={alMover}
					onPointerUp={alSoltar}
					onPointerCancel={alSoltar}
					onWheel={alRodar}
				>
					{url ? (
						<img
							src={url}
							alt=""
							draggable={false}
							onLoad={() => setPintando(false)}
							onError={() => setPintando(false)}
							className="size-full rounded-[4.5%] object-contain shadow-lg"
						/>
					) : (
						<div className="flex size-full items-center justify-center rounded-[4.5%] bg-muted">
							{cargando ? (
								<Loader2 className="size-6 animate-spin text-muted-foreground" />
							) : (
								<span className="px-6 text-center text-xs text-muted-foreground">
									{t("Elige una carta de la lista para diseñarla.")}
								</span>
							)}
						</div>
					)}
					{seguro && url && (
						<div
							className="pointer-events-none absolute rounded-[3%] border border-dashed border-cyan-400/90"
							style={{ inset: `${(3 / 88) * 100}% ${(3 / 63) * 100}%` }}
						/>
					)}
					{pintando && (
						<Loader2 className="absolute top-2 right-2 size-4 animate-spin text-white drop-shadow" />
					)}
					{tieneArte && url && (
						<WithTooltip
							content={t("Arrastra para mover la ilustración; la rueda la acerca o aleja.")}
						>
							<span className="absolute -bottom-4 left-1/2 -translate-x-1/2 text-muted-foreground">
								<Move className="size-3.5" />
							</span>
						</WithTooltip>
					)}
				</div>
			</div>
			{avisos.length > 0 && (
				<ScrollArea className="max-h-32 shrink-0 border-t">
					<ul className="flex flex-col gap-1 px-3 py-2">
						{avisos.map((a) => (
							<li
								key={JSON.stringify(a)}
								className={cn(
									"flex items-start gap-1.5 text-[11px]",
									esGrave(a) ? "text-amber-700 dark:text-amber-400" : "text-muted-foreground",
								)}
							>
								{esGrave(a) ? (
									<AlertTriangle className="mt-px size-3.5 shrink-0" />
								) : (
									<Info className="mt-px size-3.5 shrink-0" />
								)}
								{textoAviso(a, t)}
							</li>
						))}
					</ul>
				</ScrollArea>
			)}
		</div>
	);
}
