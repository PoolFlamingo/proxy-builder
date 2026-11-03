import type { EstadoConexion } from "@/stores/red-store";

/**
 * Cómo se enseña la conexión de la partida en red. Una conexión «perdida» justo
 * después de volver de segundo plano (`volviendoHasta`) todavía no es un error:
 * la app está reconectando, así que se enseña como «volviendo».
 */
export type AspectoConexion = EstadoConexion | "volviendo";

export function aspectoDeConexion(
	conexion: EstadoConexion,
	volviendoHasta: number,
	ahora: number,
): AspectoConexion {
	return conexion === "perdida" && ahora < volviendoHasta ? "volviendo" : conexion;
}
