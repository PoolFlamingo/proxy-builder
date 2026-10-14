import { Loader2, Wand2 } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { toast } from "sonner";
import type { Collection } from "@/bindings/Collection";
import type { DeckCard } from "@/bindings/DeckCard";
import { Fila, Selector } from "@/components/campos";
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
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { call } from "@/lib/ipc";
import { lanzarTrabajo } from "@/lib/trabajos";
import { useDeck } from "@/stores/deck-store";
import { useUi } from "@/stores/ui-store";
import { numerosUsados, scryfallDe } from "./grupos";
import { coleccionRecordada, recordarColeccion } from "./use-borrador";

/**
 * **Diseñar el deck**: un borrador para cada carta de Scryfall (o propia
 * vinculada) que aún no tiene diseño, con la colección elegida y el arte de
 * Scryfall de provisional. Rust los hace en orden y los numera; al terminar
 * entran en el deck de una vez (un solo «deshacer» los quita).
 */
export function LoteDialog() {
	const abierto = useUi((s) => s.dialogo?.tipo === "disenarDeck");
	return (
		<Dialog open={abierto} onOpenChange={(o) => !o && useUi.getState().abrir(null)}>
			{abierto && <Contenido />}
		</Dialog>
	);
}

const sinDiseno = (c: DeckCard) => !c.design && scryfallDe(c) !== null;

function Contenido() {
	const { t, mensaje } = useLang();
	const cartas = useDeck((s) => s.deck?.cards ?? []);
	const lang = useDeck((s) => s.deck?.settings.designLang ?? "es");
	const seleccion = useUi((s) => s.seleccion);
	const [alcance, setAlcance] = useState<"todo" | "seleccion">(
		seleccion.length > 1 ? "seleccion" : "todo",
	);
	const [colecciones, setColecciones] = useState<Collection[]>([]);
	const [coleccion, setColeccion] = useState<string>(coleccionRecordada() ?? "");
	const [progreso, setProgreso] = useState<{ hechas: number; total: number } | null>(null);
	const jobId = useRef<number | null>(null);

	useEffect(() => {
		call("collection_list", {})
			.then(setColecciones)
			.catch(() => {});
	}, []);

	const elegidas = useMemo(() => {
		const sel = new Set(seleccion);
		return cartas.filter((c) => sinDiseno(c) && (alcance === "todo" || sel.has(c.id)));
	}, [cartas, seleccion, alcance]);

	const empezar = async () => {
		const deck = useDeck.getState().deck;
		if (!deck || elegidas.length === 0) return;
		const hechas: DeckCard[] = [];
		const col = coleccion || null;
		recordarColeccion(col);
		setProgreso({ hechas: 0, total: elegidas.length });
		try {
			const { jobId: id, fin } = await lanzarTrabajo(
				"design://progress",
				"design://done",
				() =>
					call("design_drafts_for_deck", {
						cards: elegidas,
						usados: numerosUsados(deck.cards),
						collection: col,
						lang,
					}),
				(p) => {
					setProgreso({ hechas: p.done, total: p.total });
					if (p.card) hechas.push(p.card);
				},
			);
			jobId.current = id;
			const final = await fin;
			jobId.current = null;
			// Lo hecho entra de una vez, también si se canceló a medias.
			if (hechas.length > 0) {
				const porId = new Map(hechas.map((c) => [c.id, c]));
				useDeck.getState().setCartas((cs) => cs.map((c) => porId.get(c.id) ?? c));
			}
			if (final.error && final.error.code !== "Cancelled") toast.error(mensaje(final.error));
			useUi.getState().anunciar(t("{n} borradores hechos", { n: hechas.length }));
			useUi.getState().abrir(null);
		} catch (e) {
			toast.error(mensaje(e));
			setProgreso(null);
		}
	};

	const cancelar = () => {
		if (jobId.current !== null) void call("job_cancel", { jobId: jobId.current });
	};

	const enMarcha = progreso !== null;
	const ninguna = "__ninguna";
	return (
		<DialogContent className="sm:max-w-md" ocupado={enMarcha} alInterrumpir={cancelar}>
			<DialogHeader>
				<DialogTitle>{t("Diseñar el deck")}</DialogTitle>
				<DialogDescription className="text-xs">
					{t(
						"Un borrador para cada carta sin diseño: textos de la carta real, el arte de Scryfall de momento y su número por el orden del deck. Luego se revisan una a una.",
					)}
				</DialogDescription>
			</DialogHeader>
			<div className="flex flex-col gap-2.5">
				{seleccion.length > 1 && (
					<ToggleGroup
						type="single"
						size="sm"
						value={alcance}
						onValueChange={(v) => (v === "todo" || v === "seleccion") && setAlcance(v)}
						disabled={enMarcha}
					>
						<ToggleGroupItem value="todo" className="px-3 text-xs">
							{t("Todo el deck")}
						</ToggleGroupItem>
						<ToggleGroupItem value="seleccion" className="px-3 text-xs">
							{t("Solo las seleccionadas")}
						</ToggleGroupItem>
					</ToggleGroup>
				)}
				<Fila etiqueta={t("Colección")}>
					<Selector
						valor={coleccion || ninguna}
						opciones={[
							{ valor: ninguna, etiqueta: t("Ninguna") },
							...colecciones.map((c) => ({ valor: c.id, etiqueta: c.name })),
						]}
						onChange={(v) => setColeccion(v === ninguna ? "" : v)}
						ancho="w-48"
					/>
				</Fila>
				<p className="text-xs text-muted-foreground">
					{elegidas.length === 0
						? t("No hay cartas sin diseño que se puedan diseñar.")
						: t("{n} cartas sin diseño", { n: elegidas.length })}
				</p>
				{progreso && (
					<div className="flex flex-col gap-1.5">
						<Progress value={(progreso.hechas * 100) / Math.max(1, progreso.total)} />
						<span className="text-[11px] text-muted-foreground">
							{t("{hechas} de {total}", { hechas: progreso.hechas, total: progreso.total })}
						</span>
					</div>
				)}
			</div>
			<DialogFooter>
				{enMarcha ? (
					<Button size="sm" variant="outline" onClick={cancelar}>
						{t("Cancelar")}
					</Button>
				) : (
					<Button size="sm" onClick={empezar} disabled={elegidas.length === 0}>
						{enMarcha ? <Loader2 className="animate-spin" /> : <Wand2 />}
						{t("Hacer los borradores")}
					</Button>
				)}
			</DialogFooter>
		</DialogContent>
	);
}
