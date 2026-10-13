import { AlertTriangle } from "lucide-react";
import { useState } from "react";
import type { DeckStats } from "@/bindings/DeckStats";
import { useLang } from "@/components/language-provider";
import { GraficoApilado, LeyendaApilado } from "@/components/ui/grafico-apilado";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { WithTooltip } from "@/components/ui/with-tooltip";
import { cn } from "@/lib/utils";
import { Bloque, Cifra, ListaCartas } from "./piezas";
import { BRACKETS, COLORES, formatoMoneda, type Moneda, TIPOS } from "./textos";

/** Lo que salta a la vista: cifras, la curva, los colores y los avisos. */
export function Resumen({ s, moneda }: { s: DeckStats; moneda: Moneda }) {
	const { t, lang } = useLang();
	const [vista, setVista] = useState<"tipo" | "permanentes">("tipo");
	const dinero = formatoMoneda(lang, moneda);
	const total = moneda === "eur" ? s.prices.totalEur : s.prices.totalUsd;
	const sinPrecio = moneda === "eur" ? s.prices.withoutEur : s.prices.withoutUsd;
	const tiposCurva = s.curve[0]?.byType.map((b) => b.key) ?? [];
	const presentes = tiposCurva.filter((k) =>
		s.curve.some((c) => (c.byType.find((b) => b.key === k)?.count ?? 0) > 0),
	);
	const pctTierras = s.deckSize ? Math.round((s.lands / s.deckSize) * 100) : 0;

	return (
		<div className="flex flex-col gap-2.5">
			<div className="grid grid-cols-2 gap-2.5 sm:grid-cols-3 lg:grid-cols-6">
				<Cifra
					titulo={t("Cartas")}
					valor={String(s.deckSize)}
					nota={
						s.tokens
							? t("{n} sin tierras · {t} tokens", { n: s.nonlands, t: s.tokens })
							: t("{n} sin tierras", { n: s.nonlands })
					}
				/>
				<Cifra
					titulo={t("Tierras")}
					valor={String(s.lands)}
					nota={t("{p} % · recomendadas ~{r}", {
						p: pctTierras,
						r: Math.round(s.recommendedLands),
					})}
					ayuda={t(
						"Recomendación de la fórmula de Frank Karsten para esta curva: {r} tierras (descuenta {b} cartas baratas de rampa o robo).",
						{ r: s.recommendedLands.toFixed(1), b: s.cheapRampDraw },
					)}
				/>
				<Cifra
					titulo={t("Coste medio")}
					valor={s.averageCmc.toFixed(2)}
					nota={t("con tierras {c} · mediana {m}", {
						c: s.averageCmcWithLands.toFixed(2),
						m: s.medianCmc,
					})}
				/>
				<Cifra
					titulo={t("Valor de maná")}
					valor={String(Math.round(s.totalManaValue))}
					nota={t("suma de costes sin tierras")}
				/>
				<Cifra
					titulo={t("Precio estimado")}
					valor={dinero.format(total)}
					nota={sinPrecio ? t("{n} sin precio", { n: sinPrecio }) : t("todas con precio")}
				/>
				{s.bracket ? (
					<Cifra
						titulo={t("Bracket (estimado)")}
						valor={String(s.bracket.level)}
						nota={`${t(BRACKETS[s.bracket.level]?.nombre ?? "")} · ${t("{n} Game Changers", {
							n: s.bracket.gameChangers.length,
						})}`}
						ayuda={t(BRACKETS[s.bracket.level]?.ayuda ?? "")}
					/>
				) : (
					<Cifra
						titulo={t("Probabilidad de 2-4 tierras")}
						valor={`${Math.round(
							(s.probability.openingLands.slice(2, 5).reduce((a, b) => a + b, 0) || 0) * 100,
						)} %`}
						nota={t("en la mano inicial")}
					/>
				)}
			</div>

			{(s.unknown > 0 || s.problems.length > 0) && (
				<div className="flex flex-col gap-1.5 rounded-lg border border-amber-500/40 bg-amber-500/5 p-2.5">
					{s.unknown > 0 && (
						<ListaCartas
							titulo={t("Sin datos de Scryfall (propias sin vincular): no cuentan")}
							cartas={s.unknownCards}
							clase="bg-amber-500"
						/>
					)}
					{s.problems.map((p) =>
						p.code === "copies" ? (
							<ListaCartas
								key={p.code}
								titulo={t("Más de {n} copias", { n: p.expected ?? 0 })}
								cartas={p.cards}
								clase="bg-destructive"
							/>
						) : (
							<p key={p.code} className="flex items-center gap-1.5 text-xs">
								<AlertTriangle className="size-3.5 text-amber-600" />
								{t("El mazo tiene {a} cartas y se esperan {e}.", {
									a: p.actual ?? 0,
									e: p.expected ?? 0,
								})}
							</p>
						),
					)}
				</div>
			)}

			<Bloque
				titulo={t("Curva de maná (sin tierras)")}
				extra={
					<ToggleGroup
						type="single"
						variant="outline"
						size="sm"
						value={vista}
						onValueChange={(v) => v && setVista(v as typeof vista)}
					>
						<ToggleGroupItem value="tipo" className="h-6 px-2 text-[11px]">
							{t("Por tipo")}
						</ToggleGroupItem>
						<ToggleGroupItem value="permanentes" className="h-6 px-2 text-[11px]">
							{t("Permanentes y hechizos")}
						</ToggleGroupItem>
					</ToggleGroup>
				}
			>
				<GraficoApilado
					alto="h-48"
					datos={s.curve.map((c) => {
						const etiqueta = c.cmc === 7 ? "7+" : String(c.cmc);
						const partes =
							vista === "tipo"
								? c.byType.map((b) => ({
										clave: b.key,
										valor: b.count,
										clase: TIPOS[b.key]?.clase ?? "bg-primary",
									}))
								: [
										{ clave: "p", valor: c.permanents, clase: "bg-primary" },
										{ clave: "h", valor: c.spells, clase: "bg-sky-400" },
									];
						return {
							clave: String(c.cmc),
							etiqueta,
							partes,
							detalle: (
								<div className="flex flex-col gap-0.5">
									<b>{t("{n} cartas de coste {c}", { n: c.total, c: etiqueta })}</b>
									{partes
										.filter((p) => p.valor > 0)
										.map((p) => (
											<span key={p.clave}>
												{p.valor} ·{" "}
												{vista === "tipo"
													? t(TIPOS[p.clave]?.nombre ?? p.clave)
													: p.clave === "p"
														? t("permanentes")
														: t("instantáneos y conjuros")}
											</span>
										))}
								</div>
							),
						};
					})}
				/>
				<LeyendaApilado
					elementos={
						vista === "tipo"
							? presentes.map((k) => ({
									clave: k,
									etiqueta: t(TIPOS[k]?.nombre ?? k),
									clase: TIPOS[k]?.clase ?? "bg-primary",
								}))
							: [
									{ clave: "p", etiqueta: t("Permanentes"), clase: "bg-primary" },
									{ clave: "h", etiqueta: t("Instantáneos y conjuros"), clase: "bg-sky-400" },
								]
					}
				/>
			</Bloque>

			<div className="grid gap-2.5 sm:grid-cols-2">
				<Bloque titulo={t("Colores")}>
					<div className="flex flex-wrap gap-1.5">
						{s.colors.map((c) => (
							<WithTooltip
								key={c.color}
								content={t("{c}: {n} cartas, {p} símbolos de maná", {
									c: t(COLORES[c.color]?.nombre ?? c.color),
									n: c.cards,
									p: c.pips,
								})}
							>
								<span
									className={cn(
										"flex min-w-12 items-center justify-between gap-2 rounded-md px-2 py-1 font-mono text-xs",
										COLORES[c.color]?.chip,
										c.cards === 0 && c.pips === 0 && "opacity-30",
									)}
								>
									<b>{c.color}</b>
									{c.cards}
								</span>
							</WithTooltip>
						))}
					</div>
					<p className="text-[11px] text-muted-foreground">
						{t("Identidad de color: {c}", {
							c: s.colorIdentity.length ? s.colorIdentity.join("") : t("incolora"),
						})}
						{s.xSpells > 0 && ` · ${t("{n} con X en el coste", { n: s.xSpells })}`}
					</p>
				</Bloque>
				<Bloque titulo={t("Tokens que usa el deck")}>
					{s.tokenCards.length ? (
						<ul className="flex flex-col gap-0.5 text-xs">
							{s.tokenCards.map((c) => (
								<li key={c.name} className="flex gap-1.5">
									<span className="w-5 text-right font-mono text-[11px] text-muted-foreground tabular-nums">
										{c.count}
									</span>
									<span className="truncate">{c.name}</span>
								</li>
							))}
						</ul>
					) : (
						<p className="text-[11px] text-muted-foreground">{t("Ninguno en el deck.")}</p>
					)}
				</Bloque>
			</div>
		</div>
	);
}
