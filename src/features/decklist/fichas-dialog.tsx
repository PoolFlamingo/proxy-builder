import { useEffect, useState } from "react";
import { toast } from "sonner";
import type { DeckCard } from "@/bindings/DeckCard";
import type { FichaPropuesta } from "@/bindings/FichaPropuesta";
import { Icono } from "@/components/icono";
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
import { RaizScroll } from "@/features/preview/visibilidad";
import { RejillaCartas } from "@/features/variants/rejilla-cartas";
import { traerCarta } from "@/features/variants/traer";
import { insertar } from "@/lib/deck-ops";
import { call } from "@/lib/ipc";
import { useDeck } from "@/stores/deck-store";
import { useUi } from "@/stores/ui-store";

/**
 * **Detectar fichas** en el deck que ya hay: las que crean sus cartas (de
 * Scryfall o propias vinculadas) y que no tiene. Rust las busca; aquí se
 * revisan (todas marcadas de partida) y se añaden al final las elegidas.
 */
export function FichasDialog() {
	const { t, mensaje } = useLang();
	const abierto = useUi((s) => s.dialogo?.tipo === "fichas");
	const [propuestas, setPropuestas] = useState<FichaPropuesta[] | null>(null);
	const [elegidas, setElegidas] = useState<Set<string>>(new Set());
	const [anadiendo, setAnadiendo] = useState(false);
	const [viewport, setViewport] = useState<HTMLDivElement | null>(null);
	const cerrar = () => useUi.getState().abrir(null);

	useEffect(() => {
		if (!abierto) return;
		setPropuestas(null);
		const cards = useDeck.getState().deck?.cards ?? [];
		call("deck_tokens_detect", { cards })
			.then((p) => {
				setPropuestas(p);
				setElegidas(new Set(p.map((x) => x.carta.id)));
			})
			.catch((e) => {
				toast.error(mensaje(e));
				cerrar();
			});
	}, [abierto, mensaje]);

	const anadir = async () => {
		const elegir = (propuestas ?? []).filter((p) => elegidas.has(p.carta.id));
		if (elegir.length === 0) return;
		setAnadiendo(true);
		try {
			const nuevas: DeckCard[] = [];
			for (const p of elegir) nuevas.push(await traerCarta(p.carta));
			useDeck.getState().setCartas((cs) => insertar(cs, nuevas));
			useUi.getState().anunciar(t("{n} fichas añadidas", { n: nuevas.length }));
			cerrar();
		} catch (e) {
			toast.error(mensaje(e));
		} finally {
			setAnadiendo(false);
		}
	};

	const alternar = (id: string) =>
		setElegidas((s) => {
			const n = new Set(s);
			if (n.has(id)) n.delete(id);
			else n.add(id);
			return n;
		});

	return (
		<Dialog open={abierto} onOpenChange={(o) => !o && !anadiendo && cerrar()}>
			<DialogContent className="sm:max-w-3xl" cambiosSinGuardar={false}>
				<DialogHeader>
					<DialogTitle>{t("Detectar fichas")}</DialogTitle>
					<DialogDescription>
						{t(
							"Las fichas que crean las cartas del deck (también las propias vinculadas) y que aún no tiene. Desmarca las que no quieras.",
						)}
					</DialogDescription>
				</DialogHeader>
				<DialogCuerpo className="min-h-64" onViewportChange={setViewport}>
					<RaizScroll.Provider value={viewport}>
						{propuestas === null ? (
							<p className="flex items-center justify-center gap-2 py-16 text-xs text-muted-foreground">
								<Icono icon="lucide:loader-2" className="size-4 animate-spin" />
								{t("Buscando las fichas…")}
							</p>
						) : propuestas.length === 0 ? (
							<p className="py-16 text-center text-xs text-muted-foreground">
								{t("No falta ninguna ficha.")}
							</p>
						) : (
							<div className="flex flex-col gap-2.5">
								<RejillaCartas
									cartas={propuestas.map((p) => p.carta)}
									elegidas={elegidas}
									onElegir={(c) => alternar(c.id)}
								/>
								<ul className="flex flex-col gap-0.5 px-1.5 text-[11px] text-muted-foreground">
									{propuestas.map((p) => (
										<li key={p.carta.id} className="truncate">
											{t("{ficha}: la crea {carta}", {
												ficha: p.carta.typeLine ?? p.carta.name,
												carta: p.creadaPor,
											})}
										</li>
									))}
								</ul>
							</div>
						)}
					</RaizScroll.Provider>
				</DialogCuerpo>
				<DialogFooter>
					<Button size="sm" variant="outline" onClick={cerrar} disabled={anadiendo}>
						{t("Cancelar")}
					</Button>
					<Button size="sm" onClick={anadir} disabled={anadiendo || elegidas.size === 0}>
						{anadiendo ? (
							<Icono icon="lucide:loader-2" className="animate-spin" />
						) : (
							<Icono icon="lucide:plus" />
						)}
						{t("Añadir {n} fichas", { n: elegidas.size })}
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	);
}
