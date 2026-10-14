import { useEffect, useState } from "react";
import type { CardDesign } from "@/bindings/CardDesign";
import type { DesignLang } from "@/bindings/DesignLang";
import type { Resumen } from "@/bindings/Resumen";
import { call } from "@/lib/ipc";

/**
 * Cómo se maqueta la cara elegida (layout y texto partido en habilidades,
 * capítulos, niveles…). Lo calcula Rust; aquí solo se pide con algo de espera.
 */
export function useResumen(
	design: CardDesign | null,
	cara: number,
	lang: DesignLang,
): Resumen | null {
	const [resumen, setResumen] = useState<Resumen | null>(null);
	useEffect(() => {
		if (!design) {
			setResumen(null);
			return;
		}
		const id = setTimeout(() => {
			call("design_structure", { design, cara, lang })
				.then(setResumen)
				.catch(() => setResumen(null));
		}, 200);
		return () => clearTimeout(id);
	}, [design, cara, lang]);
	return resumen;
}
