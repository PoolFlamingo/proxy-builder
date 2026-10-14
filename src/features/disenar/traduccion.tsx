import { Languages, Loader2 } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { toast } from "sonner";
import type { CardDesign } from "@/bindings/CardDesign";
import type { Sugerencia } from "@/bindings/Sugerencia";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import { call } from "@/lib/ipc";
import { cn } from "@/lib/utils";

/**
 * **Traducción sugerida** (experimental) para las cartas sin impresión
 * española, o con la línea de tipo sin traducir: frases idénticas de otras
 * cartas, con su texto oficial. Lo que no se encuentra sale en inglés y
 * marcado. Nunca se aplica sola.
 */
export function TraduccionSugerida({
	design,
	cara,
	cambiar,
}: {
	design: CardDesign;
	cara: number;
	cambiar: (f: (d: CardDesign) => CardDesign) => void;
}) {
	const { t, mensaje } = useLang();
	const [s, setS] = useState<Sugerencia | null>(null);
	const [buscando, setBuscando] = useState(false);
	const clave = `${design.card.scryfallId}:${cara}`;
	const claveActual = useRef(clave);
	// Al cambiar de carta o de cara, la sugerencia de antes ya no vale.
	useEffect(() => {
		claveActual.current = clave;
		setS(null);
	}, [clave]);
	const datos = design.card.faces[cara];
	if (!datos) return null;
	const faltaTipo = datos.typeLine.es === datos.typeLine.en && datos.typeLine.en !== "";
	if (!design.card.sinTextoEs && !faltaTipo) return null;

	const buscar = async () => {
		setBuscando(true);
		const pedida = clave;
		try {
			const r = await call("design_suggest_translation", { design, cara });
			// Una respuesta de otra carta o de otra cara no vale.
			if (claveActual.current === pedida) setS(r);
		} catch (e) {
			toast.error(mensaje(e));
		} finally {
			setBuscando(false);
		}
	};
	const usarTipo = (tipo: string) =>
		cambiar((d) => ({
			...d,
			card: {
				...d.card,
				faces: d.card.faces.map((f, i) =>
					i === cara ? { ...f, typeLine: { ...f.typeLine, es: tipo } } : f,
				),
			},
		}));
	const usarReglas = (reglas: string) =>
		cambiar((d) => ({
			...d,
			faces: d.faces.map((f, i) =>
				i === cara
					? { ...f, texts: { ...f.texts, es: { ...f.texts.es, rulesOverride: reglas } } }
					: f,
			),
		}));

	return (
		<div className="flex flex-col gap-2 rounded-lg border border-dashed p-2.5">
			<div className="flex items-center gap-2">
				<span className="flex-1 text-[11px] font-semibold text-muted-foreground">
					{t("Traducción sugerida (experimental)")}
				</span>
				<Button size="xs" variant="outline" onClick={buscar} disabled={buscando}>
					{buscando ? <Loader2 className="animate-spin" /> : <Languages />}
					{t("Buscar frases oficiales")}
				</Button>
			</div>
			{!s && (
				<p className="text-[11px] text-muted-foreground">
					{t(
						"Busca en Scryfall las mismas frases en otras cartas que sí están en español y propone su texto oficial. Tarda un poco: una búsqueda por frase.",
					)}
				</p>
			)}
			{s && (
				<>
					{s.tipo && s.tipo !== datos.typeLine.es && (
						<div className="flex items-center gap-2 text-xs">
							<span className="min-w-0 flex-1">
								<span className="text-muted-foreground">{t("Tipo")}: </span>
								{s.tipo}
							</span>
							<Button size="xs" variant="ghost" onClick={() => s.tipo && usarTipo(s.tipo)}>
								{t("Usar")}
							</Button>
						</div>
					)}
					{s.total > 0 && (
						<>
							<p className="text-[11px] text-muted-foreground">
								{t("{n} de {total} frases encontradas; lo marcado se queda en inglés.", {
									n: s.traducidas,
									total: s.total,
								})}
							</p>
							<ul className="flex flex-col gap-1">
								{s.frases.map((f, i) => (
									<li
										key={i}
										className={cn(
											"rounded px-1.5 py-0.5 text-[11px]",
											f.es
												? "bg-emerald-500/10"
												: "bg-amber-500/15 text-amber-800 dark:text-amber-300",
										)}
									>
										{f.es ?? f.en}
									</li>
								))}
							</ul>
							<Button
								size="xs"
								variant="outline"
								className="self-end"
								onClick={() => usarReglas(s.reglas)}
							>
								{t("Usar como reglas en español")}
							</Button>
						</>
					)}
				</>
			)}
		</div>
	);
}
