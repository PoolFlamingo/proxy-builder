import { useEffect, useMemo, useRef, useState } from "react";
import type { LineError } from "@/bindings/LineError";
import type { ParsedDecklist } from "@/bindings/ParsedDecklist";
import { paraTraducir, useLang } from "@/components/language-provider";
import { ScrollArea } from "@/components/ui/scroll-area";
import { WithTooltip } from "@/components/ui/with-tooltip";
import { call } from "@/lib/ipc";
import { cn } from "@/lib/utils";

const MOTIVOS: Record<LineError, string> = {
	zeroQuantity: paraTraducir("La cantidad es 0"),
	badQuantity: paraTraducir("La cantidad no se entiende"),
	tooMany: paraTraducir("Demasiadas copias (máximo 999)"),
	emptyName: paraTraducir("Falta el nombre de la carta"),
};

/** Alto de cada línea, en px (el mismo para el texto y la columna de números). */
const LINEA = 20;

/**
 * El cuadro de la lista, con números de línea. Rust la lee 250 ms después de
 * dejar de escribir y las líneas que no entiende salen en rojo, con el motivo
 * en el tooltip del número.
 *
 * El cuadro crece con el texto (sin scroll propio) dentro del `ScrollArea`,
 * así números y texto se desplazan juntos y el scroll es el de la casa.
 */
export function EditorLista({
	valor,
	onChange,
	onLeida,
	deshabilitado,
}: {
	valor: string;
	onChange: (v: string) => void;
	onLeida?: (l: ParsedDecklist | null) => void;
	deshabilitado?: boolean;
}) {
	const { t } = useLang();
	const [leida, setLeida] = useState<ParsedDecklist | null>(null);
	const avisar = useRef(onLeida);
	avisar.current = onLeida;

	useEffect(() => {
		let vigente = true;
		const id = setTimeout(() => {
			call("parse_decklist", { text: valor })
				.then((l) => {
					if (!vigente) return;
					setLeida(l);
					avisar.current?.(l);
				})
				.catch(() => {
					/* el texto se vuelve a leer en la siguiente tecla */
				});
		}, 250);
		return () => {
			vigente = false;
			clearTimeout(id);
		};
	}, [valor]);

	const lineas = useMemo(() => Math.max(1, valor.split("\n").length), [valor]);
	const errores = useMemo(() => {
		const m = new Map<number, LineError>();
		for (const l of leida?.lines ?? [])
			if (l.kind === "invalid" && l.error) m.set(l.lineNo, l.error);
		return m;
	}, [leida]);
	const alto = Math.max(lineas + 1, 12) * LINEA + 16;

	return (
		<ScrollArea className="min-h-0 flex-1 rounded-lg border bg-background/60">
			<div className="flex" style={{ minHeight: alto }}>
				<div
					aria-hidden
					className="w-9 shrink-0 select-none border-r py-2 pr-1.5 text-right font-mono text-[10px] text-muted-foreground/70"
				>
					{Array.from({ length: lineas }, (_, i) => {
						const n = i + 1;
						const e = errores.get(n);
						const numero = (
							<div
								key={n}
								style={{ height: LINEA, lineHeight: `${LINEA}px` }}
								className={cn(e && "font-semibold text-destructive")}
							>
								{n}
							</div>
						);
						return e ? (
							<WithTooltip key={n} content={t(MOTIVOS[e])} side="right">
								{numero}
							</WithTooltip>
						) : (
							numero
						);
					})}
				</div>
				<div className="relative min-w-0 flex-1">
					{/* Las líneas malas, subrayadas por detrás del texto. */}
					{[...errores.keys()].map((n) => (
						<div
							key={n}
							aria-hidden
							className="pointer-events-none absolute inset-x-0 bg-destructive/10"
							style={{ top: 8 + (n - 1) * LINEA, height: LINEA }}
						/>
					))}
					<textarea
						value={valor}
						onChange={(e) => onChange(e.target.value)}
						disabled={deshabilitado}
						spellCheck={false}
						wrap="off"
						aria-label={t("Lista de cartas")}
						placeholder={t("4 Lightning Bolt\n1 Sol Ring (CMM) 410\n…")}
						className="relative block w-full resize-none overflow-hidden bg-transparent px-2.5 py-2 font-mono text-xs whitespace-pre outline-none placeholder:text-muted-foreground/60 disabled:opacity-60"
						style={{ height: alto, lineHeight: `${LINEA}px` }}
					/>
				</div>
			</div>
		</ScrollArea>
	);
}
