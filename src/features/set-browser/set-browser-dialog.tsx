import { ListPlus, Loader2, Plus, Replace, Search } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { toast } from "sonner";
import type { CardSummary } from "@/bindings/CardSummary";
import type { SetInfo } from "@/bindings/SetInfo";
import { type Opcion, Selector } from "@/components/campos";
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
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import { cargarLista } from "@/features/decklist/acciones";
import { RaizScroll } from "@/features/preview/visibilidad";
import { RejillaCartas } from "@/features/variants/rejilla-cartas";
import { call } from "@/lib/ipc";
import { cn } from "@/lib/utils";
import { useDeck } from "@/stores/deck-store";
import { useUi } from "@/stores/ui-store";

/**
 * «Explorar sets» (Fase 8): elegir un set, filtrar por tipo, seleccionar
 * cartas y cargarlas en el deck o llevarlas a la lista.
 */
export function SetBrowserDialog() {
	const abierto = useUi((s) => s.dialogo?.tipo === "sets");
	return (
		<Dialog open={abierto} onOpenChange={(o) => !o && useUi.getState().abrir(null)}>
			{abierto && <Contenido />}
		</Dialog>
	);
}

/** Las cartas elegidas como líneas de lista con set y número exactos. */
function comoLista(cartas: CardSummary[]): string {
	return cartas.map((c) => `1 ${c.name} (${c.set.toUpperCase()}) ${c.collectorNumber}`).join("\n");
}

function Contenido() {
	const { t, mensaje } = useLang();
	const [sets, setSets] = useState<SetInfo[] | null>(null);
	const [filtroSet, setFiltroSet] = useState("");
	const [set, setSet] = useState<SetInfo | null>(null);
	const [tipo, setTipo] = useState("all");
	const [cartas, setCartas] = useState<CardSummary[] | null>(null);
	const [elegidas, setElegidas] = useState<Set<string>>(new Set());
	const [cargando, setCargando] = useState(false);
	const [viewport, setViewport] = useState<HTMLDivElement | null>(null);

	useEffect(() => {
		call("sets_list", {})
			.then(setSets)
			.catch((e) => toast.error(mensaje(e)));
	}, [mensaje]);

	useEffect(() => {
		if (!set) return;
		let vigente = true;
		setCargando(true);
		setElegidas(new Set());
		call("set_cards", { code: set.code, typeFilter: tipo === "all" ? null : tipo })
			.then((r) => vigente && setCartas(r))
			.catch((e) => {
				if (!vigente) return;
				setCartas([]);
				if ((e as { code?: string }).code !== "NotFound") toast.error(mensaje(e));
			})
			.finally(() => vigente && setCargando(false));
		return () => {
			vigente = false;
		};
	}, [set, tipo, mensaje]);

	const visibles = useMemo(() => {
		const f = filtroSet.trim().toLowerCase();
		return (sets ?? []).filter(
			(s) => !f || s.name.toLowerCase().includes(f) || s.code.toLowerCase() === f,
		);
	}, [sets, filtroSet]);

	const tipos: Opcion<string>[] = [
		{ valor: "all", etiqueta: t("Todos los tipos") },
		{ valor: "creature", etiqueta: t("Criaturas") },
		{ valor: "instant", etiqueta: t("Instantáneos") },
		{ valor: "sorcery", etiqueta: t("Conjuros") },
		{ valor: "artifact", etiqueta: t("Artefactos") },
		{ valor: "enchantment", etiqueta: t("Encantamientos") },
		{ valor: "planeswalker", etiqueta: t("Planeswalkers") },
		{ valor: "land", etiqueta: t("Tierras") },
		{ valor: "token", etiqueta: "Tokens" },
	];

	const seleccion = (cartas ?? []).filter((c) => elegidas.has(c.id));
	const cerrar = () => useUi.getState().abrir(null);

	const alLista = (reemplazar: boolean) => {
		const texto = comoLista(seleccion);
		const deck = useDeck.getState();
		const actual = deck.deck?.decklistText ?? "";
		deck.setSinHistorial({
			decklistText:
				reemplazar || !actual.trim() ? texto : `${actual.replace(/\s+$/, "")}\n${texto}`,
		});
		useUi.getState().anunciar(t("{n} cartas llevadas a la lista", { n: seleccion.length }));
		cerrar();
	};

	const cargar = () => {
		const texto = comoLista(seleccion);
		cerrar();
		cargarLista(texto, "anadir", t)
			.then((aviso) => aviso && toast.warning(aviso))
			.catch((e) => toast.error(mensaje(e)));
	};

	return (
		<DialogContent className="flex h-[88vh] flex-col sm:max-w-6xl" cambiosSinGuardar={false}>
			<DialogHeader>
				<DialogTitle>{t("Explorar sets")}</DialogTitle>
				<DialogDescription>
					{set
						? `${set.name} · ${set.code.toUpperCase()} · ${set.releasedAt ?? ""}`
						: t("Elige un set de la lista.")}
				</DialogDescription>
			</DialogHeader>
			<div className="flex min-h-0 flex-1 gap-4">
				<div className="flex w-64 shrink-0 flex-col gap-2" data-filtro="">
					<div className="relative">
						<Search className="pointer-events-none absolute top-2 left-2 size-3.5 text-muted-foreground" />
						<Input
							value={filtroSet}
							onChange={(e) => setFiltroSet(e.target.value)}
							placeholder={t("Buscar set…")}
							className="h-7 pl-7 text-xs"
							type="search"
						/>
					</div>
					<ScrollArea className="min-h-0 flex-1 rounded-md border">
						{!sets ? (
							<div className="flex justify-center py-10">
								<Loader2 className="size-4 animate-spin text-muted-foreground" />
							</div>
						) : (
							<div className="flex flex-col p-1">
								{visibles.map((s) => (
									<button
										key={s.code}
										type="button"
										onClick={() => setSet(s)}
										className={cn(
											"flex items-center gap-2 rounded-sm px-2 py-1 text-left text-xs hover:bg-accent",
											set?.code === s.code && "bg-primary/15 font-medium",
										)}
									>
										<span className="w-10 shrink-0 font-mono text-[10px] text-muted-foreground uppercase">
											{s.code}
										</span>
										<span className="min-w-0 flex-1 truncate">{s.name}</span>
										<span className="text-[10px] text-muted-foreground">{s.cardCount}</span>
									</button>
								))}
							</div>
						)}
					</ScrollArea>
				</div>
				<div className="flex min-w-0 flex-1 flex-col gap-2">
					<div className="flex items-center gap-2.5" data-filtro="">
						<Selector valor={tipo} opciones={tipos} onChange={setTipo} ancho="w-44" />
						{cartas && cartas.length > 0 && (
							<>
								<Button
									size="xs"
									variant="ghost"
									onClick={() => setElegidas(new Set(cartas.map((c) => c.id)))}
								>
									{t("Elegir todas")}
								</Button>
								<Button
									size="xs"
									variant="ghost"
									onClick={() => setElegidas(new Set())}
									disabled={!elegidas.size}
								>
									{t("Ninguna")}
								</Button>
							</>
						)}
						{cargando && <Loader2 className="size-4 animate-spin text-muted-foreground" />}
					</div>
					<ScrollArea className="min-h-0 flex-1" onViewportChange={setViewport}>
						<RaizScroll.Provider value={viewport}>
							{cartas && cartas.length === 0 && !cargando ? (
								<p className="py-16 text-center text-xs text-muted-foreground">
									{t("Sin resultados.")}
								</p>
							) : (
								<RejillaCartas
									cartas={cartas ?? []}
									elegidas={elegidas}
									onElegir={(c) =>
										setElegidas((e) => {
											const n = new Set(e);
											if (n.has(c.id)) n.delete(c.id);
											else n.add(c.id);
											return n;
										})
									}
								/>
							)}
						</RaizScroll.Provider>
					</ScrollArea>
				</div>
			</div>
			<DialogFooter className="items-center">
				<span className="mr-auto text-xs text-muted-foreground">
					{t("{n} elegidas", { n: seleccion.length })}
				</span>
				<Button
					size="sm"
					variant="outline"
					disabled={!seleccion.length}
					onClick={() => alLista(true)}
				>
					<Replace />
					{t("Reemplazar lista")}
				</Button>
				<Button
					size="sm"
					variant="outline"
					disabled={!seleccion.length}
					onClick={() => alLista(false)}
				>
					<ListPlus />
					{t("Añadir a la lista")}
				</Button>
				<Button size="sm" disabled={!seleccion.length} onClick={cargar}>
					<Plus />
					{t("Cargar cartas")}
				</Button>
			</DialogFooter>
		</DialogContent>
	);
}
