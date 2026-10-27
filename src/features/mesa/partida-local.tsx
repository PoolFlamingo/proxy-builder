import {
	DndContext,
	type DragEndEvent,
	PointerSensor,
	useDraggable,
	useDroppable,
	useSensor,
	useSensors,
} from "@dnd-kit/core";
import { GripVertical, Loader2, Play, Plus, Shuffle, X } from "lucide-react";
import { useEffect, useState } from "react";
import { toast } from "sonner";
import type { DeckSummary } from "@/bindings/DeckSummary";
import type { Formato } from "@/bindings/Formato";
import { Fila, FilaInterruptor, Selector } from "@/components/campos";
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
import { call } from "@/lib/ipc";
import { cn } from "@/lib/utils";
import { COLORES_EQUIPO, COLORES_JUGADOR } from "./colores";
import { empezar } from "./conexion";
import {
	AJUSTES_LOCALES,
	type AjustesLocal,
	type AlMejorDe,
	type AsientoLocal,
	ajustesTrasOrden,
	ajustesTrasQuitar,
	aplicarOrden,
	asientoArchienemigo,
	conBanquillo,
	construirConfig,
	MAX_EQUIPOS,
	MAX_JUGADORES,
	type ModoEquiposLocal,
	ordenAlAzar,
	ordenTrasMover,
	problemaDeEquipos,
} from "./config-local";

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

	// Al abrir: dos asientos con el deck que hubiera elegido en la entrada.
	useEffect(() => {
		if (!abierto) return;
		const d = deckInicial ?? decks[0]?.id ?? "";
		setAsientos([
			{ nombre: `${t("Jugador")} 1`, deckId: d, equipo: null },
			{ nombre: `${t("Jugador")} 2`, deckId: d, equipo: null },
		]);
	}, [abierto, deckInicial, decks, t]);

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
	const completo =
		asientos.length >= 2 && asientos.every((a) => a.deckId && a.nombre.trim()) && !problema;
	const formato = ajustes.formato;
	const nombreDeEquipo = (n: number) => t("Equipo {n}", { n });
	const porEquipos = ajustes.modo === "equipos";

	const iniciar = async () => {
		setEmpezando(true);
		try {
			// Un deck que pasa del tamaño del formato juega con su primera lista.
			const conListas: AsientoLocal[] = [];
			for (const a of asientos) {
				let lista: string | undefined;
				if (formato === "commander") {
					const l = await call("mesa_listas", { deck: a.deckId });
					if (l.cuentan > 100) lista = l.listas[0]?.id;
				}
				conListas.push({ ...a, lista });
			}
			const config = construirConfig(
				conListas,
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
			<DialogContent className="max-w-2xl">
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
				{conBanquillo(formato) && (
					<Fila etiqueta={t("Partidas")}>
						<Selector<string>
							valor={String(ajustes.alMejorDe)}
							onChange={(v) => cambiarAjustes({ alMejorDe: Number(v) as AlMejorDe })}
							opciones={[
								{ valor: "1", etiqueta: t("Una sola") },
								{ valor: "3", etiqueta: t("Al mejor de 3") },
								{ valor: "5", etiqueta: t("Al mejor de 5") },
							]}
						/>
					</Fila>
				)}
				<Fila etiqueta={t("Equipos")}>
					<Selector<ModoEquiposLocal>
						ancho="w-56"
						valor={ajustes.modo}
						onChange={(modo) => cambiarAjustes({ modo })}
						opciones={[
							{ valor: "ninguno", etiqueta: t("Sin equipos") },
							{ valor: "equipos", etiqueta: t("Por equipos") },
							{ valor: "dosCabezas", etiqueta: t("Dos Cabezas") },
							{ valor: "archienemigo", etiqueta: t("Un archienemigo contra todos") },
						]}
					/>
				</Fila>
				{ajustes.modo === "archienemigo" && (
					<Fila etiqueta={t("Archienemigo")}>
						<Selector<string>
							valor={String(asientoArchienemigo(ajustes, asientos.length))}
							onChange={(v) => cambiarAjustes({ archienemigo: Number(v) })}
							opciones={asientos.map((a, i) => ({
								valor: String(i),
								etiqueta: a.nombre.trim() || `${t("Jugador")} ${i + 1}`,
							}))}
						/>
					</Fila>
				)}
				{porEquipos && (
					<>
						<FilaInterruptor
							etiqueta={t("Fuego amigo")}
							ayuda={t("Los efectos «a todos los oponentes» también alcanzan a tus compañeros.")}
							valor={ajustes.fuegoAmigo}
							onChange={(fuegoAmigo) => cambiarAjustes({ fuegoAmigo })}
						/>
						<FilaInterruptor
							etiqueta={t("Vida compartida")}
							ayuda={t("Cada equipo tiene una sola vida, la de todos sus jugadores.")}
							valor={ajustes.vidaCompartida}
							onChange={(vidaCompartida) => cambiarAjustes({ vidaCompartida })}
						/>
					</>
				)}
				{ajustes.modo !== "ninguno" && (
					<FilaInterruptor
						etiqueta={t("Los compañeros ven su mano")}
						ayuda={t(
							"Quienes van en un mismo equipo ven la mano del otro sin pedirla. Los demás solo ven sus reversos.",
						)}
						valor={ajustes.manoDelCompanero}
						onChange={(manoDelCompanero) => cambiarAjustes({ manoDelCompanero })}
					/>
				)}
				<FilaInterruptor
					etiqueta={t("Planechase")}
					ayuda={t(
						"Un mazo común con los planos de los decks, el plano activo en la mesa y el dado planar.",
					)}
					valor={ajustes.planechase}
					onChange={(planechase) => cambiarAjustes({ planechase })}
				/>
				{ajustes.modo === "archienemigo" && (
					<p className="text-xs text-muted-foreground">
						{t(
							"El archienemigo empieza con 40 vidas y pone en marcha los planes de su deck; los demás van juntos contra él.",
						)}
					</p>
				)}
				{ajustes.modo === "dosCabezas" && (
					<p className="text-xs text-muted-foreground">
						{t("Los equipos son las parejas por orden: 1 y 2, 3 y 4…")}
					</p>
				)}
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
										onChange={(v) => cambiar(i, { deckId: v })}
										ancho="min-w-0 flex-1"
										opciones={decks.map((d) => ({ valor: d.id, etiqueta: d.name }))}
									/>
									{porEquipos && (
										<Selector<string>
											valor={a.equipo === null ? "-" : String(a.equipo)}
											onChange={(v) => cambiar(i, { equipo: v === "-" ? null : Number(v) })}
											ancho="w-32"
											ariaLabel={t("Equipo")}
											opciones={[
												{ valor: "-", etiqueta: t("Sin equipo") },
												...COLORES_EQUIPO.slice(0, MAX_EQUIPOS).map((_, e) => ({
													valor: String(e),
													etiqueta: nombreDeEquipo(e + 1),
												})),
											]}
										/>
									)}
									{porEquipos && a.equipo !== null && (
										<span
											className="size-3 shrink-0 rounded-full"
											style={{ background: COLORES_EQUIPO[a.equipo] }}
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
				<Fila etiqueta={t("Empieza")}>
					<Selector<string>
						valor={ajustes.primero === null ? "azar" : String(ajustes.primero)}
						onChange={(v) => cambiarAjustes({ primero: v === "azar" ? null : Number(v) })}
						opciones={[
							{ valor: "azar", etiqueta: t("Al azar") },
							...asientos.map((a, i) => ({
								valor: String(i),
								etiqueta: a.nombre.trim() || `${t("Jugador")} ${i + 1}`,
							})),
						]}
					/>
				</Fila>
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
				{problema && (
					<p className="text-xs text-amber-500">
						{problema === "dosCabezasPares"
							? t("Dos Cabezas necesita 4 jugadores o más, en número par.")
							: t("Hace falta un equipo con al menos dos jugadores.")}
					</p>
				)}
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
 * Una fila de asiento que se arrastra por su asa y acepta que otra caiga
 * encima: la que se suelta ocupa su sitio y las demás se corren.
 */
function FilaOrdenable({
	i,
	etiqueta,
	children,
}: {
	i: number;
	etiqueta: string;
	children: React.ReactNode;
}) {
	const origen = useDraggable({ id: i });
	const destino = useDroppable({ id: i });
	const m = origen.transform;
	return (
		<div
			ref={destino.setNodeRef}
			className={cn(
				"flex items-center gap-1.5 rounded-lg",
				destino.isOver && "bg-primary/10",
				origen.isDragging && "relative z-10 bg-card shadow-lg",
			)}
			style={m ? { transform: `translate3d(${m.x}px, ${m.y}px, 0)` } : undefined}
		>
			<button
				type="button"
				ref={origen.setNodeRef}
				{...origen.listeners}
				{...origen.attributes}
				aria-label={etiqueta}
				className="flex size-6 shrink-0 cursor-grab touch-none items-center justify-center rounded text-muted-foreground hover:bg-accent active:cursor-grabbing"
			>
				<GripVertical className="size-3.5" />
			</button>
			{children}
		</div>
	);
}
