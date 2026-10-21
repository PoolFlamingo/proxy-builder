import { AlertTriangle, Download, Info, Loader2, Move } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { ArtFraming } from "@/bindings/ArtFraming";
import type { Aviso } from "@/bindings/Aviso";
import type { CapaInfo } from "@/bindings/CapaInfo";
import type { CardDesign } from "@/bindings/CardDesign";
import type { DesignLang } from "@/bindings/DesignLang";
import { useLang } from "@/components/language-provider";
import { SelectorIdioma } from "@/components/selector-idioma";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Switch } from "@/components/ui/switch";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { WithTooltip } from "@/components/ui/with-tooltip";
import { type AnchoMiniatura, urlArte, urlCapa, urlDiseno } from "@/lib/image-url";
import { useImagenViva, useRafaga } from "@/lib/imagen-viva";
import { call } from "@/lib/ipc";
import { cn } from "@/lib/utils";
import { ArteEnVivo } from "./arte-en-vivo";
import { CARTA_MM, type CambioEncuadre, claveCapa, moverEncuadre } from "./encuadre";
import { esGrave, textoAviso } from "./textos";

/** Cada cuánto se pasa al diseño el encuadre mientras se arrastra. */
const EMPUJE_MS = 100;
/** Tras el último giro de rueda, cuándo se da por acabado. */
const FIN_RUEDA_MS = 200;
/** El hueco de la ilustración (fracciones de la carta) si aún no hay capa. */
const HUECO_TIPICO = [0.08, 0.12, 0.84, 0.45] as const;

/**
 * La carta tal como saldrá, pintada por Rust (el mismo motor que el PDF).
 * Arrastrar sobre ella mueve la ilustración; la rueda la acerca o aleja.
 *
 * Mientras solo cambia el encuadre, la ilustración se mueve en la pantalla
 * (`ArteEnVivo`, por debajo de la carta sin ilustración) y Rust no pinta
 * nada hasta que se para; entonces entra la carta exacta.
 */
export function VistaPrevia({
	design,
	huella,
	alDia,
	recursos,
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
	/** `huella` es la del diseño de ahora. */
	alDia: boolean;
	/** Cambia con colecciones, letras o packs: la capa ya no vale. */
	recursos: unknown;
	avisos: Aviso[];
	lang: DesignLang;
	setLang: (l: DesignLang) => void;
	cara: number;
	setCara: (c: number) => void;
	onEncuadre: (framing: ArtFraming) => void;
	cargando: boolean;
	/** 2 en las de doble cara; en las partidas, aventuras… las caras son
	 *  partes de una sola cara impresa. */
	carasImpresas: number;
}) {
	const { t } = useLang();
	const [seguro, setSeguro] = useState(false);
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

	// ── Capa para mover la ilustración en la pantalla.
	const arte = design?.faces[cara]?.art ?? null;
	const tieneArte = !!arte;
	const clave = useMemo(
		() => (design ? claveCapa(design, cara, lang) : null),
		[design, cara, lang],
	);
	const [capa, setCapa] = useState<(CapaInfo & { clave: string; ancho: AnchoMiniatura }) | null>(
		null,
	);
	const pedida = useRef<string | null>(null);
	useEffect(() => {
		// Otras colecciones, letras o packs: la que hay ya no vale.
		setCapa(null);
		pedida.current = null;
	}, [recursos]);
	const pedirCapa = useCallback(() => {
		if (!design || !clave || !tieneArte) return;
		const k = `${clave}|${ancho}`;
		if (pedida.current === k) return;
		pedida.current = k;
		call("design_overlay", { design, cara, lang, ancho })
			.then((c) => {
				if (pedida.current !== k) return;
				setCapa(c ? { ...c, clave, ancho } : null);
			})
			.catch(() => {
				if (pedida.current === k) pedida.current = null;
			});
	}, [design, clave, tieneArte, cara, lang, ancho]);
	const capaVale = !!capa && capa.clave === clave && capa.cara === cara;

	// El encuadre mientras se arrastra o se gira la rueda: aquí, al momento;
	// al diseño, cada `EMPUJE_MS` y al acabar.
	const [vivo, setVivo] = useState<ArtFraming | null>(null);
	const vivoRef = useRef<ArtFraming | null>(null);
	const empuje = useRef<{ t: ReturnType<typeof setTimeout> | null; f: ArtFraming | null }>({
		t: null,
		f: null,
	});
	const finRueda = useRef<ReturnType<typeof setTimeout> | null>(null);
	const mover = (c: CambioEncuadre) => {
		const base = vivoRef.current ?? arte?.framing;
		if (!base || !arte) return;
		// El hueco de la última capa (cambia poco aunque cambien los textos);
		// sin capa (compuestas), uno típico.
		const [, , cw, ch] = capa?.arte ?? HUECO_TIPICO;
		const f = moverEncuadre(base, c, {
			w: cw * CARTA_MM.w,
			h: ch * CARTA_MM.h,
			iw: Math.max(1, arte.image.width),
			ih: Math.max(1, arte.image.height),
		});
		vivoRef.current = f;
		setVivo(f);
		const e = empuje.current;
		if (e.t) {
			e.f = f;
			return;
		}
		onEncuadre(f);
		e.t = setTimeout(() => {
			e.t = null;
			if (e.f) onEncuadre(e.f);
			e.f = null;
		}, EMPUJE_MS);
	};
	const acabar = () => {
		const e = empuje.current;
		if (e.t) clearTimeout(e.t);
		e.t = null;
		e.f = null;
		const f = vivoRef.current;
		if (f) onEncuadre(f);
		vivoRef.current = null;
		setVivo(null);
	};
	useEffect(
		() => () => {
			if (empuje.current.t) clearTimeout(empuje.current.t);
			if (finRueda.current) clearTimeout(finRueda.current);
		},
		[],
	);
	// Si cambia el encuadre (los deslizadores del formulario), se pide la capa.
	const framingClave = arte ? JSON.stringify(arte.framing) : null;
	useEffect(() => {
		if (framingClave) pedirCapa();
	}, [framingClave]);

	// Mientras cambia seguido (escribir, mover un deslizador) se pide más
	// pequeña para que siga al ratón; al parar, la buena.
	const rafaga = useRafaga(huella);
	const url = huella
		? urlDiseno(huella, rafaga ? (Math.min(ancho, 488) as AnchoMiniatura) : ancho)
		: null;
	const exacta = huella && alDia ? urlDiseno(huella, ancho) : null;
	const [capaLista, setCapaLista] = useState(false);
	const enVivo = capaVale && capaLista && !!arte;
	// Con la ilustración moviéndose en la pantalla, Rust no pinta los pasos
	// intermedios: solo la buena al parar.
	const congelada = useRef<string | null>(null);
	if (!(enVivo && (rafaga || vivo !== null))) congelada.current = url;
	const { mostrada, cargando: pintando } = useImagenViva(congelada.current);
	const componer = enVivo && (vivo !== null || mostrada !== exacta);

	const alBajar = (e: React.PointerEvent) => {
		if (!tieneArte) return;
		pedirCapa();
		(e.target as HTMLElement).setPointerCapture(e.pointerId);
		arrastre.current = { x: e.clientX, y: e.clientY };
	};
	const alMover = (e: React.PointerEvent) => {
		const a = arrastre.current;
		const m = marco.current;
		if (!a || !m) return;
		const r = m.getBoundingClientRect();
		mover({ dx: (e.clientX - a.x) / r.width, dy: (e.clientY - a.y) / r.height });
		arrastre.current = { x: e.clientX, y: e.clientY };
	};
	const alSoltar = () => {
		if (!arrastre.current) return;
		arrastre.current = null;
		acabar();
	};
	const alRodar = (e: React.WheelEvent) => {
		if (!tieneArte || e.ctrlKey) return;
		pedirCapa();
		mover({ zoom: e.deltaY < 0 ? 0.04 : -0.04 });
		if (finRueda.current) clearTimeout(finRueda.current);
		finRueda.current = setTimeout(() => {
			finRueda.current = null;
			if (!arrastre.current) acabar();
		}, FIN_RUEDA_MS);
	};

	const caras = design?.faces.length ?? 0;
	return (
		<div className="flex h-full flex-col">
			<div className="flex shrink-0 items-center gap-2.5 border-b px-3 py-2">
				<SelectorIdioma
					valor={lang}
					onCambio={setLang}
					ancho="w-40"
					ariaLabel={t("Idioma de las cartas diseñadas")}
				/>
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
					onPointerEnter={pedirCapa}
					onPointerDown={alBajar}
					onPointerMove={alMover}
					onPointerUp={alSoltar}
					onPointerCancel={alSoltar}
					onWheel={alRodar}
				>
					{mostrada ? (
						<img
							src={mostrada}
							alt=""
							draggable={false}
							className="size-full object-contain drop-shadow-lg"
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
					{capaVale && capa && arte && (
						<ArteEnVivo
							capa={capa}
							urlCapa={urlCapa(capa.huella, capa.ancho)}
							urlArte={urlArte(arte.image.blob, ancho >= 745 ? 2000 : 1000)}
							framing={vivo ?? arte.framing}
							visible={componer}
							onLista={setCapaLista}
						/>
					)}
					{seguro && mostrada && (
						<div
							className="pointer-events-none absolute rounded-[3%] border border-dashed border-cyan-400/90"
							style={{ inset: `${(3 / 88) * 100}% ${(3 / 63) * 100}%` }}
						/>
					)}
					{pintando && !componer && (
						<Loader2 className="absolute top-2 right-2 size-4 animate-spin text-white drop-shadow" />
					)}
					{tieneArte && mostrada && (
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
								<span className="min-w-0 flex-1">{textoAviso(a, t)}</span>
								{a.kind === "faltanLetras" && (
									<Button size="xs" variant="outline" onClick={() => setLang(a.lang)}>
										<Download />
										{t("Bajar las letras")}
									</Button>
								)}
							</li>
						))}
					</ul>
				</ScrollArea>
			)}
		</div>
	);
}
