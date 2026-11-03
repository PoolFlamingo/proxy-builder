/**
 * Abre el menú de una casilla con un toque. El `ContextMenu` de Radix solo se
 * abre con el clic derecho o con 700 ms de pulsación larga; para «un toque en la
 * casilla ya seleccionada» se le manda lo mismo que le llegaría con el clic
 * derecho, en el punto tocado.
 */
export function abrirMenuContextual(el: Element, x: number, y: number) {
	el.dispatchEvent(
		new MouseEvent("contextmenu", {
			bubbles: true,
			cancelable: true,
			clientX: x,
			clientY: y,
			button: 2,
		}),
	);
}

/**
 * ¿Un clic de táctil debe abrir el menú? Solo si la casilla ya estaba
 * seleccionada, no es un hueco, ningún modificador (no es una selección
 * múltiple de teclado) y no es el segundo toque de un doble toque (que abre el
 * editor de la carta).
 */
export function tocarAbreElMenu(o: {
	tactil: boolean;
	estabaSeleccionada: boolean;
	hueco: boolean;
	modificador: boolean;
	toques: number;
}): boolean {
	return o.tactil && o.estabaSeleccionada && !o.hueco && !o.modificador && o.toques < 2;
}
