import { CardEditorDialog } from "@/features/card-editor/card-editor-dialog";
import { DecklistPanel } from "@/features/decklist/decklist-panel";
import { FichasDialog } from "@/features/decklist/fichas-dialog";
import { ColeccionesDialog } from "@/features/disenar/colecciones-dialog";
import { DisenarScreen } from "@/features/disenar/disenar-screen";
import { LoteDialog } from "@/features/disenar/lote-dialog";
import { PngDialog } from "@/features/disenar/png-dialog";
import { RecursosDialog } from "@/features/disenar/recursos-dialog";
import { ExportPanel } from "@/features/export/export-panel";
import { LibraryScreen } from "@/features/library/library-screen";
import { MesaScreen } from "@/features/mesa/mesa-screen";
import { MoverADialog } from "@/features/preview/mover-a-dialog";
import { PreviewPane } from "@/features/preview/preview-pane";
import { SetBrowserDialog } from "@/features/set-browser/set-browser-dialog";
import { SincronizarTemas } from "@/features/settings/temas-tweakcn";
import {
	useActualizacionAutomatica,
	useAvisosActualizacion,
} from "@/features/shell/actualizacion-automatica";
import { AppShell } from "@/features/shell/app-shell";
import { useDeckInicial } from "@/features/shell/use-deck-inicial";
import { StatsDialog } from "@/features/stats/stats-dialog";
import { AddCardDialog } from "@/features/variants/add-card-dialog";
import { VariantDialog } from "@/features/variants/variant-dialog";
import { VincularDialog } from "@/features/variants/vincular-dialog";
import { VincularLoteDialog } from "@/features/variants/vincular-lote";

export default function App() {
	useDeckInicial();
	useActualizacionAutomatica();
	useAvisosActualizacion();
	return (
		<>
			<AppShell
				izquierda={<DecklistPanel />}
				centro={<PreviewPane />}
				derecha={<ExportPanel />}
				biblioteca={<LibraryScreen />}
				disenar={<DisenarScreen />}
				mesa={<MesaScreen />}
			/>
			<AddCardDialog />
			<VariantDialog />
			<VincularDialog />
			<VincularLoteDialog />
			<CardEditorDialog />
			<MoverADialog />
			<FichasDialog />
			<SincronizarTemas />
			<StatsDialog />
			<SetBrowserDialog />
			<ColeccionesDialog />
			<RecursosDialog />
			<LoteDialog />
			<PngDialog />
		</>
	);
}
