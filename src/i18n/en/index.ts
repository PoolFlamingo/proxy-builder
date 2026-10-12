// El diccionario de inglés, fundido a partir de sus áreas: un fichero por zona
// de la app. 🛑 Una misma cadena fuente NO puede estar en dos áreas (al fundir
// gana la última, en silencio); `pnpm idiomas` lo comprueba.

import { AJUSTES } from "./ajustes";
import { COMUN } from "./comun";
import { SHELL } from "./shell";

export const EN: Record<string, string> = {
	...COMUN,
	...AJUSTES,
	...SHELL,
};
