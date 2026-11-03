/**
 * La zona que se marca al recortar la ilustración de una carta, con el dedo:
 * se agarra una esquina para cambiarla de tamaño, el interior para moverla o
 * el exterior para marcar otra desde cero. Todo en fracciones de la carta
 * (0–1); los radios, en px, porque un dedo agarra más ancho que un ratón.
 */
export type Rect = { x: number; y: number; w: number; h: number };
export type Punto = { x: number; y: number };
export type Agarre = "nuevo" | "mover" | "nw" | "ne" | "sw" | "se";

const ESQUINAS = ["nw", "ne", "sw", "se"] as const;
type Esquina = (typeof ESQUINAS)[number];
const CONTRARIA: Record<Esquina, Esquina> = { nw: "se", ne: "sw", sw: "ne", se: "nw" };

const acotar = (v: number) => Math.min(1, Math.max(0, v));

/** Las esquinas de una zona. */
export function esquina(z: Rect, cual: Esquina): Punto {
	return {
		x: cual === "nw" || cual === "sw" ? z.x : z.x + z.w,
		y: cual === "nw" || cual === "ne" ? z.y : z.y + z.h,
	};
}

/**
 * Qué agarra un toque en `p`: la esquina más cercana si cae a menos de
 * `radio` px, si no el interior de la zona (mover) o el exterior (otra nueva).
 * `marco` es el tamaño de la carta en px.
 */
export function agarreEn(
	zona: Rect,
	p: Punto,
	marco: { w: number; h: number },
	radio: number,
): Agarre {
	let mejor: Agarre | null = null;
	let distancia = radio;
	for (const e of ESQUINAS) {
		const c = esquina(zona, e);
		const d = Math.hypot((p.x - c.x) * marco.w, (p.y - c.y) * marco.h);
		if (d <= distancia) {
			distancia = d;
			mejor = e;
		}
	}
	if (mejor) return mejor;
	const dentro = p.x >= zona.x && p.x <= zona.x + zona.w && p.y >= zona.y && p.y <= zona.y + zona.h;
	return dentro && zona.w > 0 && zona.h > 0 ? "mover" : "nuevo";
}

const entre = (a: Punto, b: Punto): Rect => ({
	x: Math.min(a.x, b.x),
	y: Math.min(a.y, b.y),
	w: Math.abs(a.x - b.x),
	h: Math.abs(a.y - b.y),
});

/**
 * La zona tras arrastrar desde `inicio` hasta `p`, partiendo de `base` (la que
 * había al tocar). Mover la deja dentro de la carta sin cambiarle el tamaño;
 * cambiar una esquina fija la contraria (y se puede pasar al otro lado).
 */
export function arrastrar(agarre: Agarre, base: Rect, inicio: Punto, p: Punto): Rect {
	if (agarre === "nuevo") return entre({ x: acotar(inicio.x), y: acotar(inicio.y) }, p);
	if (agarre === "mover") {
		return {
			...base,
			x: Math.min(1 - base.w, Math.max(0, base.x + p.x - inicio.x)),
			y: Math.min(1 - base.h, Math.max(0, base.y + p.y - inicio.y)),
		};
	}
	const movida = esquina(base, agarre);
	const opuesta = esquina(base, CONTRARIA[agarre]);
	return entre(opuesta, {
		x: acotar(movida.x + p.x - inicio.x),
		y: acotar(movida.y + p.y - inicio.y),
	});
}
