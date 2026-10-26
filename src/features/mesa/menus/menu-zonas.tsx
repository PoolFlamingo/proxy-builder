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
import type { Zona } from "@/bindings/Zona";
import { ContenidoContextual } from "@/components/contenido-contextual";
import { paraTraducir, useLang } from "@/components/language-provider";
import {
	ContextMenu,
	ContextMenuItem,
	ContextMenuSeparator,
	ContextMenuShortcut,
	ContextMenuSub,
	ContextMenuSubContent,
	ContextMenuSubTrigger,
	ContextMenuTrigger,
} from "@/components/ui/context-menu";
import { useMesa } from "@/stores/mesa-store";
import { hacer } from "../acciones";
import { teclaPara } from "../atajos";
import { COLORES } from "../colores";
import * as op from "../operaciones";
import { pedirNumero, usePantalla } from "../pantalla";

function Tecla({ contexto, nombre }: { contexto: "biblioteca" | "mesa"; nombre: string }) {
	const tecla = teclaPara(contexto, nombre);
	return tecla ? <ContextMenuShortcut>{tecla}</ContextMenuShortcut> : null;
}

/** El menú de la biblioteca (`capturas/pablo/10–11`, `14`). */
export function MenuBiblioteca({ children }: { children: ReactNode }) {
	const { t } = useLang();
	const yo = useMesa((s) => s.yo ?? 0);
	const revelada = useMesa((s) => s.jugadores[yo]?.deArribaRevelada ?? false);
	return (
		<ContextMenu>
			<ContextMenuTrigger asChild>{children}</ContextMenuTrigger>
			<ContenidoContextual className="w-64 text-xs">
				<ContextMenuItem onSelect={() => op.robar()}>
					<Hand />
					{t("Robar")}
					<Tecla contexto="biblioteca" nombre="Robar" />
				</ContextMenuItem>
				<ContextMenuItem onSelect={() => pedirNumero(t("Robar cuántas"), 2, (n) => op.robar(n))}>
					<Hand />
					{t("Robar varias…")}
				</ContextMenuItem>
				<ContextMenuItem onSelect={() => pedirNumero(t("Moler cuántas"), 1, (n) => op.moler(n))}>
					<Skull />
					{t("Moler…")}
					<Tecla contexto="biblioteca" nombre="Moler" />
				</ContextMenuItem>
				<ContextMenuItem onSelect={() => hacer({ que: "manifestar", n: 1 })}>
					<EyeOff />
					{t("Manifestar")}
				</ContextMenuItem>
				<ContextMenuSub>
					<ContextMenuSubTrigger>
						<Trash2 />
						{t("Exiliar la de arriba")}
					</ContextMenuSubTrigger>
					<ContextMenuSubContent className="text-xs">
						<ContextMenuItem onSelect={() => op.exiliarDeArriba("normal")}>
							{t("Boca arriba")}
							<Tecla contexto="biblioteca" nombre="Exiliar la de arriba" />
						</ContextMenuItem>
						<ContextMenuItem onSelect={() => op.exiliarDeArriba("bocaAbajo")}>
							{t("Boca abajo (la puedes mirar)")}
							<Tecla contexto="biblioteca" nombre="Exiliar la de arriba boca abajo" />
						</ContextMenuItem>
						<ContextMenuItem onSelect={() => op.exiliarDeArriba("oculta")}>
							{t("Oculta (nadie la ve)")}
						</ContextMenuItem>
						<ContextMenuItem
							onSelect={() =>
								pedirNumero(t("Exiliar cuántas"), 2, (n) => op.exiliarDeArriba("normal", n))
							}
						>
							{t("Varias boca arriba…")}
						</ContextMenuItem>
					</ContextMenuSubContent>
				</ContextMenuSub>
				<ContextMenuSeparator />
				<ContextMenuItem onSelect={() => op.mirar("mirar", "arriba")}>
					<Eye />
					{t("Mirar las de arriba…")}
					<Tecla contexto="biblioteca" nombre="Mirar la de arriba" />
				</ContextMenuItem>
				<ContextMenuItem onSelect={() => op.mirar("mirar", "abajo")}>
					<ArrowDownToLine />
					{t("Mirar las de abajo…")}
				</ContextMenuItem>
				<ContextMenuItem onSelect={() => op.mirar("escrutar")}>
					<Eye />
					{t("Escrutar…")}
					<Tecla contexto="biblioteca" nombre="Escrutar" />
				</ContextMenuItem>
				<ContextMenuItem onSelect={() => op.mirar("vigilar")}>
					<Eye />
					{t("Vigilar…")}
					<Tecla contexto="biblioteca" nombre="Vigilar" />
				</ContextMenuItem>
				<ContextMenuItem onSelect={() => usePantalla.getState().abrir({ tipo: "revelar" })}>
					<Sparkles />
					{t("Revelar y lanzar…")}
				</ContextMenuItem>
				<ContextMenuSeparator />
				<ContextMenuItem onSelect={() => op.buscar()}>
					<Search />
					{t("Buscar una carta")}
					<Tecla contexto="biblioteca" nombre="Buscar una carta" />
				</ContextMenuItem>
				<ContextMenuItem onSelect={() => op.buscar("tierra")}>
					<Trees />
					{t("Buscar una tierra")}
					<Tecla contexto="biblioteca" nombre="Buscar una tierra" />
				</ContextMenuItem>
				<ContextMenuItem onSelect={() => hacer({ que: "revelarArriba", n: 1 })}>
					<Eye />
					{t("Revelar la de arriba")}
				</ContextMenuItem>
				<ContextMenuItem onSelect={() => hacer({ que: "deArribaRevelada", si: !revelada })}>
					{revelada ? <EyeOff /> : <Eye />}
					{revelada
						? t("Dejar de jugar con la de arriba revelada")
						: t("Jugar con la de arriba revelada")}
				</ContextMenuItem>
				<ContextMenuSeparator />
				<ContextMenuItem onSelect={() => op.barajar()}>
					<Shuffle />
					{t("Barajar")}
					<Tecla contexto="biblioteca" nombre="Barajar" />
				</ContextMenuItem>
				<ContextMenuItem onSelect={() => hacer({ que: "rueda", robar: null })}>
					<RefreshCw />
					{t("Rueda (la mano al fondo, barajar y robar las mismas)")}
				</ContextMenuItem>
			</ContenidoContextual>
		</ContextMenu>
	);
}

/** El menú de cementerio, exilio, mando o banquillo: verlo y lo de todas a la vez. */
export function MenuZona({ zona, children }: { zona: Zona; children: ReactNode }) {
	const { t } = useLang();
	const yo = useMesa((s) => s.yo ?? 0);
	const ver = () => usePantalla.getState().abrir({ tipo: "zona", jugador: yo, zona });
	const masiva = zona === "cementerio" || zona === "exilio";
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
												hacer({
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
						<ContextMenuItem onSelect={() => hacer({ que: "barajarTodoDentro", zona })}>
							<Shuffle />
							{t("Barajar todas en la biblioteca")}
						</ContextMenuItem>
						<ContextMenuItem onSelect={() => hacer({ que: "todoAlFondoAlAzar", zona })}>
							<ArrowDownToLine />
							{t("Todas al fondo, en orden al azar")}
						</ContextMenuItem>
						{zona !== "exilio" && (
							<ContextMenuItem onSelect={() => hacer({ que: "exiliarTodo", zona })}>
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
	children,
	posicion,
}: {
	children: ReactNode;
	/** Dónde se abrió el menú, en posición del motor (para la bolita). */
	posicion: () => { x: number; y: number };
}) {
	const { t } = useLang();
	const yo = useMesa((s) => s.yo ?? 0);
	return (
		<ContextMenu>
			<ContextMenuTrigger asChild>{children}</ContextMenuTrigger>
			<ContenidoContextual className="w-60 text-xs">
				<ContextMenuItem onSelect={() => op.enderezarTodo()}>
					<RotateCcw />
					{t("Enderezar todo")}
					<Tecla contexto="mesa" nombre="Enderezar todo" />
				</ContextMenuItem>
				<ContextMenuItem
					onSelect={() => {
						const m = useMesa.getState();
						m.seleccionar(m.zonas[`${yo}:campo`]?.cartas ?? []);
					}}
				>
					<MousePointerClick />
					{t("Elegir todo")}
					<Tecla contexto="mesa" nombre="Elegir todo el campo" />
				</ContextMenuItem>
				<ContextMenuItem onSelect={() => hacer({ que: "organizar" })}>
					<LayoutGrid />
					{t("Organizar el campo")}
				</ContextMenuItem>
				<ContextMenuItem onSelect={() => usePantalla.getState().abrir({ tipo: "fichas" })}>
					<SquareStack />
					{t("Crear ficha…")}
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
											campoDe: yo,
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
