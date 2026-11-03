import { useCallback, useEffect, useState } from "react";
import type { PackInfo } from "@/bindings/PackInfo";
import { Icono } from "@/components/icono";
import { useLang } from "@/components/language-provider";
import { ID_ORIGINAL } from "@/features/disenar/marcos-originales";
import { FilaOriginales, FilasDescargables } from "@/features/disenar/recursos-dialog";
import { call } from "@/lib/ipc";

/**
 * **Packs de marcos**, en Ajustes: los que se descargan (los «Original» y los
 * del repositorio de packs, como Balatro) con su botón, y el contenido extra de
 * los ya bajados. Es la parte de «Letras, marcos y símbolos» de Diseñar que el
 * móvil no tiene (ese diálogo es de tableta y ordenador): así hay un sitio fijo
 * donde bajarlos en cualquier formato.
 */
export function AjustesMarcos() {
	const { t } = useLang();
	const [packs, setPacks] = useState<PackInfo[] | null>(null);
	const recargar = useCallback(() => {
		call("packs_list", {})
			.then(setPacks)
			.catch(() => setPacks([]));
	}, []);
	useEffect(() => recargar(), [recargar]);
	if (packs === null)
		return (
			<Icono icon="lucide:loader-2" className="m-3 size-4 animate-spin text-muted-foreground" />
		);
	const hayOriginal = packs.some((p) => p.id === ID_ORIGINAL);
	return (
		<ul className="flex flex-col divide-y rounded-lg border">
			{hayOriginal ? (
				<li className="flex items-center gap-2.5 px-3 py-2">
					<div className="min-w-0 flex-1 truncate text-xs font-medium">
						{t("Original (Card Conjurer)")}
					</div>
					<span className="flex items-center gap-1 text-[10px] text-muted-foreground">
						<Icono icon="lucide:check" className="size-3" />
						{t("Descargado")}
					</span>
				</li>
			) : (
				<FilaOriginales alTerminar={recargar} />
			)}
			<FilasDescargables alTerminar={recargar} />
		</ul>
	);
}
