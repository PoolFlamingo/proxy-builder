import { Link2, Link2Off, Loader2, ScanText, Search } from "lucide-react";
import { useEffect, useState } from "react";
import { toast } from "sonner";
import type { CardSummary } from "@/bindings/CardSummary";
import type { DeckCard } from "@/bindings/DeckCard";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import {
	Dialog,
	DialogContent,
	DialogCuerpo,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { cartaPorId } from "@/features/preview/acciones-carta";
import { RaizScroll } from "@/features/preview/visibilidad";
import { EDICION_NEUTRA, parametros, urlMiniatura } from "@/lib/image-url";
import { call } from "@/lib/ipc";
import { useDeck } from "@/stores/deck-store";
import { useUi } from "@/stores/ui-store";
import { RejillaCartas } from "./rejilla-cartas";
import { desvincular, vincular } from "./vincular";

/**
 * «Vincular con Scryfall»: decir qué carta real es una imagen propia. Se
 * busca por nombre (de partida, el del fichero) o se lee la carta entera con OCR.
 */
export function VincularDialog() {
	const dialogo = useUi((s) => s.dialogo);
	const id = dialogo?.tipo === "vincular" ? dialogo.id : null;
	const carta = id ? cartaPorId(id) : undefined;
	return (
		<Dialog open={!!carta} onOpenChange={(o) => !o && useUi.getState().abrir(null)}>
			{carta && <Contenido carta={carta} />}
		</Dialog>
	);
}

function Contenido({ carta }: { carta: DeckCard }) {
	const { t, mensaje } = useLang();
	const ajustes = useDeck((s) => s.deck?.settings);
	const vinculada = carta.source.kind === "custom" ? carta.source.scryfallId : undefined;
	const [q, setQ] = useState(carta.meta.name);
	const [resultados, setResultados] = useState<CardSummary[] | null>(null);
	const [leido, setLeido] = useState<string[]>([]);
	const [elegida, setElegida] = useState<CardSummary | null>(null);
	const [ocupado, setOcupado] = useState<"buscar" | "ocr" | "vincular" | null>(null);
	const [ocrListo, setOcrListo] = useState(true);
	const [viewport, setViewport] = useState<HTMLDivElement | null>(null);

	useEffect(() => {
		call("ocr_ready", {})
			.then(setOcrListo)
			.catch(() => setOcrListo(false));
	}, []);

	const identificar = async (texto: string, ocr: boolean) => {
		if (!carta.front) return;
		setOcupado(ocr ? "ocr" : "buscar");
		try {
			const r = await call("card_identify", { blob: carta.front.blob, hint: texto || null, ocr });
			setResultados(r.candidates);
			if (ocr) {
				setLeido(r.lines);
				setOcrListo(true);
			}
			setElegida(r.candidates.find((c) => c.id === r.best) ?? r.candidates[0] ?? null);
		} catch (e) {
			toast.error(mensaje(e));
		} finally {
			setOcupado(null);
		}
	};

	// Al abrir, una primera búsqueda con el nombre que tenga.
	useEffect(() => {
		if (carta.meta.name.trim().length >= 3) void identificar(carta.meta.name, false);
		// Solo al abrir.
	}, []);

	/** Vincula con la elegida (o con la que se pase: el doble clic). */
	const aceptar = async (c: CardSummary | null = elegida) => {
		if (!c || ocupado !== null) return;
		setOcupado("vincular");
		try {
			const nombre = await vincular(carta.id, c.id);
			useUi.getState().anunciar(t("Vinculada con {carta}", { carta: nombre }));
			useUi.getState().abrir(null);
		} catch (e) {
			toast.error(mensaje(e));
			setOcupado(null);
		}
	};

	const imagen =
		carta.front && ajustes
			? urlMiniatura(carta.front.blob, parametros(ajustes, EDICION_NEUTRA, false, 0), 488)
			: null;

	return (
		<DialogContent className="flex max-h-[88vh] flex-col sm:max-w-5xl" ocupado={ocupado !== null}>
			<DialogHeader>
				<DialogTitle>{t("Vincular con Scryfall")}</DialogTitle>
				<DialogDescription>
					{t(
						"Di qué carta real es esta imagen propia: la imagen no cambia, pero la carta cuenta en las estadísticas y en la info. Se aplica a todas las copias con la misma imagen.",
					)}
				</DialogDescription>
			</DialogHeader>
			<div className="flex min-h-0 flex-1 gap-4">
				<div className="flex w-52 shrink-0 flex-col gap-2">
					<div className="aspect-[63/88] w-full overflow-hidden rounded-[4.5%] bg-muted">
						{imagen && <img src={imagen} alt="" className="size-full object-cover" />}
					</div>
					<Button
						size="sm"
						variant="outline"
						onClick={() => identificar(q, true)}
						disabled={ocupado !== null || !carta.front}
					>
						{ocupado === "ocr" ? <Loader2 className="animate-spin" /> : <ScanText />}
						{t("Detectar con OCR")}
					</Button>
					{!ocrListo && (
						<p className="text-[10px] leading-snug text-muted-foreground">
							{t(
								"La primera vez se descargan los modelos del OCR (unos 12 MB). Después funciona sin red.",
							)}
						</p>
					)}
					{leido.length > 0 && (
						<div className="text-[11px]">
							<div className="font-medium">{t("Leído en la carta:")}</div>
							{leido.map((l) => (
								<button
									key={l}
									type="button"
									className="block w-full truncate text-left text-muted-foreground hover:text-foreground"
									onClick={() => {
										setQ(l);
										void identificar(l, false);
									}}
								>
									«{l}»
								</button>
							))}
						</div>
					)}
				</div>
				<div className="flex min-w-0 flex-1 flex-col gap-2.5">
					<div className="flex gap-1.5" data-filtro="">
						<div className="relative flex-1">
							<Search className="pointer-events-none absolute top-2 left-2 size-4 text-muted-foreground" />
							<Input
								value={q}
								onChange={(e) => setQ(e.target.value)}
								onKeyDown={(e) => e.key === "Enter" && void identificar(q, false)}
								placeholder={t("Nombre de la carta (en cualquier idioma)…")}
								className="h-8 pl-8 text-xs"
							/>
						</div>
						<Button
							size="sm"
							onClick={() => identificar(q, false)}
							disabled={ocupado !== null || q.trim().length < 3}
						>
							{ocupado === "buscar" ? <Loader2 className="animate-spin" /> : <Search />}
							{t("Buscar")}
						</Button>
					</div>
					<DialogCuerpo className="min-h-64 flex-1" onViewportChange={setViewport}>
						<RaizScroll.Provider value={viewport}>
							{resultados === null ? (
								<p className="py-16 text-center text-xs text-muted-foreground">
									{t("Busca por nombre o pulsa «Detectar con OCR».")}
								</p>
							) : resultados.length === 0 ? (
								<p className="py-16 text-center text-xs text-muted-foreground">
									{t("Sin resultados.")}
								</p>
							) : (
								<RejillaCartas
									cartas={resultados}
									elegidas={new Set(elegida ? [elegida.id] : [])}
									onElegir={(c) => setElegida(c)}
									onDoble={(c) => {
										// Doble clic = elegirla y vincular.
										setElegida(c);
										void aceptar(c);
									}}
								/>
							)}
						</RaizScroll.Provider>
					</DialogCuerpo>
				</div>
			</div>
			<DialogFooter className="items-center sm:justify-between">
				{vinculada ? (
					<Button
						size="sm"
						variant="ghost"
						onClick={() => {
							desvincular(carta.id);
							useUi.getState().abrir(null);
						}}
					>
						<Link2Off />
						{t("Quitar el vínculo")}
					</Button>
				) : (
					<span />
				)}
				<Button size="sm" onClick={() => aceptar()} disabled={!elegida || ocupado !== null}>
					{ocupado === "vincular" ? <Loader2 className="animate-spin" /> : <Link2 />}
					{elegida
						? t("Vincular con {carta}", { carta: elegida.printedName ?? elegida.name })
						: t("Vincular")}
				</Button>
			</DialogFooter>
		</DialogContent>
	);
}
