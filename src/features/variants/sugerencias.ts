import { useEffect, useState } from "react";
import { call } from "@/lib/ipc";

/** Las sugerencias de nombres mientras se escribe (a los 200 ms de parar). */
export function useSugerencias(q: string) {
	const [sugerencias, setSugerencias] = useState<string[]>([]);
	useEffect(() => {
		const texto = q.trim();
		if (texto.length < 2) return setSugerencias([]);
		let vigente = true;
		const id = setTimeout(() => {
			call("card_autocomplete", { q: texto })
				.then((s) => vigente && setSugerencias(s.slice(0, 8)))
				.catch(() => vigente && setSugerencias([]));
		}, 200);
		return () => {
			vigente = false;
			clearTimeout(id);
		};
	}, [q]);
	return [sugerencias, setSugerencias] as const;
}
