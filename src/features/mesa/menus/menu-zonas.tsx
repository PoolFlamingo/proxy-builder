import {
	ArrowDownToLine,
	Dices,
	Eye,
	EyeOff,
	Grid3x3,
	Hand,
	LayoutGrid,
	MousePointerClick,
	RefreshCw,
	RotateCcw,
	Search,
	Shuffle,
	Skull,
	Sparkles,
	SquareStack,
	Trash2,
	Trees,
	Unlink,
} from "lucide-react";
import type { ReactNode } from "react";
import type { IdJugador } from "@/bindings/IdJugador";
import type { Zona } from "@/bindings/Zona";
import { ContenidoContextual } from "@/components/contenido-contextual";
import { paraTraducir, useLang } from "@/components/language-provider";
import {
	ContextMenu,
	ContextMenuItem,
	ContextMenuLabel,
	ContextMenuSeparator,
	ContextMenuShortcut,
	ContextMenuSub,
	ContextMenuSubContent,
	ContextMenuSubTrigger,
	ContextMenuTrigger,
} from "@/components/ui/context-menu";
import { useMesa } from "@/stores/mesa-store";
import { useRed } from "@/stores/red-store";
import { hacer } from "../acciones";
import { cartaDeArribaRevelada } from "../arriba-biblioteca";
import { teclaPara } from "../atajos";
import { COLORES } from "../colores";
import * as op from "../operaciones";
import { pedirNumero, usePantalla } from "../pantalla";

function Tecla({ contexto, nombre }: { contexto: "biblioteca" | "mesa"; nombre: string }) {
	const tecla = teclaPara(contexto, nombre);
	return tecla ? <ContextMenuShortcut>{tecla}</ContextMenuShortcut> : null;
}

/** El menú de la biblioteca (`capturas/pablo/10–11`, `14`). */
export function MenuBiblioteca({ jugador, children }: { jugador: IdJugador; children: ReactNode }) {
	const { t } = useLang();
	const yo = useMesa((s) => s.yo ?? 0);
	const nombre = useMesa((s) => s.jugadores[jugador]?.nombre ?? "");
	const revelada = useMesa((s) => s.jugadores[jugador]?.deArribaRevelada ?? false);
	// La de arriba está boca arriba por una revelación (no por jugar así): se puede desrevelar.
	const arribaVista = useMesa((s) => cartaDeArribaRevelada(s, jugador)?.revelada ?? false);
	// En la biblioteca de otro, lo que se hace es por él (o lo mira quien juega).
	const ajena = jugador !== yo;
	const soloMira = useRed((s) => s.papel === "espectador");
	if (soloMira) return <>{children}</>;
	return (
		<ContextMenu>
			<ContextMenuTrigger asChild>{children}</ContextMenuTrigger>
			<ContenidoContextual className="w-64 text-xs">
				{ajena && (
					<ContextMenuLabel className="text-xs">
						{t("Biblioteca de {quien}", { quien: nombre })}
					</ContextMenuLabel>
				)}
				<ContextMenuItem onSelect={() => op.robar(1, jugador)}>
					<Hand />
					{t("Robar")}
					<Tecla contexto="biblioteca" nombre="Robar" />
				</ContextMenuItem>
				<ContextMenuItem
					onSelect={() => pedirNumero(t("Robar cuántas"), 2, (n) => op.robar(n, jugador))}
				>
					<Hand />
					{t("Robar varias…")}
				</ContextMenuItem>
				<ContextMenuItem
					onSelect={() => pedirNumero(t("Moler cuántas"), 1, (n) => op.moler(n, jugador))}
				>
					<Skull />
					{t("Moler…")}
					<Tecla contexto="biblioteca" nombre="Moler" />
				</ContextMenuItem>
				<ContextMenuItem onSelect={() => op.hacerPor(jugador, { que: "manifestar", n: 1 })}>
					<EyeOff />
					{t("Manifestar")}
				</ContextMenuItem>
				<ContextMenuSub>
					<ContextMenuSubTrigger>
						<Trash2 />
						{t("Exiliar la de arriba")}
					</ContextMenuSubTrigger>
					<ContextMenuSubContent className="text-xs">
						<ContextMenuItem onSelect={() => op.exiliarDeArriba("normal", 1, jugador)}>
							{t("Boca arriba")}
							<Tecla contexto="biblioteca" nombre="Exiliar la de arriba" />
						</ContextMenuItem>
						<ContextMenuItem onSelect={() => op.exiliarDeArriba("bocaAbajo", 1, jugador)}>
							{t("Boca abajo (la puedes mirar)")}
							<Tecla contexto="biblioteca" nombre="Exiliar la de arriba boca abajo" />
						</ContextMenuItem>
						<ContextMenuItem onSelect={() => op.exiliarDeArriba("oculta", 1, jugador)}>
							{t("Oculta (nadie la ve)")}
						</ContextMenuItem>
						<ContextMenuItem
							onSelect={() =>
								pedirNumero(t("Exiliar cuántas"), 2, (n) =>
									op.exiliarDeArriba("normal", n, jugador),
								)
							}
						>
							{t("Varias boca arriba…")}
						</ContextMenuItem>
					</ContextMenuSubContent>
				</ContextMenuSub>
				<ContextMenuSeparator />
				<ContextMenuItem onSelect={() => op.mirar("mirar", "arriba", jugador)}>
					<Eye />
					{t("Mirar las de arriba…")}
					<Tecla contexto="biblioteca" nombre="Mirar la de arriba" />
				</ContextMenuItem>
				<ContextMenuItem onSelect={() => op.mirar("mirar", "abajo", jugador)}>
					<ArrowDownToLine />
					{t("Mirar las de abajo…")}
				</ContextMenuItem>
				<ContextMenuItem onSelect={() => op.mirar("escrutar", "arriba", jugador)}>
					<Eye />
					{t("Escrutar…")}
					<Tecla contexto="biblioteca" nombre="Escrutar" />
				</ContextMenuItem>
				<ContextMenuItem onSelect={() => op.mirar("vigilar", "arriba", jugador)}>
					<Eye />
					{t("Vigilar…")}
					<Tecla contexto="biblioteca" nombre="Vigilar" />
				</ContextMenuItem>
				<ContextMenuItem onSelect={() => usePantalla.getState().abrir({ tipo: "revelar" })}>
					<Sparkles />
					{t("Revelar y lanzar…")}
				</ContextMenuItem>
				<ContextMenuSeparator />
				<ContextMenuItem onSelect={() => op.buscar("todo", jugador)}>
					<Search />
					{t("Buscar una carta")}
					<Tecla contexto="biblioteca" nombre="Buscar una carta" />
				</ContextMenuItem>
				<ContextMenuItem onSelect={() => op.buscar("tierra", jugador)}>
					<Trees />
					{t("Buscar una tierra")}
					<Tecla contexto="biblioteca" nombre="Buscar una tierra" />
				</ContextMenuItem>
				<ContextMenuItem onSelect={() => op.hacerPor(jugador, { que: "revelarArriba", n: 1 })}>
					<Eye />
					{t("Revelar la de arriba")}
				</ContextMenuItem>
				{arribaVista && (
					<ContextMenuItem onSelect={() => op.hacerPor(jugador, { que: "ocultarArriba", n: 1 })}>
						<EyeOff />
						{t("Dejar de revelar la de arriba")}
					</ContextMenuItem>
				)}
				<ContextMenuItem
					onSelect={() => op.hacerPor(jugador, { que: "deArribaRevelada", si: !revelada })}
				>
					{revelada ? <EyeOff /> : <Eye />}
					{revelada
						? t("Dejar de jugar con la de arriba revelada")
						: t("Jugar con la de arriba revelada")}
				</ContextMenuItem>
				<ContextMenuSeparator />
				<ContextMenuItem onSelect={() => op.barajar(jugador)}>
					<Shuffle />
					{t("Barajar")}
					<Tecla contexto="biblioteca" nombre="Barajar" />
				</ContextMenuItem>
				<ContextMenuItem onSelect={() => op.hacerPor(jugador, { que: "rueda", robar: null })}>
					<RefreshCw />
					{t("Rueda (la mano al fondo, barajar y robar las mismas)")}
				</ContextMenuItem>
			</ContenidoContextual>
		</ContextMenu>
	);
}

/** El menú de cementerio, exilio, mando o banquillo: verlo y lo de todas a la vez. */
export function MenuZona({
	zona,
	jugador,
	children,
}: {
	zona: Zona;
	jugador: IdJugador;
	children: ReactNode;
}) {
	const { t } = useLang();
	const ver = () => usePantalla.getState().abrir({ tipo: "zona", jugador, zona });
	const masiva = zona === "cementerio" || zona === "exilio";
	const soloMira = useRed((s) => s.papel === "espectador");
	if (soloMira) return <>{children}</>;
	return (
		<ContextMenu>
			<ContextMenuTrigger asChild>{children}</ContextMenuTrigger>
			<ContenidoContextual className="w-64 text-xs">
				<ContextMenuItem onSelect={ver}>
					<Grid3x3 />
					{t("Ver")}
				</ContextMenuItem>
				{masiva && (
					<>
						<ContextMenuSub>
							<ContextMenuSubTrigger>
								<Dices />
								{t("Una al azar…")}
							</ContextMenuSubTrigger>
							<ContextMenuSubContent className="text-xs">
								{(
									[
										["mano", paraTraducir("A la mano")],
										["campo", paraTraducir("Al campo")],
										["exilio", paraTraducir("Al exilio")],
									] as [Zona, string][]
								)
									.filter(([a]) => a !== zona)
									.map(([a, nombre]) => (
										<ContextMenuItem
											key={a}
											onSelect={() =>
												op.hacerPor(jugador, {
													que: "alAzarDe",
													zona,
													a,
													filtro: {
														noTierra: false,
														supertipo: null,
														tipo: null,
														cmc: null,
														colores: [],
														nombre: null,
													},
												})
											}
										>
											{t(nombre)}
										</ContextMenuItem>
									))}
							</ContextMenuSubContent>
						</ContextMenuSub>
						<ContextMenuItem
							onSelect={() => op.hacerPor(jugador, { que: "barajarTodoDentro", zona })}
						>
							<Shuffle />
							{t("Barajar todas en la biblioteca")}
						</ContextMenuItem>
						<ContextMenuItem
							onSelect={() => op.hacerPor(jugador, { que: "todoAlFondoAlAzar", zona })}
						>
							<ArrowDownToLine />
							{t("Todas al fondo, en orden al azar")}
						</ContextMenuItem>
						{zona !== "exilio" && (
							<ContextMenuItem onSelect={() => op.hacerPor(jugador, { que: "exiliarTodo", zona })}>
								<Trash2 />
								{t("Exiliarlas todas")}
							</ContextMenuItem>
						)}
					</>
				)}
			</ContenidoContextual>
		</ContextMenu>
	);
}

/** El menú del campo vacío (`capturas/pablo/18`). */
export function MenuCampo({
	jugador,
	children,
	posicion,
}: {
	/** De quién es el campo (el que se mira). */
	jugador: IdJugador;
	children: ReactNode;
	/** Dónde se abrió el menú, en posición del motor (para la bolita). */
	posicion: () => { x: number; y: number };
}) {
	const { t } = useLang();
	const soloMira = useRed((s) => s.papel === "espectador");
	if (soloMira) return <>{children}</>;
	return (
		<ContextMenu>
			<ContextMenuTrigger asChild>{children}</ContextMenuTrigger>
			<ContenidoContextual className="w-60 text-xs">
				<ContextMenuItem onSelect={() => op.enderezarTodo(jugador)}>
					<RotateCcw />
					{t("Enderezar todo")}
					<Tecla contexto="mesa" nombre="Enderezar todo" />
				</ContextMenuItem>
				<ContextMenuItem
					onSelect={() => {
						const m = useMesa.getState();
						m.seleccionar(m.zonas[`${jugador}:campo`]?.cartas ?? []);
					}}
				>
					<MousePointerClick />
					{t("Elegir todo")}
					<Tecla contexto="mesa" nombre="Elegir todo el campo" />
				</ContextMenuItem>
				<ContextMenuItem onSelect={() => op.hacerPor(jugador, { que: "organizar" })}>
					<LayoutGrid />
					{t("Organizar el campo")}
				</ContextMenuItem>
				<ContextMenuItem
					onSelect={() => usePantalla.getState().abrir({ tipo: "fichas", para: jugador })}
				>
					<SquareStack />
					{t("Crear ficha…")}
				</ContextMenuItem>
				<ContextMenuItem
					onSelect={() =>
						usePantalla.getState().abrir({ tipo: "zona", jugador, zona: "cementerio" })
					}
				>
					<Skull />
					{t("Cementerios y exilios")}
				</ContextMenuItem>
				<ContextMenuItem onSelect={() => hacer({ que: "quitarFlechas", de: null })}>
					<Unlink />
					{t("Quitar mis flechas")}
				</ContextMenuItem>
				<ContextMenuSub>
					<ContextMenuSubTrigger>{t("Poner una bolita aquí")}</ContextMenuSubTrigger>
					<ContextMenuSubContent className="text-xs">
						<div className="grid grid-cols-8 gap-1 p-1.5">
							{COLORES.map((color, i) => (
								<button
									key={color}
									type="button"
									aria-label={color}
									onClick={() =>
										void hacer({
											que: "ponerBolita",
											campoDe: jugador,
											pos: posicion(),
											color: i,
											tamano: "M",
										})
									}
									className="size-5 rounded-full ring-1 ring-white/20"
									style={{ background: color }}
								/>
							))}
						</div>
					</ContextMenuSubContent>
				</ContextMenuSub>
			</ContenidoContextual>
		</ContextMenu>
	);
}

/**
 * **El menú de la mano de otro jugador**: mirarla (Peek, Thoughtseize: sus
 * cartas se ven boca arriba en su sitio y, como se ven, se pueden mover, p. ej.
 * al cementerio), hacer que descarte al azar o entera, que la enseñe a todos
 * (Telepathy) o que robe. Queda todo en el registro.
 */
export function MenuManoAjena({ jugador, children }: { jugador: IdJugador; children: ReactNode }) {
	const { t } = useLang();
	const nombre = useMesa((s) => s.jugadores[jugador]?.nombre ?? "");
	const mirando = useMesa((s) => (s.zonas[`${jugador}:mano`]?.visibles.length ?? 0) > 0);
	const revelada = useMesa((s) => s.jugadores[jugador]?.manoRevelada ?? false);
	const soloMira = useRed((s) => s.papel === "espectador");
	if (soloMira) return <>{children}</>;
	return (
		<ContextMenu>
			<ContextMenuTrigger asChild>{children}</ContextMenuTrigger>
			<ContenidoContextual className="w-64 text-xs">
				<ContextMenuLabel className="text-xs">
					{t("Mano de {quien}", { quien: nombre })}
				</ContextMenuLabel>
				<ContextMenuItem onSelect={() => op.mirarMano(jugador, !mirando)}>
					{mirando ? <EyeOff /> : <Eye />}
					{mirando ? t("Dejar de mirar su mano") : t("Mirar su mano")}
				</ContextMenuItem>
				<ContextMenuItem
					onSelect={() => op.hacerPor(jugador, { que: "revelarMano", si: !revelada })}
				>
					<Eye />
					{revelada ? t("Que deje de enseñar su mano") : t("Que enseñe su mano a todos")}
				</ContextMenuItem>
				<ContextMenuSeparator />
				<ContextMenuItem
					onSelect={() =>
						pedirNumero(t("Descartar cuántas al azar"), 1, (n) =>
							op.hacerPor(jugador, { que: "descartar", modo: { tipo: "alAzar", valor: n } }),
						)
					}
				>
					<Dices />
					{t("Que descarte al azar…")}
				</ContextMenuItem>
				<ContextMenuItem
					onSelect={() => op.hacerPor(jugador, { que: "descartar", modo: { tipo: "todas" } })}
				>
					<Trash2 />
					{t("Que descarte toda la mano")}
				</ContextMenuItem>
				<ContextMenuItem
					onSelect={() => pedirNumero(t("Robar cuántas"), 1, (n) => op.robar(n, jugador))}
				>
					<Hand />
					{t("Que robe…")}
				</ContextMenuItem>
			</ContenidoContextual>
		</ContextMenu>
	);
}
