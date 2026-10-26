import { Hand } from "lucide-react";
import type { VistaCarta } from "@/bindings/VistaCarta";
import { useLang } from "@/components/language-provider";
import { cn } from "@/lib/utils";
import { COLORES_ETIQUETA } from "./colores";
import { ptDe } from "./pt";

/** Abreviatura de un contador para la insignia. */
function corto(tipo: string): string {
	if (tipo === "+1/+1" || tipo === "-1/-1") return tipo;
	return tipo.slice(0, 3).toUpperCase();
}

/**
 * Lo que se pinta encima de una carta del campo: contadores, fuerza y
 * resistencia cambiadas, etiquetas, palabras clave, cantidad del montón e
 * insignia «Robada».
 */
export function Insignias({ c, yo }: { c: VistaCarta; yo: number | null }) {
	const { t } = useLang();
	const pt = ptDe(c);
	const contadores = Object.entries(c.contadores);
	const robada = c.controlador !== c.dueno;
	return (
		<>
			{c.etiquetas.length > 0 && (
				<div className="pointer-events-none absolute inset-x-0.5 top-[14%] flex flex-col items-center gap-0.5">
					{c.etiquetas.map((e) => (
						<span
							key={e.texto}
							className="max-w-full truncate rounded px-1 text-[9px] font-semibold text-white shadow"
							style={{ background: COLORES_ETIQUETA[e.color] ?? "#78716c" }}
						>
							{e.texto}
						</span>
					))}
				</div>
			)}
			{contadores.length > 0 && (
				<div className="pointer-events-none absolute top-1/2 left-0.5 flex -translate-y-1/2 flex-col gap-0.5">
					{contadores.map(([tipo, n]) => (
						<span
							key={tipo}
							className="rounded-full bg-black/80 px-1.5 text-[10px] leading-4 font-bold text-white ring-1 ring-white/30"
						>
							{corto(tipo)}
							{n > 1 || tipo.includes("/") ? ` ×${n}` : ""}
						</span>
					))}
				</div>
			)}
			{c.palabrasClave.length > 0 && (
				<div className="pointer-events-none absolute right-0.5 bottom-[22%] flex max-w-[70%] flex-wrap justify-end gap-0.5">
					{c.palabrasClave.map((p) => (
						<span key={p} className="rounded bg-sky-700/90 px-1 text-[8px] leading-3 text-white">
							{p}
						</span>
					))}
				</div>
			)}
			{pt && (
				<span
					className={cn(
						"pointer-events-none absolute right-0.5 bottom-0.5 rounded px-1 text-[11px] font-bold tabular-nums",
						pt.cambiada ? "bg-amber-400 text-black" : "bg-black/75 text-white",
					)}
				>
					{pt.f}/{pt.r}
				</span>
			)}
			{c.cantidad > 1 && (
				<span className="pointer-events-none absolute -top-2 -right-2 rounded-full bg-primary px-1.5 text-[11px] font-bold text-primary-foreground shadow">
					×{c.cantidad}
				</span>
			)}
			{robada && (
				<span className="pointer-events-none absolute top-0.5 left-0.5 flex items-center gap-0.5 rounded bg-rose-600/90 px-1 text-[8px] font-semibold text-white">
					<Hand className="size-2.5" />
					{c.controlador === yo ? t("Robada") : t("Prestada")}
				</span>
			)}
			{c.habilidad && (
				<span className="pointer-events-none absolute inset-x-1 bottom-1 line-clamp-3 rounded bg-black/80 px-1 text-[9px] text-white">
					{c.habilidad}
				</span>
			)}
		</>
	);
}
