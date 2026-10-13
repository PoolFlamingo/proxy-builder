import { Check, Loader2, X } from "lucide-react";
import { useEffect, useState } from "react";
import type { DeckStats } from "@/bindings/DeckStats";
import { paraTraducir, useLang } from "@/components/language-provider";
import {
	Dialog,
	DialogContent,
	DialogCuerpo,
	DialogDescription,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { GraficoBarras } from "@/components/ui/grafico-barras";
import { WithTooltip } from "@/components/ui/with-tooltip";
import { call } from "@/lib/ipc";
import { cn } from "@/lib/utils";
import { useDeck } from "@/stores/deck-store";
import { useUi } from "@/stores/ui-store";

const COLORES: Record<string, { nombre: string; clase: string }> = {
	W: { nombre: paraTraducir("Blanco"), clase: "bg-amber-100 text-amber-900" },
	U: { nombre: paraTraducir("Azul"), clase: "bg-sky-500 text-white" },
	B: { nombre: paraTraducir("Negro"), clase: "bg-neutral-800 text-white" },
	R: { nombre: paraTraducir("Rojo"), clase: "bg-red-600 text-white" },
	G: { nombre: paraTraducir("Verde"), clase: "bg-green-600 text-white" },
	C: { nombre: paraTraducir("Incoloro"), clase: "bg-stone-400 text-black" },
};

const TIPOS: Record<string, string> = {
	Creature: paraTraducir("Criaturas"),
	Planeswalker: paraTraducir("Planeswalkers"),
	Battle: paraTraducir("Batallas"),
	Instant: paraTraducir("Instantáneos"),
	Sorcery: paraTraducir("Conjuros"),
	Artifact: paraTraducir("Artefactos"),
	Enchantment: paraTraducir("Encantamientos"),
	Land: paraTraducir("Tierras"),
	Other: paraTraducir("Otros"),
};

const RAREZAS: Record<string, string> = {
	common: paraTraducir("Común"),
	uncommon: paraTraducir("Infrecuente"),
	rare: paraTraducir("Rara"),
	mythic: paraTraducir("Mítica"),
	special: paraTraducir("Especial"),
	bonus: paraTraducir("Bonus"),
};

/**
 * Estadísticas del deck (Fase 8): las calcula Rust con los datos de Scryfall
 * que ya tiene en caché (pide los que falten y refresca precios de más de un
 * día).
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
	const { t, mensaje, lang } = useLang();
	const cartas = useDeck((s) => s.deck?.cards);
	const [s, setS] = useState<DeckStats | null>(null);
	const [error, setError] = useState<unknown>(null);

	useEffect(() => {
		if (!cartas) return;
		let vigente = true;
		call("deck_stats", { cards: cartas })
			.then((r) => vigente && setS(r))
			.catch((e) => vigente && setError(e));
		return () => {
			vigente = false;
		};
	}, [cartas]);

	const euros = new Intl.NumberFormat(lang === "en" ? "en-GB" : "es-ES", {
		style: "currency",
		currency: "EUR",
	});

	return (
		<DialogContent className="flex max-h-[88vh] flex-col sm:max-w-3xl" cambiosSinGuardar={false}>
			<DialogHeader>
				<DialogTitle>{t("Estadísticas del deck")}</DialogTitle>
				<DialogDescription>
					{t("Datos y precios de Scryfall. Las cartas propias no cuentan.")}
				</DialogDescription>
			</DialogHeader>
			<DialogCuerpo>
				{error !== null ? (
					<p className="py-10 text-center text-xs text-destructive">{mensaje(error)}</p>
				) : !s ? (
					<div className="flex justify-center py-16">
						<Loader2 className="size-5 animate-spin text-muted-foreground" />
					</div>
				) : (
					<div className="flex flex-col gap-4">
						<div className="grid grid-cols-2 gap-2.5 sm:grid-cols-4">
							<Cifra titulo={t("Cartas")} valor={String(s.cards)} />
							<Cifra titulo={t("Tierras")} valor={String(s.lands)} />
							<Cifra titulo={t("Coste medio")} valor={s.averageCmc.toFixed(2)} />
							<Cifra
								titulo={t("Precio estimado")}
								valor={euros.format(s.totalEur)}
								nota={s.withoutPrice ? t("{n} sin precio", { n: s.withoutPrice }) : undefined}
							/>
						</div>
						{(s.tokens > 0 || s.unknown > 0) && (
							<p className="text-[11px] text-muted-foreground">
								{t("Fuera de la cuenta: {t} tokens y {u} cartas sin datos.", {
									t: s.tokens,
									u: s.unknown,
								})}
							</p>
						)}

						<Bloque titulo={t("Curva de maná (sin tierras)")}>
							<GraficoBarras
								alto="h-40"
								destacarUltimo={false}
								formatearValor={(v) => String(Math.round(v))}
								datos={s.curve.map((n, i) => ({
									clave: String(i),
									etiqueta: i === 7 ? "7+" : String(i),
									valor: n,
									detalle: t("{n} cartas de coste {c}", { n, c: i === 7 ? "7+" : i }),
								}))}
							/>
						</Bloque>

						<div className="grid gap-4 sm:grid-cols-2">
							<Bloque titulo={t("Colores")}>
								<div className="flex flex-wrap gap-1.5">
									{s.colors.map((c) => (
										<WithTooltip key={c.key} content={t(COLORES[c.key]?.nombre ?? c.key)}>
											<span
												className={cn(
													"flex min-w-12 items-center justify-between gap-2 rounded-md px-2 py-1 font-mono text-xs",
													COLORES[c.key]?.clase,
													c.count === 0 && "opacity-30",
												)}
											>
												<b>{c.key}</b>
												{c.count}
											</span>
										</WithTooltip>
									))}
								</div>
								<p className="mt-2 text-[11px] text-muted-foreground">
									{t("Identidad de color: {c}", {
										c: s.colorIdentity.length ? s.colorIdentity.join("") : t("incolora"),
									})}
								</p>
							</Bloque>
							<Bloque titulo={t("Rareza")}>
								<Lista
									filas={s.rarity.map((r) => ({
										k: r.key,
										n: r.count,
										txt: t(RAREZAS[r.key] ?? r.key),
									}))}
								/>
							</Bloque>
						</div>

						<Bloque titulo={t("Tipos")}>
							<Lista
								filas={s.types.map((r) => ({
									k: r.key,
									n: r.count,
									txt: t(TIPOS[r.key] ?? r.key),
								}))}
							/>
						</Bloque>

						<Bloque titulo={t("Legalidad")}>
							<div className="grid grid-cols-2 gap-1.5 sm:grid-cols-3">
								{s.legality.map((l) => (
									<WithTooltip
										key={l.format}
										content={
											l.offenders.length
												? t("No legales: {c}", { c: l.offenders.slice(0, 8).join(", ") })
												: null
										}
									>
										<span
											className={cn(
												"flex items-center gap-1.5 rounded-md border px-2 py-1 text-xs capitalize",
												l.legal
													? "border-green-600/40 text-green-700 dark:text-green-400"
													: "text-muted-foreground",
											)}
										>
											{l.legal ? <Check className="size-3.5" /> : <X className="size-3.5" />}
											{l.format}
										</span>
									</WithTooltip>
								))}
							</div>
						</Bloque>
					</div>
				)}
			</DialogCuerpo>
		</DialogContent>
	);
}

function Cifra({ titulo, valor, nota }: { titulo: string; valor: string; nota?: string }) {
	return (
		<div className="rounded-lg border bg-card p-2.5">
			<div className="text-[10px] font-semibold tracking-wider text-muted-foreground uppercase">
				{titulo}
			</div>
			<div className="text-lg font-semibold">{valor}</div>
			{nota && <div className="text-[10px] text-muted-foreground">{nota}</div>}
		</div>
	);
}

function Bloque({ titulo, children }: { titulo: string; children: React.ReactNode }) {
	return (
		<section className="flex flex-col gap-2">
			<h3 className="text-[11px] font-semibold tracking-wider text-muted-foreground uppercase">
				{titulo}
			</h3>
			{children}
		</section>
	);
}

function Lista({ filas }: { filas: { k: string; n: number; txt: string }[] }) {
	const max = Math.max(1, ...filas.map((f) => f.n));
	return (
		<div className="flex flex-col gap-1">
			{filas.map((f) => (
				<div key={f.k} className="grid grid-cols-[7rem_1fr_2rem] items-center gap-2 text-xs">
					<span className="truncate">{f.txt}</span>
					<div className="h-2 overflow-hidden rounded-full bg-muted">
						<div
							className="h-full rounded-full bg-primary"
							style={{ width: `${(f.n / max) * 100}%` }}
						/>
					</div>
					<span className="text-right font-mono text-[11px]">{f.n}</span>
				</div>
			))}
		</div>
	);
}
