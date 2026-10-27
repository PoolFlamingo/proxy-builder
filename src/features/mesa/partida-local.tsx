import { DndContext, type DragEndEvent, PointerSensor, useSensor, useSensors } from "@dnd-kit/core";
import { Loader2, Play, Plus, Shuffle, X } from "lucide-react";
import { useEffect, useState } from "react";
import { toast } from "sonner";
import type { DeckSummary } from "@/bindings/DeckSummary";
import type { Formato } from "@/bindings/Formato";
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
	const nombreDeAsiento = (i: number) => asientos[i]?.nombre.trim() || `${t("Jugador")} ${i + 1}`;
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
										onChange={(v) => cambiar(i, { deckId: v })}
										ancho="min-w-0 flex-1"
										opciones={decks.map((d) => ({ valor: d.id, etiqueta: d.name }))}
									/>
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
