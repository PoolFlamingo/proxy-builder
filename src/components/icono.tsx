import type { SVGProps } from "react";
import { INCLUIDOS } from "@/lib/iconos/incluidos";
import { useIconos } from "@/stores/iconos-store";

/** Lo que mide un icono sin clase de tamaño (lo de lucide). */
const MEDIDA = 24;

export type IconoProps = Omit<SVGProps<SVGSVGElement>, "children"> & {
	/** Un id de Iconify (`lucide:crown`, `game-icons:crossed-swords`). Vacío: nada. */
	icon: string | null | undefined;
	size?: number | string;
};

/**
 * **Un icono de cualquier colección de Iconify.** Los que la app escribe tal
 * cual van dentro (`lib/iconos/incluidos.ts`, `pnpm iconos`) y se pintan al
 * momento; los que elige la persona (`SelectorIcono`) los trae Rust la primera
 * vez (`iconos_datos`) y se quedan en `app_data_dir/iconos/`. Mientras llega
 * uno, se guarda su hueco para que nada salte.
 */
export function Icono({ icon, size, width, height, ...resto }: IconoProps) {
	const id = icon?.trim() ?? "";
	const incluido = id ? INCLUIDOS[id] : undefined;
	const bajado = useIconos((s) => (id && !incluido ? s.datos[id] : undefined));
	if (!id) return null;
	const datos = incluido ?? bajado;
	if (!datos && !incluido) useIconos.getState().pedir(id);
	return (
		<svg
			xmlns="http://www.w3.org/2000/svg"
			width={width ?? size ?? MEDIDA}
			height={height ?? size ?? MEDIDA}
			viewBox={datos?.vb ?? "0 0 24 24"}
			aria-hidden="true"
			data-icono={id}
			// Datos de @iconify/json o de la API de Iconify pasados por Rust: nunca
			// texto de la persona.
			dangerouslySetInnerHTML={datos ? { __html: datos.body } : undefined}
			{...resto}
		/>
	);
}
