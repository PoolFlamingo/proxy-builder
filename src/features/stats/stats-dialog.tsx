import { Loader2 } from "lucide-react";
import { useEffect, useState } from "react";
import type { DeckKind } from "@/bindings/DeckKind";
import type { DeckStats } from "@/bindings/DeckStats";
import { useLang } from "@/components/language-provider";
import {
	Dialog,
	DialogContent,
	DialogCuerpo,
	DialogDescription,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "@/components/ui/select";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { call } from "@/lib/ipc";
import { useDeck } from "@/stores/deck-store";
import { useUi } from "@/stores/ui-store";
import { Funciones } from "./funciones";
import { Legalidad } from "./legalidad";
import { Mana } from "./mana";
import { Precios } from "./precios";
import { Probabilidades } from "./probabilidades";
import { Resumen } from "./resumen";
import { CLASES_MAZO, type Moneda } from "./textos";
import { Tipos } from "./tipos";

type Pestana =
	| "resumen"
	| "mana"
	| "tipos"
	| "funciones"
	| "probabilidades"
	| "precios"
	| "legalidad";

/**
 * Estadísticas del deck: las calcula Rust con los datos de Scryfall (de la
 * caché; pide los que falten y refresca precios de más de un día). Aquí solo
 * se pintan, repartidas en pestañas.
 */
export function StatsDialog() {
	const abierto = useUi((s) => s.dialogo?.tipo === "estadisticas");
	return (
		<Dialog open={abierto} onOpenChange={(o) => !o && useUi.getState().abrir(null)}>
			{abierto && <Contenido />}
		</Dialog>
	);
}

function Contenido() {
	const { t, mensaje } = useLang();
	const cartas = useDeck((s) => s.deck?.cards);
	const [kind, setKind] = useState<DeckKind | null>(null);
	const [s, setS] = useState<DeckStats | null>(null);
	const [cargando, setCargando] = useState(true);
	const [error, setError] = useState<unknown>(null);
	const [pestana, setPestana] = useState<Pestana>("resumen");
	const [moneda, setMoneda] = useState<Moneda>("eur");

	useEffect(() => {
		if (!cartas) return;
		let vigente = true;
		setCargando(true);
		call("deck_stats", { cards: cartas, kind })
			.then((r) => vigente && setS(r))
			.catch((e) => vigente && setError(e))
			.finally(() => vigente && setCargando(false));
		return () => {
			vigente = false;
		};
	}, [cartas, kind]);

	const pestanas: { valor: Pestana; texto: string }[] = [
		{ valor: "resumen", texto: t("Resumen") },
		{ valor: "mana", texto: t("Maná") },
		{ valor: "tipos", texto: t("Tipos") },
		{ valor: "funciones", texto: t("Funciones") },
		{ valor: "probabilidades", texto: t("Probabilidades") },
		{ valor: "precios", texto: t("Precios") },
		{ valor: "legalidad", texto: t("Legalidad") },
	];

	return (
		<DialogContent
			className="flex h-[88vh] flex-col sm:max-w-5xl [&_button[data-slot=tooltip-trigger]]:text-xs"
			cambiosSinGuardar={false}
		>
			<DialogHeader className="pr-6">
				<DialogTitle className="flex items-center gap-2">
					{t("Estadísticas del deck")}
					{cargando && s && <Loader2 className="size-4 animate-spin text-muted-foreground" />}
				</DialogTitle>
				<DialogDescription className="text-xs">
					{t(
						"Datos y precios de Scryfall. Las cartas propias cuentan si están vinculadas a su carta de Scryfall.",
					)}
				</DialogDescription>
			</DialogHeader>
			{error !== null ? (
				<p className="py-10 text-center text-xs text-destructive">{mensaje(error)}</p>
			) : !s ? (
				<div className="flex flex-1 flex-col items-center justify-center gap-2 text-xs text-muted-foreground">
					<Loader2 className="size-5 animate-spin" />
					{t("Leyendo los datos de las cartas…")}
				</div>
			) : (
				<Tabs
					value={pestana}
					onValueChange={(v) => setPestana(v as Pestana)}
					className="min-h-0 flex-1"
				>
					<div className="flex flex-wrap items-center justify-between gap-2">
						<TabsList>
							{pestanas.map((p) => (
								<TabsTrigger key={p.valor} value={p.valor}>
									{p.texto}
								</TabsTrigger>
							))}
						</TabsList>
						<div className="flex items-center gap-1.5">
							<Select
								value={kind ?? "auto"}
								onValueChange={(v) => setKind(v === "auto" ? null : (v as DeckKind))}
							>
								<SelectTrigger size="sm" className="h-7 w-60 text-xs">
									<SelectValue />
								</SelectTrigger>
								<SelectContent>
									<SelectItem value="auto" className="text-xs">
										{t("Automático: {c}", { c: t(CLASES_MAZO[s.detectedKind] ?? s.detectedKind) })}
									</SelectItem>
									{(["commander", "constructed", "limited"] as const).map((k) => (
										<SelectItem key={k} value={k} className="text-xs">
											{t(CLASES_MAZO[k])}
										</SelectItem>
									))}
								</SelectContent>
							</Select>
							<ToggleGroup
								type="single"
								variant="outline"
								size="sm"
								value={moneda}
								onValueChange={(v) => v && setMoneda(v as Moneda)}
							>
								<ToggleGroupItem value="eur" className="h-7 px-2 text-[11px]">
									€
								</ToggleGroupItem>
								<ToggleGroupItem value="usd" className="h-7 px-2 text-[11px]">
									$
								</ToggleGroupItem>
							</ToggleGroup>
						</div>
					</div>
					<DialogCuerpo>
						<TabsContent value="resumen">
							<Resumen s={s} moneda={moneda} />
						</TabsContent>
						<TabsContent value="mana">
							<Mana s={s} />
						</TabsContent>
						<TabsContent value="tipos">
							<Tipos s={s} />
						</TabsContent>
						<TabsContent value="funciones">
							<Funciones s={s} />
						</TabsContent>
						<TabsContent value="probabilidades">
							<Probabilidades s={s} />
						</TabsContent>
						<TabsContent value="precios">
							<Precios s={s} moneda={moneda} />
						</TabsContent>
						<TabsContent value="legalidad">
							<Legalidad s={s} />
						</TabsContent>
					</DialogCuerpo>
				</Tabs>
			)}
		</DialogContent>
	);
}
