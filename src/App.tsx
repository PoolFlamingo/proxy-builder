import { paraTraducir } from "@/components/language-provider";
import { AppShell } from "@/features/shell/app-shell";
import { PanelVacio } from "@/features/shell/panel-vacio";
import { useDeckInicial } from "@/features/shell/use-deck-inicial";

export default function App() {
	useDeckInicial();
	return (
		<AppShell
			izquierda={<PanelVacio titulo={paraTraducir("Lista de cartas")} />}
			centro={<PanelVacio titulo={paraTraducir("Vista previa")} />}
			derecha={<PanelVacio titulo={paraTraducir("Exportar")} />}
			biblioteca={<PanelVacio titulo={paraTraducir("Biblioteca")} />}
		/>
	);
}
