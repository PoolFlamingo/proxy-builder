import { useState } from "react";
import type { DeckStats } from "@/bindings/DeckStats";
import { Icono } from "@/components/icono";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import { WithTooltip } from "@/components/ui/with-tooltip";
import { Bloque, Cifra, Lista, seleccionarEnDeck } from "./piezas";
import { formatoMoneda, type Moneda, TIPOS } from "./textos";

/** Cuántas filas salen antes de «Ver todas». */
const PRIMERAS = 25;

/** Precio total, por tipo, por tramo y carta a carta. */
export function Precios({ s, moneda }: { s: DeckStats; moneda: Moneda }) {
	const { t, lang } = useLang();
	const [todas, setTodas] = useState(false);
	const dinero = formatoMoneda(lang, moneda);
	const pr = s.prices;
	const total = moneda === "eur" ? pr.totalEur : pr.totalUsd;
	const sin = moneda === "eur" ? pr.withoutEur : pr.withoutUsd;
	const unidad = (c: (typeof pr.cards)[number]) => (moneda === "eur" ? c.eur : c.usd);
	const cartas = [...pr.cards].sort(
		(a, b) => (unidad(b) ?? 0) * b.count - (unidad(a) ?? 0) * a.count,
	);
	const visibles = todas ? cartas : cartas.slice(0, PRIMERAS);
	const mas = cartas[0];
	const conPrecio = s.cards - sin;

	return (
		<div className="flex flex-col gap-2.5">
			<div className="grid grid-cols-2 gap-2.5 sm:grid-cols-4">
				<Cifra titulo={t("Total")} valor={dinero.format(total)} />
				<Cifra
					titulo={t("Media por carta")}
					valor={dinero.format(conPrecio > 0 ? total / conPrecio : 0)}
				/>
				<Cifra
					titulo={t("La más cara")}
					valor={mas && unidad(mas) != null ? dinero.format(unidad(mas) ?? 0) : "—"}
					nota={mas?.name}
				/>
				<Cifra
					titulo={t("Sin precio")}
					valor={String(sin)}
					nota={t("copias sin precio en {m}", { m: moneda.toUpperCase() })}
				/>
			</div>
			<p className="text-[11px] text-muted-foreground">
				{t(
					"Precios de Scryfall (Cardmarket en euros, TCGplayer en dólares) de la impresión elegida, sin foil. Se actualizan una vez al día.",
				)}
			</p>
			<div className="grid gap-2.5 sm:grid-cols-2">
				<Bloque titulo={t("Por tipo")}>
					<Lista
						formato={(n) => dinero.format(n)}
						ancho="6.5rem"
						filas={pr.byType
							.map((p) => ({
								k: p.key,
								n: moneda === "eur" ? p.eur : p.usd,
								txt: t(TIPOS[p.key]?.nombre ?? p.key),
								clase: TIPOS[p.key]?.clase,
							}))
							.sort((a, b) => b.n - a.n)}
					/>
				</Bloque>
				<Bloque titulo={t("Copias por precio (euros)")}>
					<Lista
						ancho="5rem"
						filas={pr.buckets.map((b) => ({ k: b.key, n: b.count, txt: `${b.key} €` }))}
					/>
				</Bloque>
			</div>
			<Bloque
				titulo={t("Cartas, de la más cara a la más barata")}
				extra={
					cartas.length > PRIMERAS && (
						<Button size="xs" variant="ghost" onClick={() => setTodas((v) => !v)}>
							{todas ? t("Ver menos") : t("Ver todas ({n})", { n: cartas.length })}
						</Button>
					)
				}
			>
				<table className="w-full text-xs">
					<thead>
						<tr className="text-[10px] text-muted-foreground uppercase">
							<th className="w-8 py-1 text-right font-semibold">#</th>
							<th className="py-1 pl-2.5 text-left font-semibold">{t("Carta")}</th>
							<th className="py-1 text-right font-semibold">{t("Unidad")}</th>
							<th className="py-1 text-right font-semibold">{t("Total")}</th>
							<th className="w-8" />
						</tr>
					</thead>
					<tbody>
						{visibles.map((c) => {
							const u = unidad(c);
							return (
								<tr key={c.name} className="group border-t border-border/50">
									<td className="py-1 text-right font-mono text-muted-foreground tabular-nums">
										{c.count}
									</td>
									<td className="max-w-0 truncate py-1 pl-2.5">{c.name}</td>
									<td className="py-1 text-right font-mono tabular-nums">
										{u != null ? dinero.format(u) : "—"}
									</td>
									<td className="py-1 text-right font-mono font-medium tabular-nums">
										{u != null ? dinero.format(u * c.count) : "—"}
									</td>
									<td className="py-0.5 text-right">
										<WithTooltip content={t("Seleccionar en el deck")}>
											<Button
												size="icon-xs"
												variant="ghost"
												className="opacity-0 group-hover:opacity-100 focus-visible:opacity-100"
												onClick={() =>
													seleccionarEnDeck(
														c.ids,
														t("{n} cartas seleccionadas: {f}", { n: c.ids.length, f: c.name }),
													)
												}
											>
												<Icono icon="lucide:mouse-pointer-click" />
											</Button>
										</WithTooltip>
									</td>
								</tr>
							);
						})}
					</tbody>
				</table>
			</Bloque>
		</div>
	);
}
