import { Check, X } from "lucide-react";
import type { DeckStats } from "@/bindings/DeckStats";
import { useLang } from "@/components/language-provider";
import { WithTooltip } from "@/components/ui/with-tooltip";
import { cn } from "@/lib/utils";
import { Bloque, ListaCartas } from "./piezas";
import { BRACKETS, CLASES_MAZO } from "./textos";

/** Legalidad por formato, reglas de construcción y bracket de Commander. */
export function Legalidad({ s }: { s: DeckStats }) {
	const { t } = useLang();
	const b = s.bracket;
	return (
		<div className="flex flex-col gap-2.5">
			<Bloque titulo={t("Legalidad por formato")}>
				<p className="text-[11px] text-muted-foreground">
					{t(
						"Según la lista de prohibidas de Scryfall de cada carta. No mira el tamaño del mazo ni las copias (eso va abajo).",
					)}
				</p>
				<div className="grid grid-cols-2 gap-1.5 sm:grid-cols-3">
					{s.legality.map((l) => (
						<WithTooltip
							key={l.format}
							content={
								l.offenders.length
									? t("No legales: {c}", { c: l.offenders.slice(0, 12).join(", ") })
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
								{!l.legal && l.offenders.length > 0 && (
									<span className="ml-auto font-mono text-[10px]">{l.offenders.length}</span>
								)}
							</span>
						</WithTooltip>
					))}
				</div>
			</Bloque>

			<Bloque titulo={t("Construcción como {c}", { c: t(CLASES_MAZO[s.kind] ?? s.kind) })}>
				{s.problems.length === 0 ? (
					<p className="flex items-center gap-1.5 text-xs text-green-700 dark:text-green-400">
						<Check className="size-3.5" />
						{t("Tamaño y copias correctos.")}
					</p>
				) : (
					s.problems.map((p) =>
						p.code === "copies" ? (
							<ListaCartas
								key={p.code}
								titulo={t("Más de {n} copias", { n: p.expected ?? 0 })}
								cartas={p.cards}
								abiertaAlPrincipio
								clase="bg-destructive"
							/>
						) : (
							<p key={p.code} className="flex items-center gap-1.5 text-xs">
								<X className="size-3.5 text-destructive" />
								{t("El mazo tiene {a} cartas y se esperan {e}.", {
									a: p.actual ?? 0,
									e: p.expected ?? 0,
								})}
							</p>
						),
					)
				)}
			</Bloque>

			{b && (
				<Bloque
					titulo={t("Bracket de Commander (estimado)")}
					extra={
						<span className="rounded-md bg-primary px-2 py-0.5 text-xs font-semibold text-primary-foreground">
							{b.level} · {t(BRACKETS[b.level]?.nombre ?? "")}
						</span>
					}
				>
					<p className="text-[11px] text-muted-foreground">
						{t(BRACKETS[b.level]?.ayuda ?? "")}{" "}
						{t(
							"Es el mínimo que permiten las cartas: no detecta combos ni la intención del mazo, que también cuentan.",
						)}
					</p>
					<div className="flex flex-col gap-1">
						<ListaCartas titulo={t("Game Changers")} cartas={b.gameChangers} abiertaAlPrincipio />
						<ListaCartas titulo={t("Tutores")} cartas={b.tutors} />
						<ListaCartas titulo={t("Turnos extra")} cartas={b.extraTurns} />
						<ListaCartas titulo={t("Destrucción masiva de tierras")} cartas={b.massLandDenial} />
					</div>
				</Bloque>
			)}
		</div>
	);
}
