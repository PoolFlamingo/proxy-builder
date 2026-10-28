import { Swords, Target } from "lucide-react";
import { useMemo } from "react";
import { useLang } from "@/components/language-provider";
import { NINGUNA, tableroDe, useMesa } from "@/stores/mesa-store";
import { duenoDeExtremo, idDePunta } from "./cuentas-flechas";
import { type Disposicion, posAPx } from "./disposicion";

/** Un color de reserva para una flecha de alguien que ya no está. */
const COLOR_FLECHA = "#e11d48";

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
	const yo = useMesa(tableroDe);
	const varios = useMesa((s) => s.asientos.length > 1);
	const colores = useMemo(
		() => [...new Set(flechas.map((f) => jugadores[f.deJugador]?.color ?? COLOR_FLECHA))],
		[flechas, jugadores],
	);
	if (flechas.length === 0) return null;
	const punto = (e: (typeof flechas)[number]["de"]): { x: number; y: number } | null => {
		if (e.tipo === "carta") {
			const c = cartas[e.id];
			if (!c?.pos) return null;
			// Una carta de la mesa de otro (en una partida local se ve en pequeño): la
			// flecha apunta hacia allí, arriba en el centro, como a un jugador.
			return c.controlador === yo ? posAPx(disp.campo, c.pos) : { x: disp.campo.w / 2, y: 30 };
		}
		// Un jugador: arriba en el centro (el rival imaginado) o mi vida.
		return { x: disp.campo.w / 2, y: 30 };
	};
	const dibujadas = flechas.flatMap((f, i) => {
		const a = punto(f.de);
		const b = punto(f.a);
		if (!a || !b) return [];
		const color = jugadores[f.deJugador]?.color ?? COLOR_FLECHA;
		return [{ f, i, a, b, color }];
	});
	return (
		<>
			<svg className="pointer-events-none absolute inset-0 size-full" style={{ zIndex: 6000 }}>
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
