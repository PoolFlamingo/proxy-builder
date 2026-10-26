import type { AjustesCara } from "@/bindings/AjustesCara";
import type { CardDesign } from "@/bindings/CardDesign";

/**
 * **Estilo y ajustes propios de cada cara** (cartas de dos caras impresas).
 * La carta guarda lo suyo (estilo, variante, marco, emblema y opciones) y una
 * cara puede llevar `faces[i].ajustes`: con ellos, esa cara se pinta con lo
 * suyo y no con lo de la carta. Las pestañas Carta, Marco y Ajustes finos
 * editan «un diseño»: para la cara con ajustes propios se les da el diseño
 * proyectado (`proyectar`) y lo que cambian vuelve a sus ajustes
 * (`cambiarEnCara`), así no saben nada de caras.
 */

type Cambiar = (f: (d: CardDesign) => CardDesign) => void;

/** ¿Esta cara lleva ajustes propios? */
export const tieneAjustes = (d: CardDesign, cara: number): boolean =>
	d.faces[cara]?.ajustes !== undefined;

/** Lo de la carta que una cara puede llevar por su cuenta. */
export function ajustesDeLaCarta(d: CardDesign): AjustesCara {
	return {
		style: d.style,
		estiloPack: d.estiloPack,
		variantePack: d.variantePack,
		frame: d.frame,
		emblem: d.emblem,
		options: d.options,
	};
}

/** El diseño tal como lo ve la cara: con sus ajustes en el sitio de los de la carta. */
export function proyectar(d: CardDesign, cara: number): CardDesign {
	const a = d.faces[cara]?.ajustes;
	return a ? { ...d, ...a } : d;
}

/**
 * Un `cambiar` para el diseño proyectado de `cara`: lo que `f` cambia de
 * estilo, marco, emblema y opciones va a los ajustes de la cara y lo demás
 * (la colección, el estado…) a la carta. Sin ajustes propios, tal cual.
 */
export function cambiarEnCara(cambiar: Cambiar, cara: number): Cambiar {
	return (f) =>
		cambiar((d) => {
			if (!tieneAjustes(d, cara)) return f(d);
			const nuevo = f(proyectar(d, cara));
			const ajustes = ajustesDeLaCarta(nuevo);
			return {
				...nuevo,
				...ajustesDeLaCarta(d),
				faces: nuevo.faces.map((x, i) => (i === cara ? { ...x, ajustes } : x)),
			};
		});
}

/** Da a la cara ajustes propios (de partida, los de la carta) o se los quita. */
export function conAjustesPropios(d: CardDesign, cara: number, propios: boolean): CardDesign {
	return {
		...d,
		faces: d.faces.map((f, i) =>
			i === cara ? { ...f, ajustes: propios ? ajustesDeLaCarta(d) : undefined } : f,
		),
	};
}
