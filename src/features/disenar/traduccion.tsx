import { useEffect, useRef, useState } from "react";
import { toast } from "sonner";
import type { CardDesign } from "@/bindings/CardDesign";
import type { Sugerencia } from "@/bindings/Sugerencia";
import { Icono } from "@/components/icono";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import { idioma } from "@/lib/idiomas";
import { call } from "@/lib/ipc";
import { conIdioma, porIdioma, sinTextoEn, textosDe } from "@/lib/por-idioma";
import { cn } from "@/lib/utils";
import { useDeck } from "@/stores/deck-store";

/**
 * **Traducción sugerida** (experimental) para las cartas sin impresión en el
 * idioma de diseño, o con la línea de tipo sin traducir: frases idénticas de
 * otras cartas, con su texto oficial. Lo que no se encuentra sale en inglés
 * y marcado. Nunca se aplica sola. Solo en los idiomas con cartas impresas
 * (en rumano, polaco, neerlandés y ucraniano no hay dónde buscar).
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
	const lang = useDeck((st) => st.deck?.settings.designLang ?? "es");
	const [s, setS] = useState<Sugerencia | null>(null);
	const [buscando, setBuscando] = useState(false);
	const clave = `${design.card.scryfallId}:${cara}:${lang}`;
	const claveActual = useRef(clave);
	// Al cambiar de carta o de cara, la sugerencia de antes ya no vale.
	useEffect(() => {
		claveActual.current = clave;
		setS(null);
	}, [clave]);
	const datos = design.card.faces[cara];
	if (!datos) return null;
	if (lang === "en" || !idioma(lang).impresas) return null;
	const tipo = porIdioma(datos.typeLine, lang);
	const faltaTipo = tipo === datos.typeLine.en && datos.typeLine.en !== "";
	if (!sinTextoEn(design.card, lang) && !faltaTipo) return null;
	const nombre = idioma(lang).nativo;

	const buscar = async () => {
		setBuscando(true);
		const pedida = clave;
		try {
			const r = await call("design_suggest_translation", { design, cara, lang });
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
					i === cara ? { ...f, typeLine: conIdioma(f.typeLine, lang, tipo) } : f,
				),
			},
		}));
	const usarReglas = (reglas: string) =>
		cambiar((d) => ({
			...d,
			faces: d.faces.map((f, i) =>
				i === cara
					? {
							...f,
							texts: conIdioma(f.texts, lang, {
								...textosDe(f.texts, lang),
								rulesOverride: reglas,
							}),
						}
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
					{buscando ? (
						<Icono icon="lucide:loader-2" className="animate-spin" />
					) : (
						<Icono icon="lucide:languages" />
					)}
					{t("Buscar frases oficiales")}
				</Button>
			</div>
			{!s && (
				<p className="text-[11px] text-muted-foreground">
					{t(
						"Busca en Scryfall las mismas frases en otras cartas que sí están en {idioma} y propone su texto oficial. Tarda un poco: una búsqueda por frase.",
						{ idioma: nombre },
					)}
				</p>
			)}
			{s && (
				<>
					{s.tipo && s.tipo !== tipo && (
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
											f.traducida
												? "bg-emerald-500/10"
												: "bg-amber-500/15 text-amber-800 dark:text-amber-300",
										)}
									>
										{f.traducida ?? f.en}
									</li>
								))}
							</ul>
							<Button
								size="xs"
								variant="outline"
								className="self-end"
								onClick={() => usarReglas(s.reglas)}
							>
								{t("Usar como reglas en {idioma}", { idioma: nombre })}
							</Button>
						</>
					)}
				</>
			)}
		</div>
	);
}
