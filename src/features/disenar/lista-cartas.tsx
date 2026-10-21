import { Link2Off, Search } from "lucide-react";
import { useMemo, useState } from "react";
import { useLang } from "@/components/language-provider";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import { WithTooltip } from "@/components/ui/with-tooltip";
import { parametros, urlMiniatura } from "@/lib/image-url";
import { textosDe } from "@/lib/por-idioma";
import { cn } from "@/lib/utils";
import { useDeck } from "@/stores/deck-store";
import { type EstadoDiseno, estadoDe, type Grupo } from "./grupos";

const ESTADOS: Record<EstadoDiseno, { clase: string }> = {
	sinVincular: { clase: "bg-muted text-muted-foreground" },
	sinDiseno: { clase: "bg-muted text-foreground/70" },
	borrador: { clase: "bg-amber-500/15 text-amber-700 dark:text-amber-300" },
	listo: { clase: "bg-emerald-500/15 text-emerald-700 dark:text-emerald-300" },
};

/** Las cartas del deck, una fila por carta (con sus copias), y su estado. */
export function ListaCartas({
	grupos,
	elegido,
	onElegir,
}: {
	grupos: Grupo[];
	elegido: string | null;
	onElegir: (g: Grupo) => void;
}) {
	const { t } = useLang();
	const ajustes = useDeck((s) => s.deck?.settings);
	const lang = ajustes?.designLang ?? "es";
	const [filtro, setFiltro] = useState("");
	const etiqueta: Record<EstadoDiseno, string> = {
		sinVincular: t("Sin vincular"),
		sinDiseno: t("Sin diseño"),
		borrador: t("Borrador"),
		listo: t("Listo"),
	};
	const visibles = useMemo(() => {
		const f = filtro.trim().toLowerCase();
		if (!f) return grupos;
		return grupos.filter((g) => {
			const c = g.cartas[0];
			const textos = c?.design?.faces[0]?.texts;
			// El nombre en cualquier idioma del diseño.
			const nombres = [c?.meta.name, ...Object.values(textos ?? {}).map((x) => x?.name)];
			return nombres.some((n) => n?.toLowerCase().includes(f));
		});
	}, [grupos, filtro]);
	const hechas = grupos.filter((g) => estadoDe(g) === "listo").length;

	return (
		<div className="flex h-full flex-col gap-2.5 p-2.5">
			<div className="relative">
				<Search className="pointer-events-none absolute top-1/2 left-2 size-3.5 -translate-y-1/2 text-muted-foreground" />
				<Input
					value={filtro}
					onChange={(e) => setFiltro(e.target.value)}
					placeholder={t("Buscar carta…")}
					className="h-8 pl-7 text-xs"
				/>
			</div>
			<p className="text-[11px] text-muted-foreground">
				{t("{hechas} de {total} listas", { hechas, total: grupos.length })}
			</p>
			<ScrollArea className="min-h-0 flex-1">
				<ul className="flex flex-col gap-1 p-1 pr-2.5">
					{visibles.map((g) => {
						const c = g.cartas[0];
						if (!c) return null;
						const estado = estadoDe(g);
						const textos = c.design?.faces[0]?.texts;
						const nombre =
							(textos && textosDe(textos, lang).name) ||
							textos?.es.name ||
							c.meta.name ||
							t("Carta sin nombre");
						const fila = (
							<button
								type="button"
								onClick={() => estado !== "sinVincular" && onElegir(g)}
								disabled={estado === "sinVincular"}
								className={cn(
									"flex w-full items-center gap-2.5 rounded-md p-1.5 text-left hover:bg-muted disabled:cursor-not-allowed disabled:opacity-55",
									elegido === g.clave && "bg-muted ring-1 ring-primary/50",
								)}
							>
								<div className="aspect-[63/88] w-9 shrink-0 overflow-hidden rounded-[3px] bg-muted">
									{c.front && ajustes && (
										<img
											src={urlMiniatura(c.front.blob, parametros(ajustes, c.edits, false, 0), 146)}
											alt=""
											loading="lazy"
											className="size-full object-cover"
										/>
									)}
								</div>
								<div className="min-w-0 flex-1">
									<div className="truncate text-xs font-medium">{nombre}</div>
									{c.design && c.meta.name && c.meta.name !== nombre && (
										<div className="truncate text-[10px] text-muted-foreground">{c.meta.name}</div>
									)}
									<div className="mt-0.5 flex items-center gap-1.5">
										<span
											className={cn(
												"rounded px-1.5 py-px text-[10px] font-medium",
												ESTADOS[estado].clase,
											)}
										>
											{etiqueta[estado]}
										</span>
										{g.cartas.length > 1 && (
											<span className="text-[10px] text-muted-foreground">×{g.cartas.length}</span>
										)}
									</div>
								</div>
								{estado === "sinVincular" && (
									<Link2Off className="size-3.5 text-muted-foreground" />
								)}
							</button>
						);
						return (
							<li key={g.clave}>
								{estado === "sinVincular" ? (
									<WithTooltip
										content={t(
											"Vincúlala primero con su carta de Scryfall (clic derecho en el deck → «Vincular con una carta…»).",
										)}
									>
										<span className="block">{fila}</span>
									</WithTooltip>
								) : (
									fila
								)}
							</li>
						);
					})}
				</ul>
			</ScrollArea>
		</div>
	);
}
