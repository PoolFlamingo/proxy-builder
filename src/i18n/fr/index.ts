// El diccionario de francés, fundido a partir de sus áreas: un fichero por zona
// de la app. 🛑 Una misma cadena fuente NO puede estar en dos áreas (al fundir
// gana la última, en silencio); `pnpm idiomas` lo comprueba.

import type { Diccionario } from "../tipos";
import { AJUSTES } from "./ajustes";
import { BIBLIOTECA } from "./biblioteca";
import { CARTAS } from "./cartas";
import { COMUN } from "./comun";
import { DISENAR } from "./disenar";
import { ESTADISTICAS } from "./estadisticas";
import { EXPORTAR } from "./exportar";
import { EXTRAS } from "./extras";
import { LISTA } from "./lista";
import { SHELL } from "./shell";
import { VINCULAR } from "./vincular";
import { VISTA } from "./vista";

export const DICCIONARIO: Diccionario = {
	...COMUN,
	...AJUSTES,
	...SHELL,
	...LISTA,
	...VISTA,
	...EXPORTAR,
	...BIBLIOTECA,
	...CARTAS,
	...EXTRAS,
	...VINCULAR,
	...ESTADISTICAS,
	...DISENAR,
};
