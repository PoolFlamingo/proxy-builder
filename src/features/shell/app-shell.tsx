import { BookOpen, Layers, Settings } from "lucide-react";
import { type ReactNode, useState } from "react";
import { useDefaultLayout } from "react-resizable-panels";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from "@/components/ui/resizable";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { SettingsDialog } from "@/features/settings/settings-dialog";
import { SaveIndicator } from "./save-indicator";

export type Vista = "deck" | "biblioteca";

/**
 * El marco de la app, como fabricard: barra superior y, debajo, tres paneles
 * redimensionables — la lista de cartas a la izquierda, las páginas en el
 * centro y los ajustes de exportación a la derecha. Los anchos se recuerdan.
 */
export function AppShell({
	izquierda,
	centro,
	derecha,
	biblioteca,
}: {
	izquierda: ReactNode;
	centro: ReactNode;
	derecha: ReactNode;
	biblioteca: ReactNode;
}) {
	const { t } = useLang();
	const [vista, setVista] = useState<Vista>("deck");
	const [ajustes, setAjustes] = useState(false);
	const layout = useDefaultLayout({ id: "proxybuilder.paneles", storage: localStorage });

	return (
		<div className="flex h-svh flex-col bg-background text-foreground">
			<header className="flex h-12 shrink-0 items-center gap-2.5 border-b px-3">
				<div className="flex items-center gap-2">
					<Layers className="size-5 text-primary" />
					<div className="leading-tight">
						<div className="text-sm font-semibold">Proxy Builder</div>
						<div className="text-[10px] text-muted-foreground">
							{t("Scryfall · listo para imprimir")}
						</div>
					</div>
				</div>
				<ToggleGroup
					type="single"
					size="sm"
					data-pestanas=""
					className="ml-4"
					value={vista}
					onValueChange={(v) => (v === "deck" || v === "biblioteca") && setVista(v)}
				>
					<ToggleGroupItem value="deck" className="gap-1.5 px-3 text-xs">
						<Layers className="size-3.5" />
						{t("Deck")}
					</ToggleGroupItem>
					<ToggleGroupItem value="biblioteca" className="gap-1.5 px-3 text-xs">
						<BookOpen className="size-3.5" />
						{t("Biblioteca")}
					</ToggleGroupItem>
				</ToggleGroup>
				<div className="ml-auto flex items-center gap-2.5">
					<SaveIndicator />
					<Button
						size="sm"
						variant="ghost"
						onClick={() => setAjustes(true)}
						aria-label={t("Ajustes")}
					>
						<Settings className="size-4" />
						{t("Ajustes")}
					</Button>
				</div>
			</header>

			<main className="min-h-0 flex-1">
				{vista === "deck" ? (
					<ResizablePanelGroup
						orientation="horizontal"
						defaultLayout={layout.defaultLayout}
						onLayoutChanged={layout.onLayoutChanged}
					>
						<ResizablePanel id="izquierda" defaultSize="22" minSize={260}>
							{izquierda}
						</ResizablePanel>
						<ResizableHandle withHandle />
						<ResizablePanel id="centro" minSize={360}>
							{centro}
						</ResizablePanel>
						<ResizableHandle withHandle />
						<ResizablePanel id="derecha" defaultSize="22" minSize={260}>
							{derecha}
						</ResizablePanel>
					</ResizablePanelGroup>
				) : (
					biblioteca
				)}
			</main>

			<SettingsDialog open={ajustes} onOpenChange={setAjustes} />
		</div>
	);
}
