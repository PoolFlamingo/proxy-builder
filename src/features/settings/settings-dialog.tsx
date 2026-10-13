import { getVersion } from "@tauri-apps/api/app";
import { Languages } from "lucide-react";
import { useEffect, useState } from "react";
import { useLang } from "@/components/language-provider";
import {
	Dialog,
	DialogContent,
	DialogCuerpo,
	DialogDescription,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { Appearance } from "./appearance";

/** Ajustes de la app: idioma y apariencia (el sistema de temas de judlex). */
export function SettingsDialog({
	open,
	onOpenChange,
}: {
	open: boolean;
	onOpenChange: (abierto: boolean) => void;
}) {
	const { t, lang, setLang } = useLang();
	const version = useVersion();
	return (
		<Dialog open={open} onOpenChange={onOpenChange}>
			<DialogContent className="sm:max-w-3xl" cambiosSinGuardar={false}>
				<DialogHeader>
					<DialogTitle>{t("Ajustes")}</DialogTitle>
					<DialogDescription className="flex items-center justify-between gap-2.5">
						<span>{t("Idioma y aspecto de la aplicación.")}</span>
						{version && (
							<span className="rounded-md border px-2 py-0.5 font-mono text-[11px] text-muted-foreground">
								Proxy Builder {version}
							</span>
						)}
					</DialogDescription>
				</DialogHeader>
				<DialogCuerpo>
					<div className="flex flex-col gap-4">
						<div className="flex items-center justify-between gap-2.5 rounded-lg border p-2.5">
							<span className="flex items-center gap-2 text-xs font-medium">
								<Languages className="size-3.5" />
								{t("Idioma")}
							</span>
							<ToggleGroup
								type="single"
								variant="outline"
								size="sm"
								value={lang}
								onValueChange={(v) => (v === "es" || v === "en") && setLang(v)}
							>
								<ToggleGroupItem value="es" className="px-3 text-xs">
									Español
								</ToggleGroupItem>
								<ToggleGroupItem value="en" className="px-3 text-xs">
									English
								</ToggleGroupItem>
							</ToggleGroup>
						</div>
						<Appearance />
					</div>
				</DialogCuerpo>
			</DialogContent>
		</Dialog>
	);
}

/** La versión de la app (la de `tauri.conf.json`), leída a Tauri una vez. */
function useVersion(): string | null {
	const [version, setVersion] = useState<string | null>(null);
	useEffect(() => {
		getVersion()
			.then((v) => setVersion(`v${v}`))
			.catch(() => setVersion(null));
	}, []);
	return version;
}
