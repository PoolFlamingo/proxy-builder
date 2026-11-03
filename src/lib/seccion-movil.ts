/**
 * Los diálogos con secciones (Ajustes, Estadísticas) en móvil son una lista que
 * entra en cada sección. El «atrás» de Android, estando la sección abierta,
 * tiene que volver a la lista y no cerrar el diálogo.
 *
 * 🛑 Por qué no es un `registrarAtras` (`lib/atras.ts`): con un diálogo abierto,
 * `atras()` cierra la capa de Radix de arriba mandando un `Escape` y nunca
 * llega a los manejadores registrados. El `Escape` sí pasa por el
 * `onEscapeKeyDown` del `DialogContent`, que es donde se intercepta.
 */
export function volverConEscape(
	enSeccion: boolean,
	volver: () => void,
): (e: { preventDefault: () => void }) => void {
	return (e) => {
		if (!enSeccion) return;
		e.preventDefault();
		volver();
	};
}
