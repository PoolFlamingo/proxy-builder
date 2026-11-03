import { useMemo, useState } from "react";
import { useShallow } from "zustand/react/shallow";
import type { IdCarta } from "@/bindings/IdCarta";
import { DialogoAdaptable } from "@/components/dialogo-adaptable";
import { Icono } from "@/components/icono";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import { Dialog, DialogDescription, DialogTitle } from "@/components/ui/dialog";
import { ScrollArea } from "@/components/ui/scroll-area";
import { cn } from "@/lib/utils";
import { cartasDe, NINGUNA, tableroDe, useMesa } from "@/stores/mesa-store";
import { useRed } from "@/stores/red-store";
import { destino, mover, sitio, zonaDe } from "../acciones";
import { caraVisible, ImagenCarta } from "../imagen-carta";
import { manejadoresLeer } from "../leer-en-grande";
import { MenuCarta } from "../menus/menu-carta";
import * as op from "../operaciones";
import { usePantalla } from "../pantalla";
import { campoAjeno, manoOcultaPorTurno } from "../privacidad";
import { type AgruparMano, agruparMano, COSTE_MAXIMO, type GrupoMano } from "./cuentas-mano";

/** Cómo se agrupó la mano la última vez (se recuerda mientras dure la app). */
let agruparGuardado: AgruparMano = "tipo";

/**
 * **La mano entera del móvil** (plan de la mesa, B5.10): todas las cartas a
 * pantalla completa, agrupadas por tipo o por coste. Tocar una la elige y la
 * lee en grande; mantenerla pulsada (o su «⋯») abre su menú de siempre; abajo,
 * lo que más se hace con la elegida (jugarla, descartarla). Es solo otra
 * forma de ver la mano: lo que se hace con ella lo manda a Rust como siempre.
 */
export function ManoCompleta() {
	const abierta = usePantalla((s) => s.capa === "mano");
	return (
		<Dialog open={abierta} onOpenChange={(o) => !o && usePantalla.getState().setCapa(null)}>
			{abierta && <ContenidoMano />}
		</Dialog>
	);
}

function ContenidoMano() {
	const { t } = useLang();
	// Siempre la mano propia, aunque se mire otro tablero (con «Enfocar al
	// jugador activo» la vista salta al rival en su turno). El espectador no
	// tiene mano: la del tablero que mira.
	const yo = useMesa((s) => s.yo ?? tableroDe(s));
	const ids = useMesa(useShallow((s) => cartasDe(s, zonaDe(yo, "mano"))));
	const cartas = useMesa((s) => s.cartas);
	const enRed = useRed((s) => s.papel !== null);
	const mostrada = usePantalla((s) => s.manoMostrada);
	const privada = useMesa((s) => manoOcultaPorTurno(s, yo, enRed, mostrada));
	const nombre = useMesa((s) => s.jugadores[yo]?.nombre ?? "");
	const propia = useMesa((s) => s.yo === yo);
	const elegida = useMesa((s) => (s.seleccion.length === 1 ? (s.seleccion[0] ?? null) : null));
	const [por, setPor] = useState<AgruparMano>(agruparGuardado);
	const visibles = privada ? NINGUNA : ids;
	const grupos = useMemo(
		() =>
			agruparMano(
				visibles.flatMap((id) => (cartas[id] ? [cartas[id]] : [])),
				por,
			),
		[visibles, cartas, por],
	);
	const enLaMano = elegida !== null && ids.includes(elegida);
	const titulo = (g: GrupoMano) => tituloDeGrupo(g, t);
	const elegirPor = (p: AgruparMano) => {
		agruparGuardado = p;
		setPor(p);
	};

	return (
		<DialogoAdaptable
			className="flex flex-col gap-2.5 sm:max-w-3xl"
			cambiosSinGuardar={false}
			data-tutorial="mesa-mano-completa"
		>
			<DialogTitle className="flex items-center gap-2 pr-10 text-sm">
				<Icono icon="lucide:hand" className="size-4" />
				{ids.length === 1
					? t("Una carta en la mano")
					: t("{n} cartas en la mano", { n: ids.length })}
				<span className="ml-auto flex gap-1" role="tablist">
					{(
						[
							["tipo", t("Por tipo")],
							["coste", t("Por coste")],
						] as const
					).map(([id, texto]) => (
						<button
							key={id}
							type="button"
							role="tab"
							aria-selected={por === id}
							onClick={() => elegirPor(id)}
							className={cn(
								"min-h-11 rounded-lg border px-3 text-xs font-medium",
								por === id ? "border-primary bg-primary text-primary-foreground" : "bg-muted/40",
							)}
						>
							{texto}
						</button>
					))}
				</span>
			</DialogTitle>
			<DialogDescription className="sr-only">
				{t("Toca una carta para elegirla y leerla; mantenla pulsada para ver su menú.")}
			</DialogDescription>
			<ScrollArea className="min-h-0 flex-1" viewportClassName="h-full">
				<div className="flex flex-col gap-3 p-1">
					{privada && (
						<Button
							variant="outline"
							onClick={() => usePantalla.getState().mostrarMano(yo)}
							className="self-start"
						>
							<Icono icon="lucide:eye" className="size-4" />
							{t("Mano de {nombre}: pulsa para verla", { nombre })}
						</Button>
					)}
					{!privada && ids.length === 0 && (
						<p className="p-6 text-center text-xs text-muted-foreground">
							{propia ? t("No tienes cartas en la mano.") : t("No ves la mano de este jugador.")}
						</p>
					)}
					{grupos.map((g) => (
						<section key={`${g.por}:${g.clave}`} className="flex flex-col gap-1.5">
							<h3 className="text-xs font-semibold text-muted-foreground">
								{titulo(g)} · {g.ids.length}
							</h3>
							<div className="flex flex-wrap gap-2.5">
								{g.ids.map((id) => (
									<CartaDeMano key={id} id={id} />
								))}
							</div>
						</section>
					))}
				</div>
			</ScrollArea>
			{enLaMano && elegida !== null && <AccionesDeLaElegida id={elegida} />}
		</DialogoAdaptable>
	);
}

function tituloDeGrupo(
	g: GrupoMano,
	t: (s: string, v?: Record<string, string | number>) => string,
) {
	if (g.por === "tipo") {
		switch (g.clave) {
			case "tierra":
				return t("Tierras");
			case "criatura":
				return t("Criaturas");
			case "planeswalker":
				return t("Planeswalkers");
			case "instantaneo":
				return t("Instantáneos");
			case "conjuro":
				return t("Conjuros");
			case "artefacto":
				return t("Artefactos");
			case "encantamiento":
				return t("Encantamientos");
			case "batalla":
				return t("Batallas");
			default:
				return t("Otras");
		}
	}
	if (g.clave === "tierras") return t("Tierras");
	return g.clave >= COSTE_MAXIMO
		? t("Coste {n} o más", { n: COSTE_MAXIMO })
		: t("Coste {n}", { n: g.clave });
}

/** Una carta de la mano: un toque la elige y la lee; mantenerla abre su menú. */
function CartaDeMano({ id }: { id: IdCarta }) {
	const c = useMesa((s) => s.cartas[id]);
	const elegida = useMesa((s) => s.seleccion.length === 1 && s.seleccion[0] === id);
	if (!c) return null;
	return (
		<MenuCarta c={c}>
			<div
				data-carta-mano={id}
				{...manejadoresLeer(id)}
				onClick={() => useMesa.getState().seleccionar([id])}
				className={cn(
					"relative h-40 w-28 shrink-0 overflow-hidden rounded-[5%] shadow-lg",
					elegida && "ring-3 ring-primary ring-offset-2 ring-offset-background",
				)}
			>
				<ImagenCarta imagen={caraVisible(c)} ancho={244} nombre={c.origen?.nombre} />
			</div>
		</MenuCarta>
	);
}

/** Lo que más se hace con la carta elegida de la mano, a mano. */
function AccionesDeLaElegida({ id }: { id: IdCarta }) {
	const { t } = useLang();
	const c = useMesa((s) => s.cartas[id]);
	const sinPermiso = useMesa((s) => (c ? campoAjeno(s, c) : true));
	const mirando = useRed((s) => s.papel === "espectador");
	if (!c || mirando) return null;
	const abrirMenu = () => {
		const el = document.querySelector<HTMLElement>(`[data-carta-mano="${id}"]`);
		const r = el?.getBoundingClientRect();
		el?.dispatchEvent(
			new MouseEvent("contextmenu", {
				bubbles: true,
				cancelable: true,
				clientX: (r?.left ?? 0) + (r?.width ?? 0) / 2,
				clientY: (r?.top ?? 0) + (r?.height ?? 0) / 2,
			}),
		);
	};
	return (
		<div className="flex shrink-0 items-center justify-between gap-2">
			<span className="min-w-0 flex-1 truncate text-xs font-medium">
				{c.origen?.nombre ?? t("Carta")}
			</span>
			<div className="flex shrink-0 gap-1.5">
				<Button
					disabled={sinPermiso}
					onClick={() => void mover([id], destino("campo", sitio.arriba, c.dueno))}
				>
					<Icono icon="lucide:play" className="size-4" />
					{t("Jugar")}
				</Button>
				<Button
					variant="outline"
					disabled={sinPermiso}
					onClick={() => void op.aZona(id, "cementerio")}
				>
					<Icono icon="game-icons:tombstone" className="size-4" />
					{t("Al cementerio")}
				</Button>
				<Button
					variant="outline"
					size="icon"
					aria-label={t("Menú de la carta")}
					onClick={abrirMenu}
				>
					<Icono icon="lucide:ellipsis" className="size-4" />
				</Button>
			</div>
		</div>
	);
}
