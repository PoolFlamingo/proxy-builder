import type { IdCarta } from "@/bindings/IdCarta";
import type { VistaCarta } from "@/bindings/VistaCarta";

/**
 * **La mano entera del móvil**, agrupada por tipo o por coste. Solo reparte
 * las cartas que ya tiene la mano (el orden dentro de cada grupo es el de la
 * mano); lo que se puede hacer con ellas sigue siendo del motor.
 */

export type AgruparMano = "tipo" | "coste";

/** Los grupos por tipo, en el orden en que se enseñan. */
export const TIPOS = [
	"tierra",
	"criatura",
	"planeswalker",
	"instantaneo",
	"conjuro",
	"artefacto",
	"encantamiento",
	"batalla",
	"otro",
] as const;
export type TipoMano = (typeof TIPOS)[number];

/** Desde este coste de maná, todas van al mismo grupo («7 o más»). */
export const COSTE_MAXIMO = 7;

type CartaMano = Pick<VistaCarta, "id" | "origen">;

/** El grupo por tipo de una carta: el primero que cuadra (una criatura artefacto es criatura). */
export function tipoDeMano(c: CartaMano): TipoMano {
	const i = c.origen?.info;
	if (!i) return "otro";
	// De una carta de dos caras cuenta la de delante.
	const t = (i.tipo ?? "").toLowerCase().split(" // ")[0] ?? "";
	if (i.esTierra) return "tierra";
	if (i.esCriatura) return "criatura";
	if (t.includes("planeswalker")) return "planeswalker";
	if (t.includes("instant")) return "instantaneo";
	if (t.includes("sorcery")) return "conjuro";
	if (t.includes("artifact")) return "artefacto";
	if (t.includes("enchantment")) return "encantamiento";
	if (t.includes("battle")) return "batalla";
	return "otro";
}

/** El coste de maná convertido de una carta, tope en `COSTE_MAXIMO`; las tierras, aparte (`null`). */
export function costeDeMano(c: CartaMano): number | null {
	const i = c.origen?.info;
	if (!i || i.esTierra) return null;
	const cmc = Math.round((i.cmcDecimas ?? 0) / 10);
	return Math.min(COSTE_MAXIMO, Math.max(0, cmc));
}

export type GrupoMano =
	| { por: "tipo"; clave: TipoMano; ids: IdCarta[] }
	| { por: "coste"; clave: number | "tierras"; ids: IdCarta[] };

/** Los grupos de la mano (sin los vacíos), en orden. */
export function agruparMano(cartas: CartaMano[], por: AgruparMano): GrupoMano[] {
	if (por === "tipo") {
		const grupos = new Map<TipoMano, IdCarta[]>();
		for (const c of cartas) {
			const t = tipoDeMano(c);
			grupos.set(t, [...(grupos.get(t) ?? []), c.id]);
		}
		return TIPOS.flatMap((clave) => {
			const ids = grupos.get(clave);
			return ids ? [{ por: "tipo" as const, clave, ids }] : [];
		});
	}
	const tierras: IdCarta[] = [];
	const costes = new Map<number, IdCarta[]>();
	for (const c of cartas) {
		const coste = costeDeMano(c);
		if (coste === null) tierras.push(c.id);
		else costes.set(coste, [...(costes.get(coste) ?? []), c.id]);
	}
	const grupos: GrupoMano[] = [...costes.entries()]
		.sort((a, b) => a[0] - b[0])
		.map(([clave, ids]) => ({ por: "coste" as const, clave, ids }));
	return tierras.length > 0
		? [{ por: "coste", clave: "tierras", ids: tierras }, ...grupos]
		: grupos;
}
