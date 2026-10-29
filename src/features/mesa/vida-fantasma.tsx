import { useMemo, useRef, useState } from "react";
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
import { WithTooltip } from "@/components/ui/with-tooltip";
import { cn } from "@/lib/utils";
import { NINGUNA, useMesa } from "@/stores/mesa-store";
import { useUi } from "@/stores/ui-store";
import { hacer } from "./acciones";
import { comandantesDe, deltaSeguro } from "./dano-comandante";
import { elegirFantasma } from "./objetivo";
import { pedirNumero, usePantalla } from "./pantalla";

/** Lo que se espera a que paren los clics de vida para mandarlos juntos, en ms. */
const JUNTAR_MS = 700;

/** Atributo del marcador del fantasma: ahí apuntan las flechas que le llegan (`flechas.tsx`). */
export const ATRIBUTO_FANTASMA = "data-vida-fantasma";

/**
 * **La vida del rival imaginado del solitario**, como la de un jugador más:
 * − y + (o la rueda) se juntan en un solo evento, y su menú tiene daño,
 * ganar vidas, veneno y daño de comandante. Con una flecha en marcha, pulsarlo
 * lo hace el objetivo (`Extremo::Fantasma`). Se pone en rojo cuando estaría
 * fuera (vida, veneno o un comandante); quién gana lo decide la persona.
 */
export function VidaFantasma() {
	const { t } = useLang();
	const f = useMesa((s) => s.general?.fantasma ?? null);
	const ver = useUi((s) => s.mesa.fantasma);
	const apuntando = usePantalla((s) => s.modo?.tipo === "flecha");
	const limiteVeneno = useMesa((s) => s.opciones?.limiteVeneno ?? 10);
	const limiteComandante = useMesa((s) => s.opciones?.limiteComandante ?? 21);
	const flechas = useMesa((s) => s.general?.flechas ?? NINGUNA);
	const cartas = useMesa((s) => s.cartas);
	const nombres = useMemo(
		() => Object.fromEntries(comandantesDe(cartas).map((c) => [c.id, c.nombre])),
		[cartas],
	);
	const [pendiente, setPendiente] = useState(0);
	const temporizador = useRef<number | null>(null);
	const acumulado = useRef(0);
	if (!f?.visible || !ver) return null;

	const sumar = (d: number) => {
		acumulado.current += d;
		setPendiente(acumulado.current);
		if (temporizador.current) window.clearTimeout(temporizador.current);
		temporizador.current = window.setTimeout(() => {
			const n = acumulado.current;
			acumulado.current = 0;
			temporizador.current = null;
			setPendiente(0);
			if (n !== 0) void hacer({ que: "vidaFantasma", delta: n });
		}, JUNTAR_MS);
	};

	const veneno = f.veneno ?? 0;
	const comandantes = Object.entries(f.danoComandante).filter(([, n]) => n > 0);
	const fuera =
		f.vida <= 0 || veneno >= limiteVeneno || comandantes.some(([, n]) => n >= limiteComandante);
	const atacado = flechas.filter((x) => x.tipo === "ataque" && x.a.tipo === "fantasma").length;
	return (
		<MenuFantasma>
			<div
				{...{ [ATRIBUTO_FANTASMA]: "" }}
				className={cn(
					"absolute flex items-center gap-1 rounded-full border border-white/15 bg-black/60 px-2 py-1 text-white shadow-lg backdrop-blur-md",
					apuntando && "cursor-crosshair ring-2 ring-primary",
					!apuntando && fuera && "ring-2 ring-rose-500",
				)}
				style={{ right: 12, top: 56, zIndex: 5500 }}
				// Con una flecha en marcha, el clic es para apuntarle (no para − ni +).
				onClickCapture={(e) => {
					if (elegirFantasma(t)) {
						e.preventDefault();
						e.stopPropagation();
					}
				}}
				onWheel={(e) => sumar(e.deltaY < 0 ? 1 : -1)}
				data-tutorial="mesa-fantasma"
			>
				<Icono icon="game-icons:ghost" className="size-4 opacity-80" />
				<span className="text-[10px] opacity-70">{t("Fantasma")}</span>
				<button type="button" className="px-1.5 text-lg leading-none" onClick={() => sumar(-1)}>
					−
				</button>
				<div className="flex min-w-9 flex-col items-center leading-none">
					<span className="text-lg font-bold tabular-nums">{f.vida + pendiente}</span>
					{pendiente !== 0 && (
						<span className="text-[10px] text-white/70 tabular-nums">
							{f.vida} {pendiente > 0 ? "+" : "−"} {Math.abs(pendiente)}
						</span>
					)}
				</div>
				<button type="button" className="px-1.5 text-lg leading-none" onClick={() => sumar(1)}>
					+
				</button>
				{veneno > 0 && (
					<WithTooltip content={t("Veneno")}>
						<span
							className={cn(
								"ml-0.5 flex items-center gap-0.5 text-xs tabular-nums",
								veneno >= limiteVeneno && "text-rose-400",
							)}
						>
							<Icono icon="game-icons:poison-bottle" className="size-3.5" />
							{veneno}
						</span>
					</WithTooltip>
				)}
				{comandantes.map(([id, n]) => (
					<WithTooltip
						key={id}
						content={t("Daño de {nombre}", { nombre: nombres[Number(id)] ?? "" })}
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
					</WithTooltip>
				))}
				{atacado > 0 && (
					<WithTooltip content={t("Atacantes")}>
						<span className="ml-0.5 flex items-center gap-0.5 text-xs text-rose-300 tabular-nums">
							<Icono icon="game-icons:crossed-swords" className="size-3.5" />
							{atacado}
						</span>
					</WithTooltip>
				)}
			</div>
		</MenuFantasma>
	);
}

function MenuFantasma({ children }: { children: React.ReactNode }) {
	const { t } = useLang();
	const f = useMesa((s) => s.general?.fantasma ?? null);
	const cartas = useMesa((s) => s.cartas);
	const comandantes = useMemo(() => comandantesDe(cartas), [cartas]);
	if (!f) return <>{children}</>;
	const veneno = f.veneno ?? 0;
	return (
		<ContextMenu>
			<ContextMenuTrigger asChild>{children}</ContextMenuTrigger>
			<ContenidoContextual className="w-64 text-xs">
				<ContextMenuItem
					onSelect={() =>
						pedirNumero(t("Daño"), 1, (n) => hacer({ que: "vidaFantasma", delta: -Math.abs(n) }))
					}
				>
					<Icono icon="game-icons:flame" />
					{t("Daño…")}
				</ContextMenuItem>
				<ContextMenuItem
					onSelect={() =>
						pedirNumero(t("Ganar vidas"), 1, (n) =>
							hacer({ que: "vidaFantasma", delta: Math.abs(n) }),
						)
					}
				>
					<Icono icon="game-icons:hearts" />
					{t("Ganar vidas…")}
				</ContextMenuItem>
				<ContextMenuSeparator />
				<ContextMenuItem onSelect={() => hacer({ que: "venenoFantasma", delta: 1 })}>
					<Icono icon="game-icons:poison-bottle" />
					{t("Un veneno más")}
				</ContextMenuItem>
				<ContextMenuItem
					disabled={veneno === 0}
					onSelect={() => hacer({ que: "venenoFantasma", delta: -1 })}
				>
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
										pedirNumero(t("Daño de {nombre}", { nombre: c.nombre }), 1, (n) =>
											hacer({
												que: "danoComandanteFantasma",
												comandante: c.id,
												delta: deltaSeguro(f.danoComandante[c.id] ?? 0, n),
											}),
										)
									}
								>
									{c.nombre} · {f.danoComandante[c.id] ?? 0}
								</ContextMenuItem>
							))}
						</ContextMenuSubContent>
					</ContextMenuSub>
				)}
			</ContenidoContextual>
		</ContextMenu>
	);
}
