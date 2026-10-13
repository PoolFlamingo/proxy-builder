import { useState } from "react";
import type { DeckStats } from "@/bindings/DeckStats";
import { useLang } from "@/components/language-provider";
import { GraficoBarras } from "@/components/ui/grafico-barras";
import {
	Select,
	SelectContent,
	SelectGroup,
	SelectItem,
	SelectLabel,
	SelectTrigger,
	SelectValue,
} from "@/components/ui/select";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { cn } from "@/lib/utils";
import { Bloque, porcentaje } from "./piezas";
import { FUNCIONES } from "./textos";

/** Turnos que se enseñan en la calculadora. */
const TURNOS = 10;

/**
 * Cartas vistas en el turno `turno` (0 = mano inicial): yendo primero no se
 * roba en el turno 1. Es el índice de `bySeen` (0 = 7 cartas).
 */
function indice(turno: number, primero: boolean) {
	if (turno === 0) return 0;
	return primero ? turno - 1 : turno;
}

/** Probabilidades de robo: tierras en mano, caídas de tierra y una calculadora. */
export function Probabilidades({ s }: { s: DeckStats }) {
	const { t } = useLang();
	const p = s.probability;
	const opciones = [
		...p.roles.map((r) => ({
			valor: `r:${r.key}`,
			texto: t("{f} ({n} cartas)", { f: t(FUNCIONES[r.key]?.nombre ?? r.key), n: r.copies }),
			prob: r,
		})),
		...p.cards.map((c) => ({
			valor: `c:${c.key}`,
			texto: c.copies > 1 ? `${c.key} (×${c.copies})` : c.key,
			prob: c,
		})),
	];
	const [elegida, setElegida] = useState(opciones[0]?.valor ?? "");
	const [primero, setPrimero] = useState(true);
	const [turno, setTurno] = useState(3);
	const actual = opciones.find((o) => o.valor === elegida) ?? opciones[0];
	const nombreTurno = (n: number) => (n === 0 ? t("Mano inicial") : t("Turno {n}", { n }));

	if (p.deckSize === 0) {
		return (
			<p className="py-10 text-center text-xs text-muted-foreground">{t("El deck está vacío.")}</p>
		);
	}

	return (
		<div className="flex flex-col gap-2.5">
			<Bloque titulo={t("Calculadora de robo")}>
				<p className="text-[11px] text-muted-foreground">
					{t(
						"Probabilidad de haber robado al menos una copia antes de un turno, con un mazo de {n} cartas y mano de 7 (sin mulligan).",
						{ n: p.deckSize },
					)}
				</p>
				<div className="flex flex-wrap items-center gap-2">
					<Select value={actual?.valor} onValueChange={setElegida}>
						<SelectTrigger size="sm" className="h-7 w-72 text-xs">
							<SelectValue />
						</SelectTrigger>
						<SelectContent>
							{p.roles.length > 0 && (
								<SelectGroup>
									<SelectLabel className="text-[10px]">{t("Funciones")}</SelectLabel>
									{opciones
										.filter((o) => o.valor.startsWith("r:"))
										.map((o) => (
											<SelectItem key={o.valor} value={o.valor} className="text-xs">
												{o.texto}
											</SelectItem>
										))}
								</SelectGroup>
							)}
							<SelectGroup>
								<SelectLabel className="text-[10px]">{t("Cartas")}</SelectLabel>
								{opciones
									.filter((o) => o.valor.startsWith("c:"))
									.map((o) => (
										<SelectItem key={o.valor} value={o.valor} className="text-xs">
											{o.texto}
										</SelectItem>
									))}
							</SelectGroup>
						</SelectContent>
					</Select>
					<Select value={String(turno)} onValueChange={(v) => setTurno(Number(v))}>
						<SelectTrigger size="sm" className="h-7 w-32 text-xs">
							<SelectValue />
						</SelectTrigger>
						<SelectContent>
							{Array.from({ length: TURNOS + 1 }, (_, i) => (
								<SelectItem key={i} value={String(i)} className="text-xs">
									{nombreTurno(i)}
								</SelectItem>
							))}
						</SelectContent>
					</Select>
					<ToggleGroup
						type="single"
						variant="outline"
						size="sm"
						value={primero ? "play" : "draw"}
						onValueChange={(v) => v && setPrimero(v === "play")}
					>
						<ToggleGroupItem value="play" className="h-7 px-2 text-[11px]">
							{t("Empiezo yo")}
						</ToggleGroupItem>
						<ToggleGroupItem value="draw" className="h-7 px-2 text-[11px]">
							{t("Empieza el rival")}
						</ToggleGroupItem>
					</ToggleGroup>
				</div>
				{actual && (
					<div className="grid items-center gap-2.5 sm:grid-cols-[10rem_1fr]">
						<div className="flex flex-col items-center justify-center rounded-lg bg-muted/50 p-2.5">
							<span className="text-3xl font-semibold tabular-nums">
								{porcentaje(actual.prob.bySeen[indice(turno, primero)] ?? 0)}
							</span>
							<span className="text-center text-[11px] text-muted-foreground">
								{t("{v} cartas vistas", { v: 7 + indice(turno, primero) })}
							</span>
						</div>
						<GraficoBarras
							alto="h-28"
							destacarUltimo={false}
							enteros={100}
							formatearValor={(v) => `${Math.round(v * 100)} %`}
							onPulsar={(clave) => setTurno(Number(clave))}
							datos={Array.from({ length: TURNOS + 1 }, (_, i) => ({
								clave: String(i),
								etiqueta: i === 0 ? t("Mano") : `T${i}`,
								valor: actual.prob.bySeen[indice(i, primero)] ?? 0,
								detalle: `${nombreTurno(i)}: ${porcentaje(actual.prob.bySeen[indice(i, primero)] ?? 0)}`,
							}))}
						/>
					</div>
				)}
			</Bloque>

			<div className="grid gap-2.5 sm:grid-cols-2">
				<Bloque
					titulo={t("Tierras en la mano inicial")}
					extra={
						<span className="text-[11px] text-muted-foreground">
							{t("{n} de media", { n: p.expectedLands.toFixed(2) })}
						</span>
					}
				>
					<GraficoBarras
						alto="h-32"
						destacarUltimo={false}
						enteros={100}
						formatearValor={(v) => `${Math.round(v * 100)} %`}
						datos={p.openingLands.map((v, k) => ({
							clave: String(k),
							etiqueta: String(k),
							valor: v,
							detalle: t("{k} tierras: {p}", { k, p: porcentaje(v) }),
						}))}
					/>
					<p className="text-[11px] text-muted-foreground">
						{t("Manos de 2 a 4 tierras: {p} · con 0-1 o 5+: {q}", {
							p: porcentaje(p.openingLands.slice(2, 5).reduce((a, b) => a + b, 0)),
							q: porcentaje(
								p.openingLands.slice(0, 2).reduce((a, b) => a + b, 0) +
									p.openingLands.slice(5).reduce((a, b) => a + b, 0),
							),
						})}
					</p>
				</Bloque>
				<Bloque titulo={t("Llegar a las tierras de cada turno")}>
					<p className="text-[11px] text-muted-foreground">
						{t("Probabilidad de tener al menos N tierras en el turno N.")}
					</p>
					<table className="w-full text-xs">
						<thead>
							<tr className="text-[10px] text-muted-foreground uppercase">
								<th className="py-1 text-left font-semibold">{t("Turno")}</th>
								<th className="py-1 text-right font-semibold">{t("Empiezo yo")}</th>
								<th className="py-1 text-right font-semibold">{t("Empieza el rival")}</th>
							</tr>
						</thead>
						<tbody>
							{p.landDrops.map((d) => (
								<tr key={d.turn} className="border-t border-border/50">
									<td className="py-1">{t("Turno {n}", { n: d.turn })}</td>
									<td className={cn("py-1 text-right font-mono tabular-nums", tono(d.play))}>
										{porcentaje(d.play)}
									</td>
									<td className={cn("py-1 text-right font-mono tabular-nums", tono(d.draw))}>
										{porcentaje(d.draw)}
									</td>
								</tr>
							))}
						</tbody>
					</table>
				</Bloque>
			</div>
		</div>
	);
}

function tono(p: number) {
	if (p >= 0.8) return "text-green-700 dark:text-green-400";
	if (p >= 0.5) return "";
	return "text-muted-foreground";
}
