import { useEffect } from "react";
import { useShallow } from "zustand/react/shallow";
import { useLang } from "@/components/language-provider";
import type { AnchoMesa } from "@/lib/image-url";
import { tableroDe, useMesa } from "@/stores/mesa-store";
import { useUi } from "@/stores/ui-store";
import { cartasDeFantasma, type Soltada, soltadaSigue, useVivo } from "@/stores/vivo-store";
import { type Disposicion, posAPx, type Rect } from "../disposicion";
import { caraVisible, DorsoGenerico, ImagenCarta } from "../imagen-carta";

/**
 * Quita lo que ya caducó (un cursor quieto se queda 5 s, una carta soltada
 * espera 0,8 s a que llegue su movimiento). Va una vez en la mesa, no en cada
 * tablero: las miniaturas también dependen de ello.
 */
export function usePurgarVivo() {
	useEffect(() => {
		const i = window.setInterval(() => useVivo.getState().purgar(performance.now()), 500);
		return () => window.clearInterval(i);
	}, []);
}

/**
 * **Lo que mueven los demás** (E7), sobre el campo que se está mirando: el
 * cursor de cada jugador con su color y su nombre, la carta que arrastra
 * (si no es visible para mí, su dorso) y los aros de aviso. Se mueve con
 * una transición de 100 ms: los datos llegan a 10–20 por segundo y así se
 * ve fluido. No recibe clics (`pointer-events-none`).
 */
export function FantasmasVivos({ disp }: { disp: Disposicion }) {
	const { t } = useLang();
	const activo = useUi((s) => s.mesa.movimientoEnVivo);
	const viendo = useMesa(tableroDe);
	const fantasmas = useVivo(useShallow((s) => Object.entries(s.fantasmas)));
	const soltadas = useVivo(
		useShallow((s) => Object.entries(s.soltadas).filter(([, x]) => x.donde?.tablero === viendo)),
	);
	const senales = useVivo((s) => s.senales);
	const jugadores = useMesa((s) => s.jugadores);
	const cartas = useMesa((s) => s.cartas);
	const reducir = useUi((s) => s.mesa.reducirAnimaciones);

	if (!activo) return null;
	const transicion = reducir ? undefined : "transform 100ms linear";
	return (
		<div className="pointer-events-none absolute inset-0 overflow-hidden" style={{ zIndex: 8500 }}>
			{senales.map((s) => {
				const c = s.carta === null ? undefined : cartas[s.carta];
				if (
					!c?.pos ||
					c.zona.tipo !== "de" ||
					c.zona.zona !== "campo" ||
					c.zona.jugador !== viendo
				) {
					return null;
				}
				const p = posAPx(disp.campo, c.pos);
				const color = jugadores[s.jugador]?.color ?? "#e11d48";
				return (
					<div
						key={s.id}
						className="absolute animate-ping rounded-xl border-4"
						style={{
							left: p.x - disp.carta.w * 0.7,
							top: p.y - disp.carta.h * 0.6,
							width: disp.carta.w * 1.4,
							height: disp.carta.h * 1.2,
							borderColor: color,
						}}
					/>
				);
			})}
			{soltadas.map(([id, x]) => (
				<CartaSoltada
					key={id}
					id={Number(id)}
					s={x}
					campo={disp.campo}
					w={disp.carta.w}
					h={disp.carta.h}
				/>
			))}
			{fantasmas.map(([clave, f]) => {
				const jugador = Number(clave);
				if (f.tablero !== viendo) return null;
				const p = posAPx(disp.campo, f);
				const color = jugadores[jugador]?.color ?? "#e11d48";
				const nombre = jugadores[jugador]?.nombre ?? t("Jugador");
				return (
					<div
						key={clave}
						className="absolute top-0 left-0 will-change-transform"
						style={{ transform: `translate(${p.x}px, ${p.y}px)`, transition: transicion }}
					>
						{cartasDeFantasma(f).map((id, i) => (
							<CartaEnVuelo
								key={id}
								id={id}
								w={disp.carta.w}
								h={disp.carta.h}
								// En pila: la primera, abajo del puntero.
								dx={i * 12}
								dy={i * 6}
							/>
						))}
						<span
							className="absolute size-3 -translate-x-1/2 -translate-y-1/2 rounded-full border border-white shadow"
							style={{ background: color }}
						/>
						<span
							className="absolute left-2 top-2 rounded px-1 text-[10px] whitespace-nowrap text-white"
							style={{ background: color }}
						>
							{nombre}
						</span>
					</div>
				);
			})}
		</div>
	);
}

/** Una carta que otro lleva en la mano, centrada en su puntero (su dorso si no es visible para mí). */
function CartaEnVuelo({
	id,
	w,
	h,
	dx,
	dy,
	ancho = 244,
}: {
	id: number;
	w: number;
	h: number;
	dx: number;
	dy: number;
	ancho?: AnchoMesa;
}) {
	const carta = useMesa((s) => s.cartas[id]);
	return (
		<div
			className="absolute opacity-80 shadow-xl"
			style={{
				width: w,
				height: h,
				left: -w / 2 + dx,
				top: -h / 2 + dy,
				transform: carta?.girada ? "rotate(90deg)" : undefined,
			}}
		>
			{carta ? <ImagenCarta imagen={caraVisible(carta)} ancho={ancho} /> : <DorsoGenerico />}
		</div>
	);
}

/**
 * **Lo que mueven los demás, sobre la miniatura** de un tablero: sus cursores
 * y las cartas que arrastran por él (en pequeño), igual que en el grande.
 * `campo` es el rectángulo del campo dentro de la miniatura.
 */
export function FantasmasMini({
	jugador,
	campo,
	ancho,
	alto,
}: {
	jugador: number;
	campo: Rect;
	ancho: number;
	alto: number;
}) {
	const activo = useUi((s) => s.mesa.movimientoEnVivo);
	const fantasmas = useVivo(
		useShallow((s) => Object.entries(s.fantasmas).filter(([, f]) => f.tablero === jugador)),
	);
	const soltadas = useVivo(
		useShallow((s) => Object.entries(s.soltadas).filter(([, x]) => x.donde?.tablero === jugador)),
	);
	const jugadores = useMesa((s) => s.jugadores);
	const reducir = useUi((s) => s.mesa.reducirAnimaciones);
	if (!activo || (fantasmas.length === 0 && soltadas.length === 0)) return null;
	const transicion = reducir ? undefined : "transform 100ms linear";
	return (
		<div className="pointer-events-none absolute inset-0" style={{ zIndex: 8500 }}>
			{soltadas.map(([id, x]) => (
				<CartaSoltada key={id} id={Number(id)} s={x} campo={campo} w={ancho} h={alto} ancho={146} />
			))}
			{fantasmas.map(([clave, f]) => {
				const p = posAPx(campo, f);
				const color = jugadores[Number(clave)]?.color ?? "#e11d48";
				return (
					<div
						key={clave}
						className="absolute top-0 left-0 will-change-transform"
						style={{ transform: `translate(${p.x}px, ${p.y}px)`, transition: transicion }}
					>
						{cartasDeFantasma(f).map((id, i) => (
							<CartaEnVuelo key={id} id={id} w={ancho} h={alto} dx={i * 3} dy={i * 2} ancho={146} />
						))}
						<span
							className="absolute size-2 -translate-x-1/2 -translate-y-1/2 rounded-full border border-white shadow"
							style={{ background: color }}
						/>
					</div>
				);
			})}
		</div>
	);
}

/**
 * Una carta que otro acaba de soltar: se sigue pintando donde la soltó hasta que
 * llega su movimiento (su sitio de antes no se ve), para que no desaparezca un
 * momento ni se deslice desde allí.
 */
function CartaSoltada({
	id,
	s,
	campo,
	w,
	h,
	ancho,
}: {
	id: number;
	s: Soltada;
	campo: Rect;
	w: number;
	h: number;
	ancho?: AnchoMesa;
}) {
	const carta = useMesa((m) => m.cartas[id]);
	if (!carta || !s.donde || !soltadaSigue(s, carta)) return null;
	const p = posAPx(campo, s.donde);
	return (
		<div className="absolute top-0 left-0" style={{ transform: `translate(${p.x}px, ${p.y}px)` }}>
			<CartaEnVuelo id={id} w={w} h={h} dx={0} dy={0} ancho={ancho} />
		</div>
	);
}
