import { useEffect, useState } from "react";
import { useLang } from "@/components/language-provider";
import { abrirDeckInicial, iniciarAutoguardado, useDeck } from "@/stores/deck-store";
import { iniciarPlan } from "@/stores/plan-store";

/**
 * Al arrancar: abre el último deck (o crea uno), enciende el autoguardado y
 * los atajos de deshacer/rehacer. Devuelve el error si no se pudo abrir nada.
 */
export function useDeckInicial(): unknown {
	const { t } = useLang();
	const [error, setError] = useState<unknown>(null);

	useEffect(() => {
		const parar = iniciarAutoguardado();
		const pararPlan = iniciarPlan();
		abrirDeckInicial(t("Deck sin título")).catch(setError);
		return () => {
			parar();
			pararPlan();
		};
		// Solo al montar: el nombre del deck nuevo se decide una vez.
	}, []);

	useEffect(() => {
		const alTeclear = (e: KeyboardEvent) => {
			if (!(e.ctrlKey || e.metaKey) || e.key.toLowerCase() !== "z") return;
			// En un campo de texto manda el deshacer del propio campo.
			const el = e.target as HTMLElement | null;
			if (el?.closest("input, textarea, [contenteditable='true']")) return;
			e.preventDefault();
			if (e.shiftKey) useDeck.getState().rehacer();
			else useDeck.getState().deshacer();
		};
		window.addEventListener("keydown", alTeclear);
		return () => window.removeEventListener("keydown", alTeclear);
	}, []);

	return error;
}
