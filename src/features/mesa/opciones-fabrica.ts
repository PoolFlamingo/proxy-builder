import type { Formato } from "@/bindings/Formato";
import type { ModoEquipos } from "@/bindings/ModoEquipos";
import type { Opciones } from "@/bindings/Opciones";

/** Los valores de fábrica de un formato (lo mismo que `Opciones::de_fabrica` de Rust). */
export function opcionesDeFabrica(
	formato: Formato,
	jugadores: number,
	equipos?: ModoEquipos,
): Opciones {
	return {
		vidaInicial: equipos?.tipo === "dosCabezas" ? 30 : formato === "commander" ? 40 : 20,
		manoInicial: 7,
		mulligan: formato === "commander" ? "londresPrimeroGratis" : "londres",
		primeroNoRoba: jugadores <= 2,
		ayudas: {
			robarAlEmpezar: true,
			enderezarAlEmpezar: true,
			impuestoAutomatico: true,
			contadoresAlEntrar: true,
			avisarVidaCero: true,
		},
		limiteVeneno: 10,
		limiteComandante: 21,
		todosTocanTodo: false,
		campoAjenoProtegido: false,
	};
}
