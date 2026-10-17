import { Check, Layers, Plus } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { toast } from "sonner";
import type { DeckCard } from "@/bindings/DeckCard";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import {
	Dialog,
	DialogContent,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import { RaizScroll, useVisible } from "@/features/preview/visibilidad";
import { urlMiniatura } from "@/lib/image-url";
import { call } from "@/lib/ipc";
import { cn } from "@/lib/utils";
import { useDeck } from "@/stores/deck-store";
import {
	ChipEtiqueta,
	CLASES,
	IDIOMAS,
	Identidad,
	nombreColores,
	useDinero,
	useFecha,
} from "./piezas";
import { useBiblioteca } from "./store";
import { PORTADA } from "./vista-cuadricula";

/**
 * Las propiedades de un deck en la biblioteca: nombre, etiquetas, notas y la
 * carta de la portada, con un resumen de lo que lleva.
 */
export function PropiedadesDialog({
	id,
	onCerrar,
	onNuevaEtiqueta,
}: {
	id: string | null;
	onCerrar: () => void;
	onNuevaEtiqueta: (para: string[]) => void;
}) {
	const { t, mensaje } = useLang();
	const deck = useBiblioteca((s) => s.decks?.find((d) => d.id === id) ?? null);
	const det = useBiblioteca((s) => (id ? s.detalles[id] : undefined));
	const etiquetas = useBiblioteca((s) => s.etiquetas);
	const fecha = useFecha();
	const dinero = useDinero();
	const [nombre, setNombre] = useState("");
	const [notas, setNotas] = useState("");
	const [portada, setPortada] = useState<string | null>(null);
	const [cartas, setCartas] = useState<DeckCard[] | null>(null);
	const [viewport, setViewport] = useState<HTMLDivElement | null>(null);

	// Al abrir: lo guardado y las cartas (para elegir portada).
	useEffect(() => {
		if (!id) return;
		const d = useBiblioteca.getState().decks?.find((x) => x.id === id);
		setNombre(d?.name ?? "");
		setNotas(d?.notes ?? "");
		setPortada(d?.coverCardId ?? null);
		setCartas(null);
		call("deck_peek", { id })
			.then((x) => setCartas(x.cards))
			.catch((e) => toast.error(mensaje(e)));
	}, [id, mensaje]);

	// Una carta por imagen (las copias tienen la misma).
	const opciones = useMemo(() => {
		const vistas = new Set<string>();
		return (cartas ?? []).filter((c) => {
			const b = c.front?.blob;
			if (!b || vistas.has(b)) return false;
			vistas.add(b);
			return true;
		});
	}, [cartas]);

	if (!deck) return null;
	const primera = opciones[0]?.id ?? null;
	const elegida = portada && opciones.some((c) => c.id === portada) ? portada : primera;

	const guardar = async () => {
		const b = useBiblioteca.getState();
		try {
			const limpio = nombre.trim();
			if (limpio && limpio !== deck.name) {
				await call("deck_rename", { id: deck.id, name: limpio });
				if (deck.id === useDeck.getState().deck?.id)
					useDeck.getState().setSinHistorial({ name: limpio });
			}
			if (notas.trim() !== deck.notes) await call("deck_notes_set", { id: deck.id, notes: notas });
			const nueva = portada === primera ? null : portada;
			if (nueva !== deck.coverCardId) await call("deck_cover_set", { id: deck.id, card: nueva });
			await b.refrescar();
			onCerrar();
		} catch (e) {
			toast.error(mensaje(e));
		}
	};

	return (
		<Dialog open={id !== null} onOpenChange={(o) => !o && onCerrar()}>
			<DialogContent className="sm:max-w-2xl" cambiosSinGuardar={false}>
				<DialogHeader>
					<DialogTitle>{t("Propiedades del deck")}</DialogTitle>
				</DialogHeader>
				<div className="grid gap-4 sm:grid-cols-[1fr_220px]">
					<div className="flex min-w-0 flex-col gap-3">
						<label className="flex flex-col gap-1 text-xs">
							<span className="text-muted-foreground">{t("Nombre")}</span>
							<Input
								value={nombre}
								onChange={(e) => setNombre(e.target.value)}
								className="text-sm"
							/>
						</label>
						<div className="flex flex-col gap-1 text-xs">
							<span className="text-muted-foreground">{t("Etiquetas")}</span>
							<div className="flex flex-wrap gap-1.5">
								{etiquetas.map((tag) => {
									const tiene = deck.tags.includes(tag.id);
									return (
										<button
											type="button"
											key={tag.id}
											onClick={() =>
												void useBiblioteca
													.getState()
													.etiquetar([deck.id], tiene ? [] : [tag.id], tiene ? [tag.id] : [])
											}
											className={cn(
												"rounded-full transition-opacity",
												!tiene && "opacity-40 hover:opacity-80",
											)}
										>
											<ChipEtiqueta tag={tag} className="text-[11px]" />
										</button>
									);
								})}
								<Button
									size="xs"
									variant="outline"
									className="h-5 rounded-full px-2 text-[10px]"
									onClick={() => onNuevaEtiqueta([deck.id])}
								>
									<Plus />
									{t("Nueva")}
								</Button>
							</div>
						</div>
						<label className="flex flex-col gap-1 text-xs">
							<span className="text-muted-foreground">{t("Notas")}</span>
							<textarea
								value={notas}
								maxLength={4000}
								onChange={(e) => setNotas(e.target.value)}
								rows={4}
								placeholder={t("Para qué es, qué falta por imprimir, con quién se juega…")}
								className="min-h-20 resize-y rounded-md border border-input bg-transparent px-2.5 py-2 text-xs shadow-xs outline-none focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/50 dark:bg-input/30"
							/>
						</label>
						<div className="flex flex-col gap-1 text-xs">
							<span className="text-muted-foreground">{t("Portada")}</span>
							<ScrollArea
								className="rounded-lg border"
								viewportClassName="max-h-48"
								onViewportChange={setViewport}
							>
								<RaizScroll.Provider value={viewport}>
									{cartas === null ? (
										<div className="flex h-24 items-center justify-center text-muted-foreground">
											<Layers className="size-5 animate-pulse" />
										</div>
									) : (
										<div className="grid grid-cols-[repeat(auto-fill,minmax(56px,1fr))] gap-1.5 p-1.5">
											{opciones.map((c) => (
												<OpcionPortada
													key={c.id}
													carta={c}
													elegida={c.id === elegida}
													onElegir={() => setPortada(c.id)}
												/>
											))}
										</div>
									)}
								</RaizScroll.Provider>
							</ScrollArea>
						</div>
					</div>
					<Resumen
						filas={[
							[
								t("Cartas"),
								det
									? `${det.deckSize}${det.tokens ? ` + ${det.tokens}` : ""}`
									: String(deck.cardCount),
							],
							[t("Clase"), det ? t(CLASES[det.kind]) : "…"],
							[
								t("Colores"),
								det ? (
									<span
										key="colores"
										className="flex flex-wrap items-center justify-end gap-x-1.5 gap-y-0.5"
									>
										<Identidad colores={det.colorIdentity} />
										{t(nombreColores(det.colorIdentity))}
									</span>
								) : (
									"…"
								),
							],
							[t("Tierras"), det ? String(det.lands) : "…"],
							[t("Coste medio"), det ? det.averageCmc.toFixed(2) : "…"],
							[
								t("Precio estimado"),
								det
									? dinero(det.priceEur ?? det.priceUsd, det.priceEur !== null ? "EUR" : "USD")
									: "…",
							],
							[
								t("Idioma"),
								det
									? det.langs.map((l) => t(IDIOMAS[l] ?? l.toUpperCase())).join(", ") || "—"
									: "…",
							],
							[
								t("Imágenes propias"),
								det ? `${det.custom} (${det.customLinked} ${t("vinculadas")})` : "…",
							],
							[t("Diseñadas"), det ? `${det.designed} (${det.designedReady} ${t("listas")})` : "…"],
							[t("Doble cara"), det ? String(det.dfc) : "…"],
							[t("Creado"), fecha(deck.createdAt)],
							[t("Modificado"), fecha(deck.updatedAt)],
							[t("Abierto"), fecha(deck.openedAt)],
						]}
					/>
				</div>
				<DialogFooter>
					<Button size="sm" variant="outline" onClick={onCerrar}>
						{t("Cancelar")}
					</Button>
					<Button size="sm" onClick={() => void guardar()} disabled={!nombre.trim()}>
						{t("Guardar")}
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	);
}

function Resumen({ filas }: { filas: [string, React.ReactNode][] }) {
	return (
		<dl className="flex flex-col gap-1.5 rounded-lg border bg-card/40 p-2.5 text-xs">
			{filas.map(([k, v]) => (
				<div key={k} className="flex items-baseline justify-between gap-2">
					<dt className="shrink-0 text-muted-foreground">{k}</dt>
					<dd className={cn("min-w-0 text-right", typeof v === "string" && "truncate")}>{v}</dd>
				</div>
			))}
		</dl>
	);
}

function OpcionPortada({
	carta,
	elegida,
	onElegir,
}: {
	carta: DeckCard;
	elegida: boolean;
	onElegir: () => void;
}) {
	const [ref, setRef] = useState<HTMLButtonElement | null>(null);
	const visible = useVisible(ref);
	return (
		<button
			ref={setRef}
			type="button"
			onClick={onElegir}
			className={cn(
				"relative aspect-[63/88] overflow-hidden rounded-[4px] bg-muted ring-offset-1 ring-offset-background",
				elegida ? "ring-2 ring-primary" : "opacity-80 hover:opacity-100",
			)}
		>
			{visible && carta.front && (
				<img
					src={urlMiniatura(carta.front.blob, PORTADA, 146)}
					alt={carta.meta.name}
					loading="lazy"
					draggable={false}
					className="size-full object-cover"
				/>
			)}
			{elegida && (
				<span className="absolute right-0.5 bottom-0.5 rounded-full bg-primary p-0.5 text-primary-foreground">
					<Check className="size-2.5" />
				</span>
			)}
		</button>
	);
}
