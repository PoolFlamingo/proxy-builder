import { estadoDeLosDialogos } from "@/lib/dialogos-abiertos";
import { useActualizacion } from "@/stores/actualizacion-store";
import { useEdicion } from "@/stores/edicion-store";
import { useMesa } from "@/stores/mesa-store";
import { useTrabajo } from "@/stores/trabajo-store";

/**
 * **Qué impide cerrar la app sin preguntar.** Cada campo es una razón; el
 * diálogo de cierre cuenta solo las que se cumplen.
 */
export type Pendientes = {
	/** La fase del trabajo largo en marcha («Preparando imágenes…»), si lo hay. */
	trabajo: string | null;
	/** Se está bajando o instalando una actualización. */
	actualizando: boolean;
	/** Un diálogo abierto tiene un proceso en curso. */
	dialogoEnCurso: boolean;
	/** Un diálogo abierto tiene algo escrito sin guardar. */
	dialogoSinGuardar: boolean;
	/** La carta que se diseña tiene cambios sin guardar. */
	edicion: boolean;
	/** El deck no se ha podido guardar (se perderían sus últimos cambios). */
	deck: boolean;
	/** Hay una partida abierta en la mesa (se guarda y se puede seguir). */
	partida: boolean;
};

/**
 * Lo que hay pendiente ahora. `deckSinGuardar` lo dice quien intentó guardar
 * el deck antes de cerrar (`guardarAhora`): si no pudo, es una razón más.
 */
export function pendientes(deckSinGuardar = false): Pendientes {
	const dialogos = estadoDeLosDialogos();
	const paso = useActualizacion.getState().paso;
	return {
		trabajo: useTrabajo.getState().actual?.fase ?? null,
		actualizando: paso === "bajando" || paso === "instalando",
		dialogoEnCurso: dialogos.enCurso,
		dialogoSinGuardar: dialogos.sinGuardar,
		edicion: useEdicion.getState().pendiente !== null,
		deck: deckSinGuardar,
		partida: useMesa.getState().activa,
	};
}

/** ¿Hay alguna razón para preguntar? */
export function hayPendientes(p: Pendientes): boolean {
	return (
		p.trabajo !== null ||
		p.actualizando ||
		p.dialogoEnCurso ||
		p.dialogoSinGuardar ||
		p.edicion ||
		p.deck ||
		p.partida
	);
}
