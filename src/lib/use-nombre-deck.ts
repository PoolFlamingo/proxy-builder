import { useCallback } from "react";
import { useLang } from "@/components/language-provider";
import { nombreVisible } from "./nombre-deck";

/** El nombre de un deck tal como se enseña (el de fábrica, en el idioma de ahora). */
export function useNombreDeck(): (nombre: string) => string {
	const { t } = useLang();
	const sinTitulo = t("Deck sin título");
	return useCallback((nombre: string) => nombreVisible(nombre, sinTitulo), [sinTitulo]);
}
