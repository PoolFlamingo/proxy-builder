import type { DeckStats } from "@/bindings/DeckStats";
import { useLang } from "@/components/language-provider";
import { GraficoBarras } from "@/components/ui/grafico-barras";
import { localeDe } from "@/lib/idiomas";
import { Bloque, Cifra, Lista } from "./piezas";
import { RAREZAS, SUPERTIPOS, TIPOS } from "./textos";

/** Tipos, subtipos, habilidades, criaturas, rareza y sets. */
export function Tipos({ s }: { s: DeckStats }) {
	const { t, lang } = useLang();
	const fecha = (f: string | null) =>
		f
			? new Date(f).toLocaleDateString(localeDe(lang), {
					year: "numeric",
					month: "short",
				})
			: "—";
	return (
		<div className="flex flex-col gap-2.5">
			<div className="grid gap-2.5 sm:grid-cols-2">
				<Bloque titulo={t("Tipos")}>
					<Lista
						filas={s.types.map((r) => ({
							k: r.key,
							n: r.count,
							txt: t(TIPOS[r.key]?.nombre ?? r.key),
							clase: TIPOS[r.key]?.clase,
						}))}
					/>
					{s.supertypes.length > 0 && (
						<p className="text-[11px] text-muted-foreground">
							{s.supertypes.map((r) => `${t(SUPERTIPOS[r.key] ?? r.key)}: ${r.count}`).join(" · ")}
						</p>
					)}
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

			<Bloque titulo={t("Criaturas")}>
				<div className="grid grid-cols-3 gap-2.5">
					<Cifra titulo={t("Criaturas")} valor={String(s.creatures.count)} />
					<Cifra titulo={t("Fuerza media")} valor={s.creatures.averagePower?.toFixed(2) ?? "—"} />
					<Cifra
						titulo={t("Resistencia media")}
						valor={s.creatures.averageToughness?.toFixed(2) ?? "—"}
					/>
				</div>
				{s.creatures.count > 0 && (
					<GraficoBarras
						alto="h-28"
						enteros={1}
						destacarUltimo={false}
						formatearValor={(v) => String(Math.round(v))}
						datos={s.creatures.power.map((n, i) => ({
							clave: String(i),
							etiqueta: i === 7 ? "7+" : String(i),
							valor: n,
							detalle: t("{n} criaturas de fuerza {f}", { n, f: i === 7 ? "7+" : i }),
						}))}
					/>
				)}
			</Bloque>

			<div className="grid gap-2.5 sm:grid-cols-2">
				<Bloque titulo={t("Subtipos")}>
					<Lista
						ancho="8rem"
						filas={s.subtypes.map((r) => ({ k: r.key, n: r.count, txt: r.key }))}
						vacio={t("Ninguno.")}
					/>
				</Bloque>
				<Bloque titulo={t("Habilidades")}>
					<Lista
						ancho="8rem"
						filas={s.keywords.map((r) => ({ k: r.key, n: r.count, txt: r.key }))}
						vacio={t("Ninguna.")}
					/>
				</Bloque>
			</div>

			<Bloque
				titulo={t("Sets")}
				extra={
					<span className="text-[11px] text-muted-foreground">
						{t("{n} sets · de {a} a {b}", {
							n: s.sets.length,
							a: fecha(s.oldest),
							b: fecha(s.newest),
						})}
					</span>
				}
			>
				<Lista
					ancho="14rem"
					filas={s.sets.map((r) => ({
						k: r.code,
						n: r.count,
						txt: `${r.code.toUpperCase()} · ${r.name}`,
					}))}
				/>
			</Bloque>
		</div>
	);
}
