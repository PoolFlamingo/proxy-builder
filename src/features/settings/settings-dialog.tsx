import { getVersion } from "@tauri-apps/api/app";
import { useEffect, useState } from "react";
import { Icono } from "@/components/icono";
import { useLang } from "@/components/language-provider";
import { SelectorIdioma } from "@/components/selector-idioma";
import {
	Dialog,
	DialogContent,
	DialogCuerpo,
	DialogDescription,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { Actualizaciones, ReiniciarParaActualizar } from "./actualizaciones";
import { Almacenamiento } from "./almacenamiento";
import { Appearance } from "./appearance";
import { AjustesMesa } from "./mesa";
import { NombresIdiomas } from "./nombres-idiomas";

/** Ajustes de la app: idioma, actualizaciones, nombres en otros idiomas y apariencia. */
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
						<span>{t("Idioma, aspecto, mesa de juego, actualizaciones y almacenamiento.")}</span>
						<span className="flex items-center gap-2">
							<ReiniciarParaActualizar />
							{version && (
								<span className="rounded-md border px-2 py-0.5 font-mono text-[11px] text-muted-foreground">
									Proxy Builder {version}
								</span>
							)}
						</span>
					</DialogDescription>
				</DialogHeader>
				<DialogCuerpo>
					<div className="flex flex-col gap-4">
						<div className="flex items-center justify-between gap-2.5 rounded-lg border p-2.5">
							<span className="flex items-center gap-2 text-xs font-medium">
								<Icono icon="lucide:languages" className="size-3.5" />
								{t("Idioma")}
							</span>
							<SelectorIdioma valor={lang} onCambio={setLang} ariaLabel={t("Idioma")} />
						</div>
						<Actualizaciones />
						<NombresIdiomas />
						<Appearance />
						<AjustesMesa />
						<Almacenamiento />
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
