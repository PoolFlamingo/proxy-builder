import { useMemo, useRef, useState } from "react";
import type { Accion } from "@/bindings/Accion";
import type { VidaFantasma as VidaFantasmaBinding } from "@/bindings/VidaFantasma";
import { AyudaAlTocar } from "@/components/ayuda-al-tocar";
import { ContenidoContextual } from "@/components/contenido-contextual";
import { Icono } from "@/components/icono";
import { useLang } from "@/components/language-provider";
import {
	ContextMenu,
	ContextMenuItem,
	ContextMenuSeparator,
	ContextMenuSub,
	ContextMenuSubContent,
	ContextMenuSubTrigger,
	ContextMenuTrigger,
} from "@/components/ui/context-menu";

import { cn } from "@/lib/utils";
import { NINGUNA, useMesa } from "@/stores/mesa-store";
import { useUi } from "@/stores/ui-store";
import { hacer } from "./acciones";
import { comandantesDe, deltaSeguro } from "./dano-comandante";
import { useMesaMovil } from "./formato-mesa";
import { nombreFantasma } from "./movil/cuentas-vida";
import { elegirFantasma } from "./objetivo";
import { pedirNumero, usePantalla } from "./pantalla";
import { MAS_MENOS_MOVIL, MAS_MENOS_TACTIL } from "./vida-jugador";

/** Lo que se espera a que paren los clics de vida para mandarlos juntos, en ms. */
const JUNTAR_MS = 700;

/** Atributo del marcador del fantasma: ahí apuntan las flechas que le llegan (`flechas.tsx`). */
export const ATRIBUTO_FANTASMA = "data-vida-fantasma";

/** Cuántos rivales imaginados puede haber (`MAX_FANTASMAS` de `mesa::modelo`). */
const MAX_FANTASMAS = 9;

/**
 * **Los rivales imaginados del solitario**, uno debajo de otro y cada uno con su
 * vida; si no caben, sigue otra columna a la izquierda. Solo son marcadores:
 * el sparring no los mira, así que tener más no trae más reacciones.
 */
export function VidaFantasma() {
	const f = useMesa((s) => s.general?.fantasma ?? null);
	const otros = useMesa((s) => s.general?.fantasmasExtra ?? NINGUNA);
	const ver = useUi((s) => s.mesa.fantasma);
	// En el móvil baja un poco (arriba a la derecha está «Pasar turno») y es más pequeña.
	const movil = useMesaMovil();
	if (!f?.visible || !ver) return null;
	const arriba = movil ? 52 : 56;
	return (
		<div
			// Con `wrap-reverse` el eje cruzado va al revés: `items-start` los pega a la derecha.
			className="pointer-events-none absolute flex flex-col flex-wrap-reverse content-start items-start gap-1"
			style={{
				right: movil ? 8 : 12,
				top: arriba,
				zIndex: 5500,
				// Sin llegar a mi marcador de vida, abajo a la derecha.
				maxHeight: `calc(100% - ${arriba + ALTO_ABAJO}px)`,
			}}
		>
			<Marcador f={f} otro={null} otros={otros.length} />
			{otros.map((x, i) => (
				// Se señalan por su sitio: al quitar uno, los de detrás pasan a ocupar el suyo.
				<Marcador key={i} f={x} otro={i} otros={otros.length} />
			))}
		</div>
	);
}

/** Lo que dejan libre abajo: mi vida, las zonas y la mano. */
const ALTO_ABAJO = 165;

/**
 * La vida de un rival imaginado, como la de un jugador más: − y + (o la rueda)
 * se juntan en un solo evento, y su menú tiene daño, ganar vidas, veneno y daño
 * de comandante. Con una flecha en marcha, pulsarlo lo hace el objetivo. Se pone
 * en rojo cuando estaría fuera (vida, veneno o un comandante); quién gana lo
 * decide la persona.
 */
function Marcador({
	f,
	otro,
	otros,
}: {
	f: VidaFantasmaBinding;
	otro: number | null;
	otros: number;
}) {
	const { t } = useLang();
	const apuntando = usePantalla((s) => s.modo?.tipo === "flecha");
	const limiteVeneno = useMesa((s) => s.opciones?.limiteVeneno ?? 10);
	const limiteComandante = useMesa((s) => s.opciones?.limiteComandante ?? 21);
	const flechas = useMesa((s) => s.general?.flechas ?? NINGUNA);
	const cartas = useMesa((s) => s.cartas);
	const nombres = useMemo(
		() => Object.fromEntries(comandantesDe(cartas).map((c) => [c.id, c.nombre])),
		[cartas],
	);
	const movil = useMesaMovil();
	// En el móvil, con varios, una ficha pequeña: si no, taparían el campo. Tocarla abre
	// la pantalla de vida, donde está cada uno con sus cuentas (y mantenerla, su menú).
	const compacto = movil && otros > 0;
	const [pendiente, setPendiente] = useState(0);
	const temporizador = useRef<number | null>(null);
	const acumulado = useRef(0);

	const sumar = (d: number) => {
		acumulado.current += d;
		setPendiente(acumulado.current);
		if (temporizador.current) window.clearTimeout(temporizador.current);
		temporizador.current = window.setTimeout(() => {
			const n = acumulado.current;
			acumulado.current = 0;
			temporizador.current = null;
			setPendiente(0);
			if (n !== 0) void hacer(accionVida(otro, n));
		}, JUNTAR_MS);
	};

	const veneno = f.veneno ?? 0;
	const comandantes = Object.entries(f.danoComandante).filter(([, n]) => n > 0);
	const fuera =
		f.vida <= 0 || veneno >= limiteVeneno || comandantes.some(([, n]) => n >= limiteComandante);
	const atacado = flechas.filter(
		(x) =>
			x.tipo === "ataque" &&
			(otro === null ? x.a.tipo === "fantasma" : x.a.tipo === "otroFantasma" && x.a.id === otro),
	).length;
	return (
		<MenuFantasma f={f} otro={otro} otros={otros}>
			<div
				{...{ [ATRIBUTO_FANTASMA]: otro === null ? "" : String(otro) }}
				className={cn(
					"pointer-events-auto flex items-center gap-1 rounded-full border border-white/15 bg-black/60 text-white shadow-lg backdrop-blur-md",
					compacto ? "min-h-8 px-2 py-0.5" : movil ? "px-1.5 py-0" : "px-2 py-1",
					apuntando && "cursor-crosshair ring-2 ring-primary",
					!apuntando && fuera && "ring-2 ring-rose-500",
				)}
				// Con una flecha en marcha, el clic es para apuntarle (no para − ni +).
				onClickCapture={(e) => {
					if (elegirFantasma(t, otro)) {
						e.preventDefault();
						e.stopPropagation();
					}
				}}
				onClick={compacto ? () => usePantalla.getState().setCapa("vida") : undefined}
				onWheel={(e) => sumar(e.deltaY < 0 ? 1 : -1)}
				data-tutorial={otro === null ? "mesa-fantasma" : undefined}
			>
				<Icono icon="game-icons:ghost" className="size-4 opacity-80" />
				{compacto ? (
					<span className="sr-only">{nombreFantasma(t, otro, otros)}</span>
				) : (
					<span className="text-[10px] opacity-70">{nombreFantasma(t, otro, otros)}</span>
				)}
				{compacto && (
					<span aria-hidden="true" className="text-[10px] opacity-70 tabular-nums">
						{otro === null ? 1 : otro + 2}
					</span>
				)}
				{!compacto && (
					<button
						type="button"
						className={cn(
							"px-1.5 text-lg leading-none",
							movil ? MAS_MENOS_MOVIL : MAS_MENOS_TACTIL,
						)}
						onClick={() => sumar(-1)}
					>
						−
					</button>
				)}
				<div className={cn("flex flex-col items-center leading-none", !compacto && "min-w-9")}>
					<span className="text-lg font-bold tabular-nums">{f.vida + pendiente}</span>
					{pendiente !== 0 && (
						<span className="text-[10px] text-white/70 tabular-nums">
							{f.vida} {pendiente > 0 ? "+" : "−"} {Math.abs(pendiente)}
						</span>
					)}
				</div>
				{!compacto && (
					<button
						type="button"
						className={cn(
							"px-1.5 text-lg leading-none",
							movil ? MAS_MENOS_MOVIL : MAS_MENOS_TACTIL,
						)}
						onClick={() => sumar(1)}
					>
						+
					</button>
				)}
				{veneno > 0 && (
					<AyudaAlTocar ayuda={t("Veneno")}>
						<span
							className={cn(
								"ml-0.5 flex items-center gap-0.5 text-xs tabular-nums",
								veneno >= limiteVeneno && "text-rose-400",
							)}
						>
							<Icono icon="game-icons:poison-bottle" className="size-3.5" />
							{veneno}
						</span>
					</AyudaAlTocar>
				)}
				{comandantes.map(([id, n]) => (
					<AyudaAlTocar
						key={id}
						ayuda={t("Daño de {nombre}", { nombre: nombres[Number(id)] ?? "" })}
					>
						<span
							className={cn(
								"ml-0.5 flex items-center gap-0.5 text-xs tabular-nums",
								n >= limiteComandante && "text-rose-400",
							)}
						>
							<Icono icon="game-icons:crowned-skull" className="size-3.5" />
							{n}
						</span>
					</AyudaAlTocar>
				))}
				{atacado > 0 && (
					<AyudaAlTocar ayuda={t("Atacantes")}>
						<span className="ml-0.5 flex items-center gap-0.5 text-xs text-rose-300 tabular-nums">
							<Icono icon="game-icons:crossed-swords" className="size-3.5" />
							{atacado}
						</span>
					</AyudaAlTocar>
				)}
			</div>
		</MenuFantasma>
	);
}

/** Las acciones de sumar a un fantasma: el primero y los demás van por variantes distintas. */
function accionVida(otro: number | null, delta: number): Accion {
	return otro === null
		? { que: "vidaFantasma", delta }
		: { que: "vidaOtroFantasma", fantasma: otro, delta };
}

function accionVeneno(otro: number | null, delta: number): Accion {
	return otro === null
		? { que: "venenoFantasma", delta }
		: { que: "venenoOtroFantasma", fantasma: otro, delta };
}

function MenuFantasma({
	f,
	otro,
	otros,
	children,
}: {
	f: VidaFantasmaBinding;
	otro: number | null;
	otros: number;
	children: React.ReactNode;
}) {
	const { t } = useLang();
	const cartas = useMesa((s) => s.cartas);
	const comandantes = useMemo(() => comandantesDe(cartas), [cartas]);
	const veneno = f.veneno ?? 0;
	return (
		<ContextMenu>
			<ContextMenuTrigger asChild>{children}</ContextMenuTrigger>
			<ContenidoContextual className="w-64 text-xs">
				<ContextMenuItem
					onSelect={() => pedirNumero(t("Daño"), 1, (n) => hacer(accionVida(otro, -Math.abs(n))))}
				>
					<Icono icon="game-icons:flame" />
					{t("Daño…")}
				</ContextMenuItem>
				<ContextMenuItem
					onSelect={() =>
						pedirNumero(t("Ganar vidas"), 1, (n) => hacer(accionVida(otro, Math.abs(n))))
					}
				>
					<Icono icon="game-icons:hearts" />
					{t("Ganar vidas…")}
				</ContextMenuItem>
				<ContextMenuSeparator />
				<ContextMenuItem onSelect={() => hacer(accionVeneno(otro, 1))}>
					<Icono icon="game-icons:poison-bottle" />
					{t("Un veneno más")}
				</ContextMenuItem>
				<ContextMenuItem disabled={veneno === 0} onSelect={() => hacer(accionVeneno(otro, -1))}>
					<Icono icon="game-icons:poison-bottle" />
					{t("Un veneno menos")}
				</ContextMenuItem>
				{comandantes.length > 0 && (
					<ContextMenuSub>
						<ContextMenuSubTrigger>
							<Icono icon="game-icons:crown" />
							{t("Daño de comandante")}
						</ContextMenuSubTrigger>
						<ContextMenuSubContent className="text-xs">
							{comandantes.map((c) => (
								<ContextMenuItem
									key={c.id}
									onSelect={() =>
										pedirNumero(t("Daño de {nombre}", { nombre: c.nombre }), 1, (n) => {
											const delta = deltaSeguro(f.danoComandante[c.id] ?? 0, n);
											return hacer(
												otro === null
													? { que: "danoComandanteFantasma", comandante: c.id, delta }
													: {
															que: "danoComandanteOtroFantasma",
															fantasma: otro,
															comandante: c.id,
															delta,
														},
											);
										})
									}
								>
									{c.nombre} · {f.danoComandante[c.id] ?? 0}
								</ContextMenuItem>
							))}
						</ContextMenuSubContent>
					</ContextMenuSub>
				)}
				<ContextMenuSeparator />
				<ContextMenuItem
					disabled={otros + 1 >= MAX_FANTASMAS}
					onSelect={() => hacer({ que: "anadirFantasma" })}
				>
					<Icono icon="game-icons:ghost" />
					{t("Añadir otro fantasma")}
				</ContextMenuItem>
				{otro !== null && (
					<ContextMenuItem onSelect={() => hacer({ que: "quitarFantasma", fantasma: otro })}>
						<Icono icon="lucide:x" />
						{t("Quitar este fantasma")}
					</ContextMenuItem>
				)}
			</ContenidoContextual>
		</ContextMenu>
	);
}
