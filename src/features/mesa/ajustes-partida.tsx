import { useDraggable, useDroppable } from "@dnd-kit/core";
import type { Formato } from "@/bindings/Formato";
import { Fila, FilaInterruptor, Selector } from "@/components/campos";
import { Icono } from "@/components/icono";
import { useLang } from "@/components/language-provider";
import { cn } from "@/lib/utils";
import { COLORES_EQUIPO } from "./colores";
import {
	type AjustesLocal,
	type AlMejorDe,
	asientoArchienemigo,
	conBanquillo,
	MAX_EQUIPOS,
	type ModoEquiposLocal,
	problemaDeEquipos,
} from "./config-local";

/**
 * **Los ajustes de una partida con varios jugadores** que comparten la
 * partida local y la sala de espera en red: cuántas partidas, cómo se juega
 * por equipos y quién empieza. Los asientos (con su nombre y su deck) los
 * pone cada diálogo, que no tienen lo mismo.
 */

/** Cómo se llama un asiento en las listas de elegir (nombre, o «Jugador n»). */
export type NombreDeAsiento = (i: number) => string;

export function AjustesDePartida({
	ajustes,
	onChange,
	formato,
	asientos,
	nombreDeAsiento,
}: {
	ajustes: AjustesLocal;
	onChange: (c: Partial<AjustesLocal>) => void;
	formato: Formato;
	/** Cuántos asientos hay. */
	asientos: number;
	nombreDeAsiento: NombreDeAsiento;
}) {
	const { t } = useLang();
	const porEquipos = ajustes.modo === "equipos";
	return (
		<>
			{conBanquillo(formato) && (
				<Fila etiqueta={t("Partidas")}>
					<Selector<string>
						valor={String(ajustes.alMejorDe)}
						onChange={(v) => onChange({ alMejorDe: Number(v) as AlMejorDe })}
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
					onChange={(modo) => onChange({ modo })}
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
						valor={String(asientoArchienemigo(ajustes, asientos))}
						onChange={(v) => onChange({ archienemigo: Number(v) })}
						opciones={Array.from({ length: asientos }, (_, i) => ({
							valor: String(i),
							etiqueta: nombreDeAsiento(i),
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
						onChange={(fuegoAmigo) => onChange({ fuegoAmigo })}
					/>
					<FilaInterruptor
						etiqueta={t("Vida compartida")}
						ayuda={t("Cada equipo tiene una sola vida, la de todos sus jugadores.")}
						valor={ajustes.vidaCompartida}
						onChange={(vidaCompartida) => onChange({ vidaCompartida })}
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
					onChange={(manoDelCompanero) => onChange({ manoDelCompanero })}
				/>
			)}
			<FilaInterruptor
				etiqueta={t("Planechase")}
				ayuda={t(
					"Un mazo común con los planos de los decks, el plano activo en la mesa y el dado planar.",
				)}
				valor={ajustes.planechase}
				onChange={(planechase) => onChange({ planechase })}
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
		</>
	);
}

/** «Empieza»: al azar o un asiento. */
export function FilaEmpieza({
	ajustes,
	onChange,
	asientos,
	nombreDeAsiento,
}: {
	ajustes: AjustesLocal;
	onChange: (c: Partial<AjustesLocal>) => void;
	asientos: number;
	nombreDeAsiento: NombreDeAsiento;
}) {
	const { t } = useLang();
	return (
		<Fila etiqueta={t("Empieza")}>
			<Selector<string>
				valor={ajustes.primero === null ? "azar" : String(ajustes.primero)}
				onChange={(v) => onChange({ primero: v === "azar" ? null : Number(v) })}
				opciones={[
					{ valor: "azar", etiqueta: t("Al azar") },
					...Array.from({ length: asientos }, (_, i) => ({
						valor: String(i),
						etiqueta: nombreDeAsiento(i),
					})),
				]}
			/>
		</Fila>
	);
}

/** Lo que impide empezar con estos equipos, dicho en una frase (o nada). */
export function AvisoDeEquipos({
	asientos,
	modo,
}: {
	asientos: { equipo: number | null }[];
	modo: ModoEquiposLocal;
}) {
	const { t } = useLang();
	const problema = problemaDeEquipos(asientos, modo);
	if (!problema) return null;
	return (
		<p className="text-xs text-amber-500">
			{problema === "dosCabezasPares"
				? t("Dos Cabezas necesita 4 jugadores o más, en número par.")
				: t("Hace falta un equipo con al menos dos jugadores.")}
		</p>
	);
}

/** El equipo de un asiento, con su color. */
export function SelectorDeEquipo({
	equipo,
	onChange,
}: {
	equipo: number | null;
	onChange: (e: number | null) => void;
}) {
	const { t } = useLang();
	return (
		<>
			<Selector<string>
				valor={equipo === null ? "-" : String(equipo)}
				onChange={(v) => onChange(v === "-" ? null : Number(v))}
				ancho="w-32"
				ariaLabel={t("Equipo")}
				opciones={[
					{ valor: "-", etiqueta: t("Sin equipo") },
					...COLORES_EQUIPO.slice(0, MAX_EQUIPOS).map((_, e) => ({
						valor: String(e),
						etiqueta: t("Equipo {n}", { n: e + 1 }),
					})),
				]}
			/>
			{equipo !== null && (
				<span
					className="size-3 shrink-0 rounded-full"
					style={{ background: COLORES_EQUIPO[equipo] }}
				/>
			)}
		</>
	);
}

/**
 * Una fila de asiento que se arrastra por su asa y acepta que otra caiga
 * encima: la que se suelta ocupa su sitio y las demás se corren.
 */
export function FilaOrdenable({
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
				<Icono icon="lucide:grip-vertical" className="size-3.5" />
			</button>
			{children}
		</div>
	);
}
