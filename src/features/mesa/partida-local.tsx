import { DndContext, type DragEndEvent, PointerSensor, useSensor, useSensors } from "@dnd-kit/core";
import { ListChecks, Loader2, Pencil, Play, Plus, Shuffle, X } from "lucide-react";
import { useEffect, useState } from "react";
import { toast } from "sonner";
import type { DeckSummary } from "@/bindings/DeckSummary";
import type { Formato } from "@/bindings/Formato";
import type { ListaDeJuego } from "@/bindings/ListaDeJuego";
import type { ListasDeDeck } from "@/bindings/ListasDeDeck";
import { Fila, Selector } from "@/components/campos";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import { WithTooltip } from "@/components/ui/with-tooltip";
import { call } from "@/lib/ipc";
import {
	AjustesDePartida,
	AvisoDeEquipos,
	FilaEmpieza,
	FilaOrdenable,
	SelectorDeEquipo,
} from "./ajustes-partida";
import { COLORES_JUGADOR } from "./colores";
import { empezar } from "./conexion";
import {
	AJUSTES_LOCALES,
	type AjustesLocal,
	type AsientoLocal,
	ajustesTrasOrden,
	ajustesTrasQuitar,
	aplicarOrden,
	construirConfig,
	MAX_JUGADORES,
	ordenAlAzar,
	ordenTrasMover,
	problemaDeEquipos,
} from "./config-local";
import { ListaDialog } from "./listas/lista-dialog";

/** El valor del selector de listas para jugar con el deck entero (Radix no admite «»). */
const TODO_EL_DECK = "*";

/** El deck pasa del tamaño del formato: hay que elegir una lista (como `Construccion::pide_lista`). */
const pideLista = (l: ListasDeDeck | undefined, formato: Formato) =>
	formato === "commander" && (l?.cuentan ?? 0) > 100;

/**
 * **Partida local** (Fase G): de 2 a 10 jugadores en este mismo equipo, cada
 * uno con un deck de la biblioteca. Quien mira es quien juega: los chips de
 * jugador de la mesa pasan de un tablero a otro, con su mano a la vista.
 */
export function PartidaLocalDialog({
	abierto,
	onCerrar,
	decks,
	deckInicial,
}: {
	abierto: boolean;
	onCerrar: () => void;
	decks: DeckSummary[];
	deckInicial: string | null;
}) {
	const { t, mensaje } = useLang();
	const [ajustes, setAjustes] = useState<AjustesLocal>(AJUSTES_LOCALES);
	const [asientos, setAsientos] = useState<AsientoLocal[]>([]);
	const [empezando, setEmpezando] = useState(false);
	// Las listas para jugar de cada deck que se sienta a la mesa, y la que se edita.
	const [listas, setListas] = useState<Record<string, ListasDeDeck>>({});
	const [editando, setEditando] = useState<{ asiento: number; lista: ListaDeJuego | null } | null>(
		null,
	);

	// Al abrir: dos asientos con el deck que hubiera elegido en la entrada.
	useEffect(() => {
		if (!abierto) return;
		const d = deckInicial ?? decks[0]?.id ?? "";
		setAsientos([
			{ nombre: `${t("Jugador")} 1`, deckId: d, equipo: null },
			{ nombre: `${t("Jugador")} 2`, deckId: d, equipo: null },
		]);
	}, [abierto, deckInicial, decks, t]);

	// Se leen las listas de cada deck elegido (una vez por deck).
	useEffect(() => {
		if (!abierto) return;
		for (const id of new Set(asientos.map((a) => a.deckId))) {
			if (!id || listas[id]) continue;
			call("mesa_listas", { deck: id })
				.then((l) => setListas((x) => ({ ...x, [id]: l })))
				.catch(() => {});
		}
	}, [abierto, asientos, listas]);

	// Un deck que pasa del tamaño del formato empieza con su primera lista (se puede cambiar).
	useEffect(() => {
		setAsientos((a) => {
			let cambia = false;
			const n = a.map((x) => {
				const l = listas[x.deckId];
				if (x.lista === undefined && pideLista(l, ajustes.formato) && l?.listas[0]) {
					cambia = true;
					return { ...x, lista: l.listas[0].id };
				}
				return x;
			});
			return cambia ? n : a;
		});
	}, [listas, ajustes.formato]);

	const cambiar = (i: number, c: Partial<AsientoLocal>) =>
		setAsientos((a) => a.map((x, k) => (k === i ? { ...x, ...c } : x)));
	const cambiarAjustes = (c: Partial<AjustesLocal>) => setAjustes((a) => ({ ...a, ...c }));
	/** Cambia el orden de los asientos; quien empieza y el archienemigo se quedan con su jugador. */
	const reordenar = (orden: number[]) => {
		setAsientos((a) => aplicarOrden(a, orden));
		setAjustes((a) => ajustesTrasOrden(a, orden));
	};
	const quitar = (i: number) => {
		setAsientos((a) => a.filter((_, k) => k !== i));
		setAjustes((a) => ajustesTrasQuitar(a, i));
	};
	const sensores = useSensors(useSensor(PointerSensor, { activationConstraint: { distance: 4 } }));
	const alSoltar = (e: DragEndEvent) => {
		if (e.over && e.over.id !== e.active.id) {
			reordenar(ordenTrasMover(asientos.length, Number(e.active.id), Number(e.over.id)));
		}
	};
	const problema = problemaDeEquipos(asientos, ajustes.modo);
	const sinLista = (a: AsientoLocal) => pideLista(listas[a.deckId], ajustes.formato) && !a.lista;
	const completo =
		asientos.length >= 2 &&
		asientos.every((a) => a.deckId && a.nombre.trim() && !sinLista(a)) &&
		!problema;
	const formato = ajustes.formato;
	const nombreDeEquipo = (n: number) => t("Equipo {n}", { n });
	const nombreDeAsiento = (i: number) => asientos[i]?.nombre.trim() || `${t("Jugador")} ${i + 1}`;
	const porEquipos = ajustes.modo === "equipos";

	const iniciar = async () => {
		setEmpezando(true);
		try {
			const config = construirConfig(
				asientos,
				ajustes,
				nombreDeEquipo,
				t("Contra el archienemigo"),
			);
			if (await empezar(config)) onCerrar();
		} catch (e) {
			toast.error(mensaje(e));
		} finally {
			setEmpezando(false);
		}
	};

	return (
		<Dialog open={abierto} onOpenChange={(v) => !v && onCerrar()}>
			<DialogContent className="max-w-3xl">
				<DialogHeader>
					<DialogTitle>{t("Partida local")}</DialogTitle>
					<DialogDescription>
						{t(
							"Varios jugadores en este equipo, cada uno con su deck. Pasas de un tablero a otro con los chips de jugador.",
						)}
					</DialogDescription>
				</DialogHeader>
				<Fila etiqueta={t("Formato")}>
					<Selector<Formato>
						valor={formato}
						onChange={(f) => cambiarAjustes({ formato: f })}
						opciones={[
							{ valor: "commander", etiqueta: "Commander" },
							{ valor: "constructed", etiqueta: t("60 cartas") },
							{ valor: "limited", etiqueta: t("Limitado") },
							{ valor: "libre", etiqueta: t("Libre") },
						]}
					/>
				</Fila>
				<AjustesDePartida
					ajustes={ajustes}
					onChange={cambiarAjustes}
					formato={formato}
					asientos={asientos.length}
					nombreDeAsiento={nombreDeAsiento}
				/>
				<ScrollArea className="max-h-80">
					<DndContext sensors={sensores} onDragEnd={alSoltar}>
						<div className="flex flex-col gap-1.5 p-1">
							{asientos.map((a, i) => (
								<FilaOrdenable
									key={i}
									i={i}
									etiqueta={t("Mover a {jugador}", {
										jugador: a.nombre.trim() || `${t("Jugador")} ${i + 1}`,
									})}
								>
									<span
										className="size-3 shrink-0 rounded-full"
										style={{ background: COLORES_JUGADOR[i % COLORES_JUGADOR.length] }}
									/>
									<Input
										value={a.nombre}
										onChange={(e) => cambiar(i, { nombre: e.target.value })}
										className="h-7 w-36 text-xs"
										aria-label={t("Nombre")}
									/>
									<Selector<string>
										valor={a.deckId}
										onChange={(v) => cambiar(i, { deckId: v, lista: undefined })}
										ancho="min-w-0 flex-1"
										opciones={decks.map((d) => ({ valor: d.id, etiqueta: d.name }))}
									/>
									{(listas[a.deckId]?.listas.length ?? 0) > 0 ||
									pideLista(listas[a.deckId], formato) ? (
										<ListaDeAsiento
											lista={listas[a.deckId]}
											valor={a.lista}
											obligatoria={pideLista(listas[a.deckId], formato)}
											onChange={(lista) => cambiar(i, { lista })}
											onEditar={() =>
												setEditando({
													asiento: i,
													lista: listas[a.deckId]?.listas.find((l) => l.id === a.lista) ?? null,
												})
											}
										/>
									) : null}
									{porEquipos && (
										<SelectorDeEquipo
											equipo={a.equipo}
											onChange={(equipo) => cambiar(i, { equipo })}
										/>
									)}
									<Button
										size="icon"
										variant="ghost"
										className="size-7"
										disabled={asientos.length <= 2}
										onClick={() => quitar(i)}
										aria-label={t("Quitar")}
									>
										<X className="size-3.5" />
									</Button>
								</FilaOrdenable>
							))}
						</div>
					</DndContext>
				</ScrollArea>
				<FilaEmpieza
					ajustes={ajustes}
					onChange={cambiarAjustes}
					asientos={asientos.length}
					nombreDeAsiento={nombreDeAsiento}
				/>
				<p className="text-xs text-muted-foreground">
					{t(
						"Se juega por asientos, de arriba abajo: arrastra a los jugadores para cambiar el orden.",
					)}
				</p>
				<div className="flex gap-2">
					<Button
						size="sm"
						variant="outline"
						disabled={asientos.length >= MAX_JUGADORES}
						onClick={() =>
							setAsientos((x) => [
								...x,
								{
									nombre: `${t("Jugador")} ${x.length + 1}`,
									deckId: x[x.length - 1]?.deckId ?? decks[0]?.id ?? "",
									equipo: null,
								},
							])
						}
					>
						<Plus className="size-3.5" />
						{t("Añadir jugador")}
					</Button>
					<Button
						size="sm"
						variant="outline"
						onClick={() => reordenar(ordenAlAzar(asientos.length))}
					>
						<Shuffle className="size-3.5" />
						{t("Mezclar los asientos")}
					</Button>
				</div>
				{asientos.some(sinLista) && (
					<p className="flex items-start gap-1.5 rounded-md bg-amber-500/10 p-1.5 text-[11px] text-amber-600 dark:text-amber-300">
						<ListChecks className="mt-0.5 size-3.5 shrink-0" />
						{t(
							"{jugadores}: su deck tiene más de 100 cartas y en Commander se juega con 100. Haz una lista con el comandante y 99 más.",
							{
								jugadores: asientos
									.map((a, i) => (sinLista(a) ? nombreDeAsiento(i) : null))
									.filter(Boolean)
									.join(", "),
							},
						)}
					</p>
				)}
				{editando && asientos[editando.asiento] && (
					<ListaDialog
						abierto
						onCerrar={() => setEditando(null)}
						deckId={asientos[editando.asiento]?.deckId ?? ""}
						formato={formato}
						lista={editando.lista}
						onGuardada={(l) => {
							const deck = asientos[editando.asiento]?.deckId ?? "";
							setListas((x) => {
								const actual = x[deck];
								return actual
									? {
											...x,
											[deck]: {
												...actual,
												listas: [...actual.listas.filter((y) => y.id !== l.id), l].sort((a, b) =>
													a.nombre.localeCompare(b.nombre),
												),
											},
										}
									: x;
							});
							cambiar(editando.asiento, { lista: l.id });
						}}
						onBorrada={(id) => {
							const deck = asientos[editando.asiento]?.deckId ?? "";
							setListas((x) => {
								const actual = x[deck];
								return actual
									? {
											...x,
											[deck]: { ...actual, listas: actual.listas.filter((y) => y.id !== id) },
										}
									: x;
							});
							setAsientos((a) => a.map((y) => (y.lista === id ? { ...y, lista: undefined } : y)));
						}}
					/>
				)}
				<AvisoDeEquipos asientos={asientos} modo={ajustes.modo} />
				<DialogFooter>
					<Button variant="outline" onClick={onCerrar}>
						{t("Cancelar")}
					</Button>
					<Button disabled={!completo || empezando} onClick={() => void iniciar()}>
						{empezando ? <Loader2 className="size-4 animate-spin" /> : <Play className="size-4" />}
						{t("Empezar")}
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	);
}

/**
 * La lista para jugar de un asiento: la de la lista de este deck o el deck
 * entero (si no pasa del tamaño del formato), y un botón para crear o editar
 * la elegida. Cada jugador tiene la suya.
 */
function ListaDeAsiento({
	lista,
	valor,
	obligatoria,
	onChange,
	onEditar,
}: {
	lista: ListasDeDeck | undefined;
	valor: string | undefined;
	obligatoria: boolean;
	onChange: (id: string | undefined) => void;
	onEditar: () => void;
}) {
	const { t } = useLang();
	return (
		<>
			<Selector<string>
				valor={valor ?? TODO_EL_DECK}
				onChange={(v) => onChange(v === TODO_EL_DECK ? undefined : v)}
				ancho="w-36"
				ariaLabel={t("Lista para jugar")}
				opciones={[
					...(obligatoria
						? []
						: [
								{
									valor: TODO_EL_DECK,
									etiqueta: t("Todo el deck ({n})", { n: lista?.cuentan ?? 0 }),
								},
							]),
					...(lista?.listas ?? []).map((l) => ({ valor: l.id, etiqueta: l.nombre })),
				]}
			/>
			<WithTooltip content={valor ? t("Editar la lista…") : t("Nueva lista…")}>
				<Button
					size="icon"
					variant="outline"
					className="size-7"
					onClick={onEditar}
					aria-label={valor ? t("Editar la lista…") : t("Nueva lista…")}
				>
					{valor ? <Pencil className="size-3.5" /> : <Plus className="size-3.5" />}
				</Button>
			</WithTooltip>
		</>
	);
}
