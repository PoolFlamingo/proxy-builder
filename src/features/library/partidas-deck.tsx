import { useCallback, useEffect, useRef, useState } from "react";
import { toast } from "sonner";
import type { FilaPartida } from "@/bindings/FilaPartida";
import type { HistorialDeck } from "@/bindings/HistorialDeck";
import { Confirmar } from "@/components/confirmar";
import { EstadoCarga } from "@/components/estado-carga";
import { Icono } from "@/components/icono";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { WithTooltip } from "@/components/ui/with-tooltip";
import { formatoDuracion } from "@/features/mesa/cuentas-fin";
import { COMO } from "@/features/mesa/dialogos/terminar";
import { call } from "@/lib/ipc";
import { cn } from "@/lib/utils";
import { useFecha } from "./piezas";

/** El porcentaje de victorias, redondeado (0 si no hay partidas). */
export function porcentajeVictorias(partidas: number, victorias: number): number {
	return partidas === 0 ? 0 : Math.round((victorias / partidas) * 100);
}

/**
 * **Las partidas de un deck** (pestaña «Partidas» de sus propiedades): cuántas
 * se han jugado, cuántas ganó y la lista, de la más reciente a la más
 * antigua. Las pide a Rust aparte de los detalles del deck: jugar no cambia
 * la fecha del deck, y la caché de detalles no se enteraría.
 */
export function PartidasDeDeck({
	deckId,
	onVerRepeticion,
}: {
	deckId: string;
	onVerRepeticion?: (id: string) => void;
}) {
	const { t, mensaje } = useLang();
	const fecha = useFecha();
	const [h, setH] = useState<HistorialDeck | null>(null);
	const [borrar, setBorrar] = useState<string | null>(null);

	const [error, setError] = useState<unknown>(null);
	const hayLista = useRef(false);
	hayLista.current = h !== null;
	const cargar = useCallback(() => {
		setError(null);
		call("partidas_de_deck", { deck: deckId })
			.then(setH)
			.catch((e) => {
				// Sin nada cargado, el error va en su sitio (con «Reintentar»); si ya había
				// lista, se queda y se avisa.
				setError(e);
				if (hayLista.current) toast.error(mensaje(e));
			});
	}, [deckId, mensaje]);
	useEffect(() => {
		setH(null);
		cargar();
	}, [cargar]);

	if (h === null) return <EstadoCarga error={error} onReintentar={cargar} className="h-40" />;
	if (h.partidas === 0) {
		return (
			<p className="rounded-lg border p-6 text-center text-xs text-muted-foreground">
				{t("Todavía no has terminado ninguna partida con este deck.")}
			</p>
		);
	}
	return (
		<div className="flex flex-col gap-2">
			<p className="text-xs tabular-nums">
				{t("Partidas: {partidas} · Victorias: {victorias} ({porcentaje} %)", {
					partidas: h.partidas,
					victorias: h.victorias,
					porcentaje: porcentajeVictorias(h.partidas, h.victorias),
				})}
			</p>
			<ScrollArea className="rounded-lg border" viewportClassName="max-h-72">
				<div className="flex flex-col divide-y">
					{h.filas.map((f) => (
						<Fila
							key={f.id}
							fila={f}
							deckId={deckId}
							fecha={fecha(f.terminada)}
							onBorrar={() => setBorrar(f.id)}
							onVerRepeticion={onVerRepeticion}
						/>
					))}
				</div>
			</ScrollArea>
			<Confirmar
				abierto={borrar !== null}
				onCerrar={() => setBorrar(null)}
				titulo={t("¿Borrar esta partida del historial?")}
				descripcion={t("Se borra también su repetición.")}
				accion={t("Borrar")}
				destructiva
				onConfirmar={async () => {
					if (borrar === null) return;
					try {
						await call("partida_borrar", { id: borrar });
						cargar();
					} catch (e) {
						toast.error(mensaje(e));
					}
				}}
			/>
		</div>
	);
}

function Fila({
	fila,
	deckId,
	fecha,
	onBorrar,
	onVerRepeticion,
}: {
	fila: FilaPartida;
	deckId: string;
	fecha: string;
	onBorrar: () => void;
	onVerRepeticion?: (id: string) => void;
}) {
	const { t } = useLang();
	// Quien se cuenta es el primer asiento con este deck: en una partida local
	// con el mismo deck en varios asientos, los demás son rivales.
	const mio = fila.jugadores.findIndex((j) => j.deckId === deckId);
	const rivales = fila.jugadores.filter((_, i) => i !== mio).map((j) => j.nombre);
	const como = COMO.find((c) => c.valor === fila.como)?.nombre;
	return (
		<div className="flex items-center gap-2 p-2 text-xs">
			<span
				className={cn(
					"flex w-20 shrink-0 items-center gap-1 font-medium",
					fila.gano ? "text-primary" : "text-muted-foreground",
				)}
			>
				{fila.gano && <Icono icon="lucide:trophy" className="size-3.5" />}
				{fila.gano ? t("Victoria") : t("Puesto {puesto}", { puesto: fila.puesto })}
			</span>
			<span className="flex min-w-0 flex-1 flex-col">
				<span className="truncate">
					{rivales.length === 0
						? t("En solitario")
						: t("Con {rivales}", { rivales: rivales.join(", ") })}
				</span>
				<span className="truncate text-[11px] text-muted-foreground">
					{[
						fecha,
						como ? t(como) : null,
						t("Rondas: {rondas} · Duración: {duracion}", {
							rondas: fila.rondas,
							duracion: formatoDuracion(fila.duracionMs),
						}),
					]
						.filter(Boolean)
						.join(" · ")}
				</span>
			</span>
			{fila.repetible && onVerRepeticion && (
				<Button size="xs" variant="outline" onClick={() => onVerRepeticion(fila.id)}>
					{t("Ver repetición")}
				</Button>
			)}
			<WithTooltip content={t("Borrar")}>
				<Button size="icon-sm" variant="ghost" onClick={onBorrar} aria-label={t("Borrar")}>
					<Icono icon="lucide:trash-2" className="size-3.5" />
				</Button>
			</WithTooltip>
		</div>
	);
}
