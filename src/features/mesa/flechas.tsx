import { Swords, Target } from "lucide-react";
import { createContext, useContext, useLayoutEffect, useMemo, useRef, useState } from "react";
import type { IdJugador } from "@/bindings/IdJugador";
import { useLang } from "@/components/language-provider";
import { type Mesa, NINGUNA, tableroDe, useMesa } from "@/stores/mesa-store";
import { duenoDeExtremo, entradaDesde, idDePunta } from "./cuentas-flechas";
import { type Disposicion, posAPx } from "./disposicion";

/** Un color de reserva para una flecha de alguien que ya no está. */
const COLOR_FLECHA = "#e11d48";

/**
 * Dónde está la mesa de cada jugador (su centro) visto desde el tablero
 * grande, en sus coordenadas. Lo pone `mesas.tsx` con varios tableros; sin
 * él (un solo tablero), lo de fuera entra por arriba en el centro.
 */
export const MesasAlrededor = createContext<Partial<Record<IdJugador, { x: number; y: number }>>>(
	{},
);

/**
 * El jugador cuyo marcador de vida está en pantalla: yo y, si no hay «yo» (la
 * repetición con todo a la vista), el del tablero que se mira.
 */
export function duenoDeControles(m: Pick<Mesa, "yo"> & { mirando: IdJugador | null }): IdJugador {
	return m.yo ?? tableroDe(m);
}

/** Atributo del marcador de vida del que juega (`vida-jugador.tsx`): ahí apuntan las flechas que le llegan. */
const ATRIBUTO_VIDA = "data-vida-de";

/**
 * **Las flechas de ataque y objetivo** entre cartas (y hacia los jugadores),
 * en un SVG sobre el campo. Cada una va del color de quien la puso, con la
 * **punta del mismo color** (un marcador por color: `context-stroke` no lo
 * entiende todo WebKit y salía negra) y, en una partida de varios, una
 * etiqueta con **a quién va**, del color de ese jugador: así se ve claro a
 * quién ataca cada criatura aunque el objetivo esté en otra mesa.
 */
export function Flechas({ disp }: { disp: Disposicion }) {
	const { t } = useLang();
	const flechas = useMesa((s) => s.general?.flechas ?? NINGUNA);
	const cartas = useMesa((s) => s.cartas);
	const jugadores = useMesa((s) => s.jugadores);
	const viendo = useMesa(tableroDe);
	// De quién es el marcador de vida que se enseña (`Controles`): el mío o, en una
	// repetición con todo a la vista (sin «yo»), el del tablero que se mira.
	const yo = useMesa(duenoDeControles);
	const varios = useMesa((s) => s.asientos.length > 1);
	const alrededor = useContext(MesasAlrededor);
	const svg = useRef<SVGSVGElement>(null);
	// Dónde está mi marcador de vida (abajo a la derecha del tablero grande): se
	// mide, porque crece con los contadores y el botón de pasar turno.
	const [vida, setVida] = useState<{ x: number; y: number } | null>(null);
	useLayoutEffect(() => {
		const caja = svg.current?.getBoundingClientRect();
		const el = document.querySelector(`[${ATRIBUTO_VIDA}="${yo}"]`);
		if (!caja || !el) {
			setVida(null);
			return;
		}
		const r = el.getBoundingClientRect();
		// Al borde de arriba, en el centro: la punta no tapa el número.
		setVida({ x: r.left + r.width / 2 - caja.left, y: r.top - caja.top - 4 });
	}, [yo, disp, flechas]);
	const colores = useMemo(
		() => [...new Set(flechas.map((f) => jugadores[f.deJugador]?.color ?? COLOR_FLECHA))],
		[flechas, jugadores],
	);
	if (flechas.length === 0) return null;
	const desde = (j: IdJugador) => entradaDesde(disp.campo.w, disp.campo.h, alrededor[j]);
	/** El punto de un extremo y si está en este tablero (o es mi vida) o fuera. */
	const punto = (
		e: (typeof flechas)[number]["de"],
	): { x: number; y: number; aqui: boolean } | null => {
		if (e.tipo === "carta") {
			const c = cartas[e.id];
			if (!c?.pos) return null;
			// Una carta de la mesa de otro: la flecha entra por el lado de esa mesa.
			if (c.controlador === viendo) return { ...posAPx(disp.campo, c.pos), aqui: true };
			return { ...desde(c.controlador), aqui: false };
		}
		// Yo: mi marcador de vida. El de este tablero (si miro el de otro): arriba
		// en el centro. Otro: por el lado de su mesa.
		if (e.id === yo && vida) return { ...vida, aqui: true };
		if (e.id === viendo)
			return { x: disp.campo.w / 2, y: Math.min(30, disp.campo.h / 2), aqui: true };
		return { ...desde(e.id), aqui: false };
	};
	const dibujadas = flechas.flatMap((f, i) => {
		const a = punto(f.de);
		const b = punto(f.a);
		if (!a || !b) return [];
		// Entre dos mesas que no son esta: se ve en sus miniaturas, no aquí (y sería
		// una flecha de un borde a sí mismo).
		if (!a.aqui && !b.aqui) return [];
		if (Math.hypot(b.x - a.x, b.y - a.y) < 12) return [];
		const color = jugadores[f.deJugador]?.color ?? COLOR_FLECHA;
		return [{ f, i, a, b, color }];
	});
	return (
		<>
			<svg
				ref={svg}
				className="pointer-events-none absolute inset-0 size-full"
				style={{ zIndex: 6000 }}
			>
				<defs>
					{colores.map((color) => (
						<marker
							key={color}
							id={idDePunta(color)}
							viewBox="0 0 10 10"
							refX="8"
							refY="5"
							markerWidth="6"
							markerHeight="6"
							orient="auto-start-reverse"
						>
							<path d="M 0 0 L 10 5 L 0 10 z" fill={color} />
						</marker>
					))}
				</defs>
				{dibujadas.map(({ f, i, a, b, color }) => (
					<line
						key={i}
						x1={a.x}
						y1={a.y}
						x2={b.x}
						y2={b.y}
						stroke={color}
						strokeWidth={f.tipo === "ataque" ? 4 : 2.5}
						strokeDasharray={f.tipo === "objetivo" ? "8 5" : undefined}
						markerEnd={`url(#${idDePunta(color)})`}
						opacity={0.9}
					/>
				))}
			</svg>
			{varios &&
				dibujadas.map(({ f, i, a, b }) => {
					const dueno = duenoDeExtremo(f.a, cartas);
					const objetivo = dueno === null ? undefined : jugadores[dueno];
					if (!objetivo) return null;
					const carta = f.a.tipo === "carta" ? cartas[f.a.id]?.origen?.nombre : undefined;
					// Cerca de la punta, no de la mitad: ahí es donde se mira a qué llega.
					const x = a.x + (b.x - a.x) * 0.72;
					const y = a.y + (b.y - a.y) * 0.72;
					const Icono = f.tipo === "ataque" ? Swords : Target;
					return (
						<div
							key={`e${i}`}
							className="pointer-events-none absolute flex max-w-48 items-center gap-1 rounded-full border border-white/40 px-1.5 py-0.5 text-[10px] leading-none font-semibold text-white shadow-md [text-shadow:0_1px_2px_rgba(0,0,0,0.6)]"
							style={{
								left: x,
								top: y,
								transform: "translate(-50%, -50%)",
								background: objetivo.color,
								zIndex: 6001,
							}}
							role="img"
							aria-label={t("{tipo} a {nombre}", {
								tipo: f.tipo === "ataque" ? t("Ataque") : t("Objetivo"),
								nombre: objetivo.nombre,
							})}
						>
							<Icono className="size-3 shrink-0" />
							<span className="truncate">
								{carta ? `${objetivo.nombre} · ${carta}` : objetivo.nombre}
							</span>
						</div>
					);
				})}
		</>
	);
}
