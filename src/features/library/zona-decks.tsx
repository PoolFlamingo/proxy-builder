import { type ReactNode, useMemo, useState } from "react";
import type { DeckSummary } from "@/bindings/DeckSummary";
import { Icono } from "@/components/icono";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import {
	Empty,
	EmptyContent,
	EmptyDescription,
	EmptyHeader,
	EmptyMedia,
	EmptyTitle,
} from "@/components/ui/empty";
import { ScrollArea } from "@/components/ui/scroll-area";
import { RaizScroll } from "@/features/preview/visibilidad";
import { cn } from "@/lib/utils";
import { useUi } from "@/stores/ui-store";
import { FiltrosActivos } from "./barra-herramientas";
import { agrupar, type ClaveGrupo, filtrar, type Grupo, hayFiltros, ordenar } from "./organizar";
import { ChipEtiqueta, CLASES, IDIOMAS, Identidad, nombreColores, useDinero } from "./piezas";
import { useAncla, useFiltros, usePrefs } from "./preferencias";
import { useBiblioteca } from "./store";
import { VistaCuadricula } from "./vista-cuadricula";
import { VistaLista } from "./vista-lista";

export type Organizados = {
	visibles: DeckSummary[];
	grupos: Grupo[];
	plegados: Set<string>;
	plegar: (id: string) => void;
	/** Los ids en el orden de la pantalla (sin repetir ni los de grupos plegados). */
	enPantalla: string[];
};

/** Los decks de la biblioteca filtrados, ordenados y repartidos en grupos con lo que dice la pantalla. */
export function useOrganizados(): Organizados {
	const decks = useBiblioteca((s) => s.decks);
	const detalles = useBiblioteca((s) => s.detalles);
	const etiquetas = useBiblioteca((s) => s.etiquetas);
	const prefs = usePrefs();
	const { filtros } = useFiltros();
	const [plegados, setPlegados] = useState<Set<string>>(new Set());

	const visibles = useMemo(
		() =>
			ordenar(
				filtrar(decks ?? [], detalles, etiquetas, filtros),
				detalles,
				prefs.orden,
				prefs.ascendente,
				prefs.favoritosArriba,
			),
		[decks, detalles, etiquetas, filtros, prefs.orden, prefs.ascendente, prefs.favoritosArriba],
	);
	const grupos = useMemo(
		() => agrupar(visibles, detalles, etiquetas, prefs.agrupar),
		[visibles, detalles, etiquetas, prefs.agrupar],
	);
	const enPantalla = useMemo(
		() => [
			...new Set(
				grupos.filter((g) => !plegados.has(g.id)).flatMap((g) => g.decks.map((d) => d.id)),
			),
		],
		[grupos, plegados],
	);
	const plegar = (id: string) =>
		setPlegados((p) => {
			const n = new Set(p);
			if (n.has(id)) n.delete(id);
			else n.add(id);
			return n;
		});
	return { visibles, grupos, plegados, plegar, enPantalla };
}

/**
 * Lo que hay bajo la barra: las fichas de los filtros, los decks (en grupos,
 * en cuadrícula o en lista) y el pie. `flotante` va encima, en el borde de
 * abajo (la barra de la selección de la biblioteca).
 */
export function ZonaDecks({ org, flotante }: { org: Organizados; flotante?: ReactNode }) {
	const { t } = useLang();
	const decks = useBiblioteca((s) => s.decks);
	const contando = useBiblioteca((s) => s.contando);
	const prefs = usePrefs();
	const { filtros, limpiar } = useFiltros();
	const ancla = useAncla("decks");
	const [viewport, setViewport] = useState<HTMLDivElement | null>(null);
	const totalDecks = decks?.length ?? 0;
	const Vista = prefs.vista === "lista" ? VistaLista : VistaCuadricula;
	const conGrupos = prefs.agrupar !== "ninguno";
	return (
		<div className="relative flex min-w-0 flex-1 flex-col" data-tutorial={ancla}>
			<FiltrosActivos />
			{flotante}
			<ScrollArea className="min-h-0 flex-1" onViewportChange={setViewport}>
				<RaizScroll.Provider value={viewport}>
					{decks === null ? null : totalDecks === 0 ? (
						<Empty className="mt-16">
							<EmptyHeader>
								<EmptyMedia variant="icon">
									<Icono icon="lucide:layers" />
								</EmptyMedia>
								<EmptyTitle>{t("No hay decks guardados")}</EmptyTitle>
								<EmptyDescription>
									{t(
										"Los decks se guardan solos mientras trabajas: pega una lista o añade cartas en la pestaña «Deck».",
									)}
								</EmptyDescription>
							</EmptyHeader>
							<EmptyContent>
								<Button size="sm" onClick={() => useUi.getState().setVista("deck")}>
									<Icono icon="lucide:plus" className="size-4" />
									{t("Empezar un deck")}
								</Button>
							</EmptyContent>
						</Empty>
					) : org.visibles.length === 0 ? (
						<Empty className="mt-16">
							<EmptyHeader>
								<EmptyMedia variant="icon">
									<Icono icon="lucide:filter-x" />
								</EmptyMedia>
								<EmptyTitle>{t("Ningún deck coincide")}</EmptyTitle>
								<EmptyDescription>
									{contando
										? t("Aún se están calculando los colores y precios de los decks.")
										: t("Prueba con otra búsqueda o quita algún filtro.")}
								</EmptyDescription>
							</EmptyHeader>
							{hayFiltros(filtros) && (
								<EmptyContent>
									<Button size="sm" variant="outline" onClick={limpiar}>
										<Icono icon="lucide:x" />
										{t("Quitar los filtros")}
									</Button>
								</EmptyContent>
							)}
						</Empty>
					) : (
						<div className="flex flex-col gap-2.5 p-2.5">
							{org.grupos.map((g) =>
								conGrupos ? (
									<section key={g.id} className="flex flex-col gap-2.5">
										<button
											type="button"
											onClick={() => org.plegar(g.id)}
											className="flex items-center gap-2 self-start rounded-md px-1 py-0.5 text-xs font-semibold hover:bg-accent"
										>
											<Icono
												icon="lucide:chevron-right"
												className={cn(
													"size-3.5 text-muted-foreground transition-transform",
													!org.plegados.has(g.id) && "rotate-90",
												)}
											/>
											<TituloGrupo clave={g.clave} contando={contando} />
											<span className="font-mono text-[10px] font-normal text-muted-foreground tabular-nums">
												{g.decks.length}
											</span>
										</button>
										{!org.plegados.has(g.id) && <Vista decks={g.decks} />}
									</section>
								) : (
									<Vista key={g.id} decks={g.decks} />
								),
							)}
						</div>
					)}
				</RaizScroll.Provider>
			</ScrollArea>
			<Pie visibles={org.visibles} total={totalDecks} />
		</div>
	);
}

/** El título de un grupo según de qué es. */
function TituloGrupo({ clave: c, contando }: { clave: ClaveGrupo; contando: boolean }) {
	const { t } = useLang();
	switch (c.tipo) {
		case "todos":
			return <span>{t("Todos los decks")}</span>;
		case "etiqueta":
			return <ChipEtiqueta tag={c.tag} className="text-[11px]" />;
		case "sinEtiqueta":
			return <span className="text-muted-foreground">{t("Sin etiqueta")}</span>;
		case "color":
			return (
				<span className="flex items-center gap-1.5">
					<Identidad colores={c.colores} />
					{t(nombreColores(c.colores))}
				</span>
			);
		case "clase":
			return <span>{t(CLASES[c.clase])}</span>;
		case "idioma":
			return c.idioma ? (
				<span>{t(IDIOMAS[c.idioma] ?? c.idioma.toUpperCase())}</span>
			) : (
				<span className="text-muted-foreground">{t("Sin idioma")}</span>
			);
		case "sinDatos":
			return (
				<span className="text-muted-foreground">
					{contando ? t("Calculando…") : t("Sin datos")}
				</span>
			);
	}
}

/** Abajo: cuántos se ven, cuántas cartas suman y lo que costarían. */
function Pie({ visibles, total }: { visibles: DeckSummary[]; total: number }) {
	const { t } = useLang();
	const detalles = useBiblioteca((s) => s.detalles);
	const contando = useBiblioteca((s) => s.contando);
	const dinero = useDinero();
	let cartas = 0;
	let eur = 0;
	let usd = 0;
	let conEur = false;
	for (const d of visibles) {
		cartas += d.cardCount;
		const x = detalles[d.id];
		if (x?.priceEur != null) {
			eur += x.priceEur;
			conEur = true;
		}
		if (x?.priceUsd != null) usd += x.priceUsd;
	}
	const precio = conEur ? dinero(eur, "EUR") : usd > 0 ? dinero(usd, "USD") : null;
	const partes: [string, ReactNode][] = [
		[
			"decks",
			visibles.length === total
				? t("{n} decks", { n: total })
				: t("Mostrando {a} de {b} decks", { a: visibles.length, b: total }),
		],
		["cartas", t("{n} cartas", { n: cartas })],
	];
	if (precio) partes.push(["precio", `≈ ${precio}`]);
	return (
		<footer className="flex items-center gap-2 border-t px-3 py-1 text-[10px] text-muted-foreground">
			{partes.map(([k, p], i) => (
				<span key={k} className="flex items-center gap-2">
					{i > 0 && <span aria-hidden>·</span>}
					{p}
				</span>
			))}
			{contando && (
				<span className="ml-auto animate-pulse">{t("Calculando colores y precios…")}</span>
			)}
		</footer>
	);
}
