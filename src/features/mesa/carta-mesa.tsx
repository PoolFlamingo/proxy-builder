import { Eye } from "lucide-react";
import { memo, useRef } from "react";
import type { IdCarta } from "@/bindings/IdCarta";
import type { VistaCarta } from "@/bindings/VistaCarta";
import { cn } from "@/lib/utils";
import { useMesa } from "@/stores/mesa-store";
import { useUi } from "@/stores/ui-store";
import { enVuelo, origenDe, useVivo } from "@/stores/vivo-store";
import { COLORES } from "./colores";
import { useFlip } from "./flip";
import { useSinAnimaciones } from "./fps";
import { caraVisible, ImagenCarta } from "./imagen-carta";
import { Insignias } from "./insignias";
import { MenuCarta } from "./menus/menu-carta";
import { usePantalla } from "./pantalla";

export type AlPulsar = (e: React.PointerEvent, c: VistaCarta) => void;

/**
 * **Una carta en la mesa**, en su sitio (`x`, `y`: el centro en píxeles del
 * tablero). La imagen va al ancho que diga Ajustes > Mesa; girada, boca
 * abajo (con el dorso), con sus insignias y su menú. Se repinta solo si
 * cambia ella (`memo` y el selector de su id en el store).
 */
export const CartaMesa = memo(function CartaMesa({
	id,
	x,
	y,
	w,
	h,
	giro = 0,
	z,
	alPulsar,
	alDobleClic,
	insignias = true,
	levantar = false,
}: {
	id: IdCarta;
	x: number;
	y: number;
	w: number;
	h: number;
	/** Giro extra (el abanico de la mano), en grados. */
	giro?: number;
	z?: number;
	alPulsar: AlPulsar;
	alDobleClic?: (c: VistaCarta) => void;
	insignias?: boolean;
	/** Se levanta al pasar el ratón (la mano). */
	levantar?: boolean;
}) {
	const c = useMesa((s) => s.cartas[id]);
	const yo = useMesa((s) => s.yo);
	const elegida = useMesa((s) => s.seleccion.includes(id));
	const calidad = useUi((s) => s.mesa.calidad);
	const reducir = useSinAnimaciones();
	const sombras = useUi((s) => s.mesa.sombras);
	// Mientras se lleva en la mano. Ya soltada, la oculta `enVueloAjeno` hasta que llega.
	const arrastrando = usePantalla(
		(s) => !s.arrastre?.soltado && (s.arrastre?.ids.includes(id) ?? false),
	);
	// Va con una arrastrada (adjunta o amontonada): se ve en el arrastre hasta que
	// su carta llega, y entonces se coloca junto a ella sin deslizarse.
	const acompana = usePantalla((s) => s.arrastre?.con.includes(id) ?? false);
	// Acaba de soltarse (aquí o por otro): aterriza sin deslizarse desde donde estaba.
	const aterriza = useVivo((s) => s.soltadas[id] !== undefined) || acompana;
	// Otro la arrastra (o se acaba de soltar y aún no llega su movimiento):
	// no se pinta en su sitio de antes, se ve el arrastre y nada más.
	const enVueloAjeno = useVivo((s) =>
		enVuelo(s, id, () => {
			const c = useMesa.getState().cartas[id];
			return origenDe(c ?? { zona: null, pos: null });
		}),
	);
	const ref = useRef<HTMLDivElement>(null);
	useFlip(id, ref, !reducir, `${c?.zona.tipo}${JSON.stringify(c?.zona)}${x}|${y}`);
	if (!c) return null;
	const ancho = calidad === 488 || w > 200 ? 488 : w > 120 ? 244 : calidad;
	const conocida = c.origen !== null && !c.bocaAbajo;
	return (
		<MenuCarta c={c}>
			<div
				ref={ref}
				data-carta={id}
				onPointerDown={(e) => alPulsar(e, c)}
				onDoubleClick={() => alDobleClic?.(c)}
				onPointerEnter={() => useMesa.getState().setSobre(id)}
				onPointerLeave={() => {
					if (useMesa.getState().sobre === id) useMesa.getState().setSobre(null);
				}}
				className={cn(
					"group absolute top-0 left-0 cursor-grab touch-none",
					!reducir && !aterriza && "transition-[transform] duration-150 ease-out",
					// Arrastrada (por mí o por otro): no queda su sitio de antes.
					(arrastrando || acompana || enVueloAjeno) && "pointer-events-none opacity-0",
				)}
				style={{
					width: w,
					height: h,
					zIndex: z ?? c.z,
					transform: `translate(${x - w / 2}px, ${y - h / 2}px) rotate(${(c.girada ? 90 : 0) + giro}deg)`,
				}}
			>
				<div
					className={cn(
						"relative size-full rounded-[5%] transition-transform",
						levantar && "group-hover:-translate-y-[35%] group-hover:scale-110",
						sombras && "shadow-[0_3px_10px_rgba(0,0,0,0.55)]",
						c.faseFuera && "opacity-40",
						elegida && "ring-2 ring-primary ring-offset-1 ring-offset-black/40",
					)}
					style={
						c.resaltada !== null
							? {
									boxShadow: `0 0 0 3px ${COLORES[c.resaltada] ?? "#fff"}, 0 0 14px ${COLORES[c.resaltada] ?? "#fff"}`,
								}
							: undefined
					}
				>
					<ImagenCarta
						imagen={caraVisible(c)}
						ancho={ancho}
						nombre={conocida ? c.origen?.nombre : undefined}
					/>
					{insignias &&
						(c.girada ? (
							// Girada, las insignias van derechas sobre la caja apaisada
							// que se ve, para que se lean.
							<div
								className="pointer-events-none absolute top-1/2 left-1/2"
								style={{
									width: h,
									height: w,
									transform: "translate(-50%, -50%) rotate(-90deg)",
								}}
							>
								<Insignias c={c} yo={yo} />
							</div>
						) : (
							<Insignias c={c} yo={yo} />
						))}
					{c.revelada && c.zona.tipo === "de" && c.zona.zona === "mano" && (
						<span className="pointer-events-none absolute top-1 right-1 rounded-full bg-amber-400 p-0.5 text-black shadow">
							<Eye className="size-3" />
						</span>
					)}
					{c.bocaAbajo && c.origen && (
						<span className="pointer-events-none absolute right-1 bottom-1 rounded bg-black/70 p-0.5 text-white">
							<Eye className="size-2.5" />
						</span>
					)}
				</div>
			</div>
		</MenuCarta>
	);
});
