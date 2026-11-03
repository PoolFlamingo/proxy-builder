import type { EstadoResultado } from "@/bindings/EstadoResultado";
import type { ResultadoPasar } from "@/bindings/ResultadoPasar";
import { Icono } from "@/components/icono";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import { Progress } from "@/components/ui/progress";
import { cn } from "@/lib/utils";
import { textoDeError } from "./cuentas";
import type { AvanceImagenes } from "./imagenes";

const ICONO: Record<EstadoResultado, { icono: string; clase: string }> = {
	hecho: { icono: "lucide:check", clase: "text-emerald-500" },
	rechazado: { icono: "lucide:minus", clase: "text-muted-foreground" },
	fallo: { icono: "lucide:x", clase: "text-destructive" },
};

/** Qué pasó con cada deck. `onAbrir` (solo al recibir): el botón «Abrir». */
export function Resultados({
	resultados,
	onAbrir,
}: {
	resultados: ResultadoPasar[];
	onAbrir?: (r: ResultadoPasar) => void;
}) {
	const { t, mensaje } = useLang();
	if (resultados.length === 0) return null;
	return (
		<ul className="flex flex-col gap-1 p-1">
			{resultados.map((r, i) => {
				const ic = ICONO[r.estado];
				const detalle =
					r.estado === "rechazado"
						? t("No recibido")
						: r.estado === "fallo"
							? textoDeError(r.error, t, mensaje)
							: r.reemplazado
								? t("Reemplazado")
								: t("Guardado");
				return (
					<li key={i} className="flex items-center gap-2 rounded-lg border px-2 py-1.5">
						<Icono icon={ic.icono} className={cn("size-4 shrink-0", ic.clase)} />
						<div className="min-w-0 flex-1">
							<div className="truncate font-medium">{r.nombre}</div>
							<div className="text-muted-foreground">{detalle}</div>
						</div>
						{onAbrir && r.estado === "hecho" && r.deckId && (
							<Button size="xs" variant="outline" onClick={() => onAbrir(r)}>
								{t("Abrir")}
							</Button>
						)}
					</li>
				);
			})}
		</ul>
	);
}

/** «Bajando las imágenes…» tras recibir «solo cartas originales». */
export function ProgresoImagenes({
	avance,
	fallidas,
}: {
	avance: AvanceImagenes | null;
	fallidas: number;
}) {
	const { t } = useLang();
	if (!avance && fallidas === 0) return null;
	return (
		<div className="flex flex-col gap-1.5 rounded-lg border p-2.5">
			{avance ? (
				<>
					<div className="flex items-center justify-between">
						<span>{t("Bajando las imágenes de las cartas…")}</span>
						<span className="tabular-nums text-muted-foreground">
							{avance.hechas}/{avance.total}
						</span>
					</div>
					<Progress value={avance.total > 0 ? (avance.hechas / avance.total) * 100 : 0} />
				</>
			) : (
				<span>{t("Imágenes bajadas.")}</span>
			)}
			{fallidas > 0 && (
				<span className="text-amber-500">
					{t("No se han podido bajar {n} imágenes: se quedan en blanco hasta que las busques.", {
						n: fallidas,
					})}
				</span>
			)}
		</div>
	);
}
