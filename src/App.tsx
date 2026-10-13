import { CardEditorDialog } from "@/features/card-editor/card-editor-dialog";
import { DecklistPanel } from "@/features/decklist/decklist-panel";
import { ExportPanel } from "@/features/export/export-panel";
import { LibraryScreen } from "@/features/library/library-screen";
import { MoverADialog } from "@/features/preview/mover-a-dialog";
import { PreviewPane } from "@/features/preview/preview-pane";
import { SetBrowserDialog } from "@/features/set-browser/set-browser-dialog";
import { AppShell } from "@/features/shell/app-shell";
import { useDeckInicial } from "@/features/shell/use-deck-inicial";
import { StatsDialog } from "@/features/stats/stats-dialog";
import { AddCardDialog } from "@/features/variants/add-card-dialog";
import { VariantDialog } from "@/features/variants/variant-dialog";

export default function App() {
	useDeckInicial();
	return (
		<>
			<AppShell
				izquierda={<DecklistPanel />}
				centro={<PreviewPane />}
				derecha={<ExportPanel />}
				biblioteca={<LibraryScreen />}
			/>
			<AddCardDialog />
			<VariantDialog />
			<CardEditorDialog />
			<MoverADialog />
			<StatsDialog />
			<SetBrowserDialog />
		</>
	);
}
