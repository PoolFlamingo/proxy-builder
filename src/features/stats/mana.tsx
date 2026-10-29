import type { DeckStats } from "@/bindings/DeckStats";
import { Icono } from "@/components/icono";
import { useLang } from "@/components/language-provider";
import { cn } from "@/lib/utils";
import { Bloque, Lista } from "./piezas";
import { COLORES } from "./textos";

/** Diferencia (en puntos) a partir de la que se avisa de pocas fuentes. */
const AVISO_FUENTES = 0.1;

/** Lo que piden los costes frente a lo que dan las tierras, color a color. */
export function Mana({ s }: { s: DeckStats }) {
	const { t } = useLang();
	const totalPips = s.colors.reduce((a, c) => a + c.pips, 0);
	const totalFuentes = s.colors.reduce((a, c) => a + c.landSources, 0);
	const filas = s.colors.filter((c) => c.pips + c.landSources + c.otherSources + c.cards > 0);
	const pocas = filas.filter(
		(c) =>
			totalPips > 0 &&
			totalFuentes > 0 &&
			c.pips / totalPips - c.landSources / totalFuentes > AVISO_FUENTES,
	);
	const nombresColores = [
		t("Incoloras"),
		t("Monocolor"),
		t("2 colores"),
		t("3 colores"),
		t("4 colores"),
		t("5 colores"),
	];

	return (
		<div className="flex flex-col gap-2.5">
			<Bloque titulo={t("Símbolos de maná frente a fuentes")}>
				<p className="text-[11px] text-muted-foreground">
					{t(
						"Qué parte de los símbolos de color de los costes es de cada color, y qué parte de las tierras lo produce. Si un color pide mucho más de lo que dan las tierras, costará lanzarlo a tiempo.",
					)}
				</p>
				{filas.length === 0 ? (
					<p className="text-[11px] text-muted-foreground">{t("Sin datos de maná.")}</p>
				) : (
					<div className="flex flex-col gap-2">
						{filas.map((c) => {
							const pp = totalPips ? c.pips / totalPips : 0;
							const pf = totalFuentes ? c.landSources / totalFuentes : 0;
							return (
								<div key={c.color} className="grid grid-cols-[2.5rem_1fr] items-center gap-2.5">
									<span
										className={cn(
											"flex h-8 items-center justify-center rounded-md font-mono text-sm font-bold",
											COLORES[c.color]?.chip,
										)}
									>
										{c.color}
									</span>
									<div className="flex flex-col gap-1 text-[11px]">
										<Barra
											etiqueta={t("Costes")}
											pct={pp}
											texto={t("{n} símbolos · {p} %", { n: c.pips, p: Math.round(pp * 100) })}
											clase={COLORES[c.color]?.barra}
										/>
										<Barra
											etiqueta={t("Tierras")}
											pct={pf}
											texto={t("{n} tierras · {p} %", {
												n: c.landSources,
												p: Math.round(pf * 100),
											})}
											clase={COLORES[c.color]?.barra}
											rayada
										/>
										{c.otherSources > 0 && (
											<span className="pl-16 text-muted-foreground">
												{t("+ {n} fuentes que no son tierra", { n: c.otherSources })}
											</span>
										)}
									</div>
								</div>
							);
						})}
					</div>
				)}
				{pocas.length > 0 && (
					<p className="flex items-center gap-1.5 text-xs text-amber-700 dark:text-amber-400">
						<Icono icon="lucide:alert-triangle" className="size-3.5" />
						{t("Pocas fuentes para lo que piden los costes: {c}", {
							c: pocas.map((c) => t(COLORES[c.color]?.nombre ?? c.color)).join(", "),
						})}
					</p>
				)}
			</Bloque>

			<div className="grid gap-2.5 sm:grid-cols-2">
				<Bloque titulo={t("Cartas por número de colores")}>
					<Lista
						filas={s.colorCount.map((n, i) => ({
							k: String(i),
							n,
							txt: nombresColores[i] ?? String(i),
						}))}
					/>
				</Bloque>
				<Bloque titulo={t("Cartas de cada color")}>
					<Lista
						ancho="5rem"
						filas={s.colors.map((c) => ({
							k: c.color,
							n: c.cards,
							txt: t(COLORES[c.color]?.nombre ?? c.color),
							clase: COLORES[c.color]?.barra,
						}))}
					/>
				</Bloque>
			</div>
		</div>
	);
}

function Barra({
	etiqueta,
	pct,
	texto,
	clase,
	rayada,
}: {
	etiqueta: string;
	pct: number;
	texto: string;
	clase?: string;
	rayada?: boolean;
}) {
	return (
		<div className="grid grid-cols-[3.5rem_1fr_8rem] items-center gap-2">
			<span className="text-muted-foreground">{etiqueta}</span>
			<div className="h-2.5 overflow-hidden rounded-full bg-muted">
				<div
					className={cn("h-full rounded-full", clase ?? "bg-primary", rayada && "opacity-60")}
					style={{ width: `${pct * 100}%` }}
				/>
			</div>
			<span className="text-right font-mono tabular-nums">{texto}</span>
		</div>
	);
}
