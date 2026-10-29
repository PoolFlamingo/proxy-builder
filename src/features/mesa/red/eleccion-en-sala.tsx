import { useEffect, useRef } from "react";
import type { Formato } from "@/bindings/Formato";
import { Fila, Selector } from "@/components/campos";
import { Icono } from "@/components/icono";
import { useLang } from "@/components/language-provider";
import { Input } from "@/components/ui/input";
import { useBiblioteca } from "@/features/library/store";
import { MAX_NOMBRE, useNombre } from "../nombre";
import type { Preparacion } from "./red-dialog";

/** El valor del selector de listas para jugar con el deck entero (Radix no admite «»). */
const TODO_EL_DECK = "*";
/** Lo que se espera a que se pare de escribir para contar el nombre a los demás, en ms. */
const ESPERA_NOMBRE_MS = 300;

const SIN_DECKS: never[] = [];

/** Lo que se aplica cuando se cambia el deck o la lista desde la sala. */
export type EleccionAplicada = { deckId: string; lista: string | null; formato: Formato };

/**
 * **Lo que cada uno elige en la sala de espera**: cómo se llama y, si juega,
 * con qué deck y qué lista para jugar, igual que en una partida local. El
 * cambio llega a los demás al momento: el nombre al dejar de escribir y el
 * deck en cuanto se ha preparado (`aplicar`). Lo que ya traía la entrada
 * (nombre, deck y lista) se queda como está hasta que se toque algo aquí.
 *
 * El deck y la lista elegidos viven en la entrada de «Jugar» (`preparar`): el
 * selector de aquí es el mismo que el de allí, no una copia.
 */
export function EleccionEnSala({
	preparar,
	juega,
	alNombre,
	aplicar,
}: {
	preparar: Preparacion;
	/** Solo mira: no elige deck. */
	juega: boolean;
	/** Cambia el nombre con el que se ve a esta app en la sala (`null`: no se puede aquí). */
	alNombre: ((nombre: string) => void) | null;
	aplicar: (e: EleccionAplicada) => void;
}) {
	const { t } = useLang();
	const nombre = useNombre((s) => s.nombre);
	const poner = useNombre((s) => s.poner);
	const decks = useBiblioteca((s) => s.decks) ?? SIN_DECKS;

	// El nombre: a los demás les llega al dejar de escribir.
	const contado = useRef(nombre.trim());
	useEffect(() => {
		const limpio = nombre.trim();
		if (!alNombre || !limpio || limpio === contado.current) return;
		const id = window.setTimeout(() => {
			contado.current = limpio;
			alNombre(limpio);
		}, ESPERA_NOMBRE_MS);
		return () => window.clearTimeout(id);
	}, [nombre, alNombre]);

	// El deck y la lista: se aplican cuando la persona los toca aquí, con las
	// listas del deck ya leídas (si no, saldrían dos cambios seguidos).
	const tocado = useRef(false);
	const aplicado = useRef("");
	const { deckId, lista, sinLista, listo, formato } = preparar;
	useEffect(() => {
		if (!tocado.current || !juega || !deckId || !listo || sinLista) return;
		const clave = `${deckId}|${lista ?? ""}`;
		if (clave === aplicado.current) return;
		aplicado.current = clave;
		tocado.current = false;
		aplicar({ deckId, lista, formato });
	}, [deckId, lista, sinLista, listo, formato, juega, aplicar]);

	const l = preparar.listas;
	return (
		<div className="flex flex-col gap-1.5 rounded-lg border bg-card/40 p-2.5">
			{alNombre && (
				<Fila etiqueta={t("Tu nombre")}>
					<div className="relative">
						<Icono
							icon="lucide:user"
							className="pointer-events-none absolute top-1/2 left-2 size-3.5 -translate-y-1/2 text-muted-foreground"
						/>
						<Input
							value={nombre}
							onChange={(e) => poner(e.target.value)}
							placeholder={t("Jugador")}
							maxLength={MAX_NOMBRE}
							className="h-7 w-44 pl-7 text-xs"
						/>
					</div>
				</Fila>
			)}
			{juega && (
				<>
					<Fila etiqueta={t("Deck")}>
						<Selector<string>
							valor={preparar.deckId ?? ""}
							onChange={(v) => {
								tocado.current = true;
								preparar.elegirDeck(v);
							}}
							opciones={decks.map((d) => ({ valor: d.id, etiqueta: d.name }))}
							ancho="w-44"
						/>
					</Fila>
					{preparar.deckId && l && (
						<Fila etiqueta={t("Lista para jugar")}>
							{preparar.pideLista && l.listas.length === 0 ? (
								<span className="text-xs text-muted-foreground">{t("Ninguna todavía")}</span>
							) : (
								<Selector<string>
									valor={preparar.lista ?? TODO_EL_DECK}
									onChange={(v) => {
										tocado.current = true;
										preparar.elegirLista(v === TODO_EL_DECK ? null : v);
									}}
									opciones={[
										...(preparar.pideLista
											? []
											: [
													{
														valor: TODO_EL_DECK,
														etiqueta: t("Todo el deck ({n})", { n: l.cuentan }),
													},
												]),
										...l.listas.map((x) => ({ valor: x.id, etiqueta: x.nombre })),
									]}
									ancho="w-44"
								/>
							)}
						</Fila>
					)}
					{preparar.sinLista && l && (
						<p className="flex items-start gap-1.5 rounded-md bg-amber-500/10 p-1.5 text-[11px] text-amber-600 dark:text-amber-300">
							<Icono icon="lucide:list-checks" className="mt-0.5 size-3.5 shrink-0" />
							{t(
								"Este deck tiene {n} cartas y en Commander se juega con 100: haz una lista con el comandante y 99 más.",
								{ n: l.cuentan },
							)}
						</p>
					)}
				</>
			)}
		</div>
	);
}
