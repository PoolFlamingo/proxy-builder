import { DialogoCompartir } from "./compartir-dialog";
import { DialogoImportar } from "./importar-dialog";
import { DialogoPasar } from "./pasar-dialog";

/** Los diálogos de pasar decks, montados una vez en el marco de la app. */
export function DialogosPasar() {
	return (
		<>
			<DialogoPasar />
			<DialogoCompartir />
			<DialogoImportar />
		</>
	);
}
