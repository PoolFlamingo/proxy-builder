import { useFormato } from "@/lib/formato";
import { useMesa } from "@/stores/mesa-store";
import { useUi, type Vista } from "@/stores/ui-store";

/**
 * **La mesa a pantalla completa en el móvil.** Con una partida abierta en la
 * pestaña «Jugar», el shell esconde la barra de arriba y la de pestañas de
 * abajo: la mesa ocupa toda la pantalla (en horizontal). `MesaScreen` y la
 * barra de la mesa leen esto para saber que tienen que enseñar su propio
 * botón de salir.
 */
export function useMesaAPantallaCompleta(): boolean {
	const { tamano } = useFormato();
	const vista = useUi((s) => s.vista);
	const activa = useMesa((s) => s.activa);
	return tamano === "movil" && vista === "mesa" && activa;
}

let anterior: Vista = "deck";
let enlazado = false;

/** Apunta la pantalla en la que se estaba antes de entrar en «Jugar». */
function recordarAnterior() {
	if (enlazado) return;
	enlazado = true;
	useUi.subscribe((s, antes) => {
		if (s.vista !== antes.vista && antes.vista !== "mesa") anterior = antes.vista;
	});
}
recordarAnterior();

/**
 * Sale de la pestaña «Jugar» a la pantalla de la que se vino (Deck si no
 * se sabe). La partida **sigue abierta y guardada**: es como cambiar de
 * pestaña; para cerrarla está el botón de la barra de la mesa.
 */
export function salirDeLaMesa() {
	useUi.getState().setVista(anterior === "mesa" ? "deck" : anterior);
}
