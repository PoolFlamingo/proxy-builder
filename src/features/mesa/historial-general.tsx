import { Search, Trash2, Trophy } from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { toast } from "sonner";
import type { FilaPartida } from "@/bindings/FilaPartida";
import type { Formato } from "@/bindings/Formato";
import type { JugadorHistorial } from "@/bindings/JugadorHistorial";
import { Selector } from "@/components/campos";
import { Confirmar } from "@/components/confirmar";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import { WithTooltip } from "@/components/ui/with-tooltip";
import { porcentajeVictorias } from "@/features/library/partidas-deck";
import { useFecha } from "@/features/library/piezas";
import { call } from "@/lib/ipc";
import { cn } from "@/lib/utils";
import { formatoDuracion } from "./cuentas-fin";
import {
	FILTRO_VACIO,
	type FiltroHistorial,
	type FiltroResultado,
	filtrarPartidas,
	hayFiltro,
	MAX_ASIENTOS_FILA,
} from "./cuentas-historial";
import { COMO } from "./dialogos/terminar";
import { verRepeticionDelHistorial } from "./repeticion";

/** Cuántas partidas se piden cada vez. */
const PAGINA = 100;

/**
 * **El historial general** (J1): todas las partidas terminadas, de todos los
 * decks, con el deck y la lista para jugar que llevó cada asiento (las N
 * cartas elegidas cuando el deck tiene más de las que caben). Se filtra por
 * texto, resultado y formato; desde aquí se ve la repetición o se borra la
 * partida. Rust pagina; los filtros se aplican a lo ya cargado.
 */
export function HistorialGeneralDialog({
	abierto,
	onCerrar,
}: {
	abierto: boolean;
	onCerrar: () => void;
}) {
	const { t, mensaje } = useLang();
	const fecha = useFecha();
	const [filas, setFilas] = useState<FilaPartida[] | null>(null);
	const [total, setTotal] = useState(0);
	const [victorias, setVictorias] = useState(0);
	const [cargando, setCargando] = useState(false);
	const [filtro, setFiltro] = useState<FiltroHistorial>(FILTRO_VACIO);
	const [borrar, setBorrar] = useState<string | null>(null);

	const cargar = useCallback(
		async (desde: number) => {
			setCargando(true);
			try {
				const h = await call("partidas_todas", { limite: PAGINA, desde });
				setFilas((viejas) => (desde === 0 ? h.filas : [...(viejas ?? []), ...h.filas]));
				setTotal(h.total);
				setVictorias(h.victorias);
			} catch (e) {
				toast.error(mensaje(e));
			} finally {
				setCargando(false);
			}
		},
		[mensaje],
	);
	useEffect(() => {
		if (!abierto) return;
		setFilas(null);
		setFiltro(FILTRO_VACIO);
		void cargar(0);
	}, [abierto, cargar]);

	const visibles = useMemo(() => filtrarPartidas(filas ?? [], filtro), [filas, filtro]);
	const quedan = filas !== null && filas.length < total;

	return (
		<Dialog open={abierto} onOpenChange={(o) => !o && onCerrar()}>
			<DialogContent className="max-w-3xl" data-tutorial="mesa-historial">
				<DialogHeader>
					<DialogTitle>{t("Historial de partidas")}</DialogTitle>
					<DialogDescription>
						{t("Todas tus partidas terminadas, con el deck y la lista para jugar de cada asiento.")}
					</DialogDescription>
				</DialogHeader>
				<div className="flex flex-wrap items-center gap-2">
					<div className="relative min-w-48 flex-1">
						<Search className="pointer-events-none absolute top-1/2 left-2 size-3.5 -translate-y-1/2 text-muted-foreground" />
						<Input
							value={filtro.texto}
							onChange={(e) => setFiltro({ ...filtro, texto: e.target.value })}
							placeholder={t("Buscar por deck, lista, jugador o nota…")}
							aria-label={t("Buscar por deck, lista, jugador o nota…")}
							className="h-7 pl-7 text-xs"
						/>
					</div>
					<Selector<FiltroResultado>
						valor={filtro.resultado}
						onChange={(resultado) => setFiltro({ ...filtro, resultado })}
						ancho="w-48"
						ariaLabel={t("Resultado")}
						opciones={[
							{ valor: "todas", etiqueta: t("Todos los resultados") },
							{ valor: "ganadas", etiqueta: t("Victorias") },
							{ valor: "perdidas", etiqueta: t("Derrotas") },
						]}
					/>
					<Selector<Formato | "todos">
						valor={filtro.formato}
						onChange={(formato) => setFiltro({ ...filtro, formato })}
						ancho="w-44"
						ariaLabel={t("Formato")}
						opciones={[
							{ valor: "todos", etiqueta: t("Todos los formatos") },
							{ valor: "commander", etiqueta: "Commander" },
							{ valor: "constructed", etiqueta: t("60 cartas") },
							{ valor: "limited", etiqueta: t("Limitado") },
							{ valor: "libre", etiqueta: t("Libre") },
						]}
					/>
				</div>
				{filas === null ? (
					<div className="h-56" />
				) : total === 0 ? (
					<p className="rounded-lg border p-6 text-center text-xs text-muted-foreground">
						{t("Todavía no has terminado ninguna partida.")}
					</p>
				) : (
					<>
						<p className="text-xs tabular-nums">
							{t("Partidas: {partidas} · Victorias: {victorias} ({porcentaje} %)", {
								partidas: total,
								victorias,
								porcentaje: porcentajeVictorias(total, victorias),
							})}
							{hayFiltro(filtro) && ` · ${t("Con el filtro: {n}", { n: visibles.length })}`}
						</p>
						<ScrollArea className="rounded-lg border" viewportClassName="max-h-[26rem]">
							<div className="flex flex-col divide-y">
								{visibles.map((f) => (
									<Fila
										key={f.id}
										fila={f}
										fecha={fecha(f.terminada)}
										onBorrar={() => setBorrar(f.id)}
										onVerRepeticion={(id) =>
											void verRepeticionDelHistorial(id).then((ok) => ok && onCerrar())
										}
									/>
								))}
								{visibles.length === 0 && (
									<p className="p-6 text-center text-xs text-muted-foreground">
										{t("Ninguna partida cumple el filtro.")}
									</p>
								)}
							</div>
						</ScrollArea>
						{quedan && (
							<Button
								size="sm"
								variant="outline"
								className="self-center"
								disabled={cargando}
								onClick={() => void cargar(filas.length)}
							>
								{t("Cargar más ({n})", { n: total - filas.length })}
							</Button>
						)}
					</>
				)}
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
							await cargar(0);
						} catch (e) {
							toast.error(mensaje(e));
						}
					}}
				/>
			</DialogContent>
		</Dialog>
	);
}

function Fila({
	fila,
	fecha,
	onBorrar,
	onVerRepeticion,
}: {
	fila: FilaPartida;
	fecha: string;
	onBorrar: () => void;
	onVerRepeticion: (id: string) => void;
}) {
	const { t } = useLang();
	const como = COMO.find((c) => c.valor === fila.como)?.nombre;
	const asientos = fila.jugadores.slice(0, MAX_ASIENTOS_FILA);
	const resto = fila.jugadores.length - asientos.length;
	return (
		<div className="flex items-start gap-2 p-2 text-xs">
			<span
				className={cn(
					"flex w-20 shrink-0 items-center gap-1 pt-0.5 font-medium",
					fila.gano ? "text-primary" : "text-muted-foreground",
				)}
			>
				{fila.gano && <Trophy className="size-3.5" />}
				{fila.gano
					? t("Victoria")
					: fila.puesto === 0
						? "—"
						: t("Puesto {puesto}", { puesto: fila.puesto })}
			</span>
			<span className="flex min-w-0 flex-1 flex-col gap-0.5">
				{asientos.map((j) => (
					<Asiento key={j.jugador} j={j} solo={fila.jugadores.length === 1} />
				))}
				{resto > 0 && (
					<span className="text-[11px] text-muted-foreground">{t("y {n} más", { n: resto })}</span>
				)}
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
			{fila.repetible && (
				<Button size="xs" variant="outline" onClick={() => onVerRepeticion(fila.id)}>
					{t("Ver repetición")}
				</Button>
			)}
			<WithTooltip content={t("Borrar")}>
				<Button size="icon-sm" variant="ghost" onClick={onBorrar} aria-label={t("Borrar")}>
					<Trash2 className="size-3.5" />
				</Button>
			</WithTooltip>
		</div>
	);
}

/** Un asiento: quién, con qué deck y con qué lista (si no fue el deck entero). */
function Asiento({ j, solo }: { j: JugadorHistorial; solo: boolean }) {
	const { t } = useLang();
	return (
		<span className="flex min-w-0 items-center gap-1.5">
			<span className="size-2 shrink-0 rounded-full" style={{ background: j.color }} />
			{!solo && <span className="shrink-0 font-medium">{j.nombre}</span>}
			<span className="truncate">{j.deck || t("Deck de otro jugador")}</span>
			{j.lista !== "" && (
				<span className="shrink-0 rounded bg-muted px-1 text-[10px] text-muted-foreground">
					{t("Lista: {lista} · {n} cartas", { lista: j.lista, n: j.cartasLista })}
				</span>
			)}
			{j.lista === "" && j.listaId !== "" && (
				<span className="shrink-0 rounded bg-muted px-1 text-[10px] text-muted-foreground">
					{t("Lista borrada")}
				</span>
			)}
			{!solo && j.puesto > 0 && (
				<span className="ml-auto shrink-0 text-[10px] text-muted-foreground tabular-nums">
					{t("Puesto {puesto}", { puesto: j.puesto })}
				</span>
			)}
		</span>
	);
}
