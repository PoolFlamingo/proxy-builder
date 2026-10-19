import { getVersion } from "@tauri-apps/api/app";
import { Brush, Languages, TriangleAlert } from "lucide-react";
import { useEffect, useId, useState } from "react";
import { useLang } from "@/components/language-provider";
import {
	Dialog,
	DialogContent,
	DialogCuerpo,
	DialogDescription,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { Switch } from "@/components/ui/switch";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { useUi } from "@/stores/ui-store";
import { Actualizaciones } from "./actualizaciones";
import { Appearance } from "./appearance";
import { NombresIdiomas } from "./nombres-idiomas";

/** Ajustes de la app: idioma, actualizaciones, editor de cartas (beta) y apariencia. */
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
						<Actualizaciones />
						<NombresIdiomas />
						<EditorBeta />
						<Appearance />
					</div>
				</DialogCuerpo>
			</DialogContent>
		</Dialog>
	);
}

/**
 * El interruptor del **editor de cartas**: apagado de fábrica porque aún es
 * beta. Encendido, aparece la pestaña «Diseñar» y las opciones de
 * diseño del menú de cada carta.
 */
function EditorBeta() {
	const { t } = useLang();
	const id = useId();
	const encendido = useUi((s) => s.editorCartas);
	return (
		<div className="flex flex-col gap-2 rounded-lg border p-2.5">
			<div className="flex items-center justify-between gap-2.5">
				<label htmlFor={id} className="flex items-center gap-2 text-xs font-medium">
					<Brush className="size-3.5" />
					{t("Editor de cartas")}
					<span className="rounded-full border border-amber-500/50 px-1.5 text-[9px] leading-4 font-medium text-amber-600 uppercase dark:text-amber-400">
						{t("Beta")}
					</span>
				</label>
				<Switch
					id={id}
					checked={encendido}
					onCheckedChange={(v) => useUi.getState().setEditorCartas(v)}
				/>
			</div>
			<p className="text-xs text-muted-foreground">
				{t(
					"Diseña tus propias versiones de las cartas: marco, textos en español o inglés y arte nuevo. Añade la pestaña «Diseñar» y las opciones de diseño al menú de cada carta.",
				)}
			</p>
			<p className="flex items-start gap-1.5 text-xs text-amber-600 dark:text-amber-400">
				<TriangleAlert className="mt-0.5 size-3.5 shrink-0" />
				{t(
					"Está en pruebas: puede tener fallos y cambiar entre versiones. Los diseños se guardan con el deck, pero revisa el PDF antes de imprimir.",
				)}
			</p>
		</div>
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
