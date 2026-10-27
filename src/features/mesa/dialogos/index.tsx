import { useMesa } from "@/stores/mesa-store";
import { usePantalla } from "../pantalla";
import { TapeteDialog } from "../tapete-dialog";
import { AzarDialog } from "./azar";
import { BuscarDialog, VerZonaDialog } from "./buscar";
import { DanoComandanteDialog } from "./dano-comandante";
import { FichasDialog } from "./fichas";
import { MirarDialog } from "./mirar";
import {
	AtajosDialog,
	ContadorDialog,
	EtiquetaDialog,
	InspeccionDialog,
	NumeroDialog,
	PtDialog,
	TextoDialog,
} from "./pequenos";
import { RegistroDialog } from "./registro";
import { RevelarDialog } from "./revelar";
import { TerminarDialog } from "./terminar";

/** El diálogo de la mesa que esté abierto (uno a la vez) y el de inspeccionar. */
export function DialogosMesa({ deckId }: { deckId: string | null }) {
	const d = usePantalla((s) => s.dialogo);
	const hayFin = useMesa((s) => s.general?.fin != null);
	return (
		<>
			<InspeccionDialog />
			{d?.tipo === "numero" && <NumeroDialog d={d} />}
			{d?.tipo === "texto" && <TextoDialog d={d} />}
			{d?.tipo === "contador" && <ContadorDialog cartas={d.cartas} />}
			{d?.tipo === "pt" && <PtDialog carta={d.carta} />}
			{d?.tipo === "etiqueta" && <EtiquetaDialog carta={d.carta} />}
			{d?.tipo === "mirar" && <MirarDialog d={d} />}
			{d?.tipo === "buscar" && <BuscarDialog de={d.de} filtro={d.filtro} />}
			{d?.tipo === "revelar" && <RevelarDialog />}
			{d?.tipo === "zona" && <VerZonaDialog jugador={d.jugador} zona={d.zona} />}
			{d?.tipo === "fichas" && <FichasDialog />}
			{d?.tipo === "azar" && <AzarDialog />}
			{d?.tipo === "terminar" && !hayFin && <TerminarDialog />}
			{d?.tipo === "danoComandante" && <DanoComandanteDialog />}
			{d?.tipo === "atajos" && <AtajosDialog />}
			{d?.tipo === "registro" && <RegistroDialog />}
			<TapeteDialog
				abierto={d?.tipo === "tapete"}
				onCerrar={() => usePantalla.getState().abrir(null)}
				deckId={deckId}
			/>
		</>
	);
}
