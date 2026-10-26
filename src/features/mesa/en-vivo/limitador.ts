/**
 * **Un límite de ritmo** para lo que se manda en vivo: deja pasar una
 * llamada como mucho cada `cadaMs`. `fuerza` pasa siempre (el último punto
 * de un arrastre no debe perderse) y cuenta como la última.
 */
export function crearLimitador(cadaMs: number, reloj: () => number = () => performance.now()) {
	let ultima = Number.NEGATIVE_INFINITY;
	return (fuerza = false): boolean => {
		const ahora = reloj();
		if (!fuerza && ahora - ultima < cadaMs) return false;
		ultima = ahora;
		return true;
	};
}
