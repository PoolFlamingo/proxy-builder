/** Los colores que se entienden al buscar («green beast», «3/3 beast roja»). */
const COLORES: Record<string, string> = {
	white: "w",
	blanca: "w",
	blanco: "w",
	blue: "u",
	azul: "u",
	black: "b",
	negra: "b",
	negro: "b",
	red: "r",
	roja: "r",
	rojo: "r",
	green: "g",
	verde: "g",
	colorless: "c",
	incolora: "c",
	incoloro: "c",
};

/**
 * La consulta de Scryfall para lo que se escribe al buscar una ficha. El
 * nombre de una ficha no lleva su color ni su fuerza («Beast», no «Green
 * Beast»), así que esas palabras se convierten en filtros: «green beast» →
 * `beast c:g`, «3/3 beast» → `beast pow=3 tou=3`. Si lo escrito ya es sintaxis
 * de Scryfall (`t:goblin`, comillas, operadores), se deja tal cual.
 */
export function consultaDeFicha(texto: string): string {
	const limpio = texto.trim().replace(/\s+/g, " ");
	if (/[:"()<>=!]|\b(or|and)\b/i.test(limpio)) return limpio;
	const nombre: string[] = [];
	let colores = "";
	const filtros: string[] = [];
	for (const palabra of limpio.split(" ")) {
		const color = COLORES[palabra.toLowerCase()];
		const pt = /^(\d+|\*)\/(\d+|\*)$/.exec(palabra);
		if (color) colores += color;
		else if (pt) {
			if (pt[1] !== "*") filtros.push(`pow=${pt[1]}`);
			if (pt[2] !== "*") filtros.push(`tou=${pt[2]}`);
		} else nombre.push(palabra);
	}
	// Solo color o solo fuerza: sin nombre no hay qué buscar por nombre.
	return [...nombre, ...(colores ? [`c:${colores}`] : []), ...filtros].join(" ");
}
