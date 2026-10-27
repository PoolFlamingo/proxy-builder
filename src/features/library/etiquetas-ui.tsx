import { type ReactNode, useState } from "react";
import { toast } from "sonner";
import type { Tag } from "@/bindings/Tag";
import { Confirmar } from "@/components/confirmar";
import { useLang } from "@/components/language-provider";
import { type EditandoEtiqueta, EtiquetaDialog } from "./etiqueta-dialog";
import { ORGANIZADOR_BIBLIOTECA, ORGANIZADOR_JUGAR } from "./preferencias";
import { useBiblioteca } from "./store";

/**
 * El crear, editar y borrar etiquetas de la barra lateral y del menú de los
 * decks: el estado de sus diálogos y los diálogos (`dialogos`, que la pantalla
 * pone en su árbol).
 */
export function useEtiquetasUi(): {
	editar: (e: EditandoEtiqueta) => void;
	borrar: (t: Tag) => void;
	dialogos: ReactNode;
} {
	const { t, mensaje } = useLang();
	const [editando, setEditando] = useState<EditandoEtiqueta | null>(null);
	const [aBorrar, setABorrar] = useState<Tag | null>(null);
	const dialogos = (
		<>
			<EtiquetaDialog editando={editando} onCerrar={() => setEditando(null)} />
			<Confirmar
				abierto={aBorrar !== null}
				onCerrar={() => setABorrar(null)}
				titulo={t("¿Eliminar la etiqueta «{n}»?", { n: aBorrar?.name ?? "" })}
				descripcion={t("Se quita de los decks que la llevan; los decks no se borran.")}
				accion={t("Eliminar")}
				destructiva
				onConfirmar={() => {
					if (!aBorrar) return;
					const id = aBorrar.id;
					useBiblioteca
						.getState()
						.borrarEtiqueta(id)
						.then(() => {
							// Fuera de los filtros de las dos pantallas, que ya no existe.
							for (const o of [ORGANIZADOR_BIBLIOTECA, ORGANIZADOR_JUGAR]) {
								const f = o.filtros.getState();
								if (f.filtros.etiquetas.includes(id))
									f.ponerFiltros({ etiquetas: f.filtros.etiquetas.filter((x) => x !== id) });
							}
						})
						.catch((e) => toast.error(mensaje(e)));
				}}
			/>
		</>
	);
	return { editar: setEditando, borrar: setABorrar, dialogos };
}
