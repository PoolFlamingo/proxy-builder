import { getVersion } from "@tauri-apps/api/app";
import { Fragment, type ReactNode, useEffect, useState } from "react";
import { DialogoAdaptable } from "@/components/dialogo-adaptable";
import { Icono } from "@/components/icono";
import { useLang } from "@/components/language-provider";
import { BarraSeccion, type EntradaSeccion, ListaSecciones } from "@/components/lista-secciones";
import { SelectorIdioma } from "@/components/selector-idioma";
import {
	Dialog,
	DialogCuerpo,
	DialogDescription,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { useFormato } from "@/lib/formato";
import { volverConEscape } from "@/lib/seccion-movil";
import { Actualizaciones, ReiniciarParaActualizar } from "./actualizaciones";
import { Almacenamiento } from "./almacenamiento";
import { Appearance } from "./appearance";
import { AjustesMarcos } from "./marcos";
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
	const movil = useFormato().tamano === "movil";
	// En móvil, una lista que entra en cada sección (`null` = la lista).
	const [seccion, setSeccion] = useState<string | null>(null);
	useEffect(() => {
		if (!open) setSeccion(null);
	}, [open]);

	const secciones: (EntradaSeccion & { cuerpo: ReactNode })[] = [
		{
			id: "idioma",
			titulo: t("Idioma"),
			icono: "lucide:languages",
			cuerpo: (
				<div className="flex items-center justify-between gap-2.5 rounded-lg border p-2.5">
					<span className="flex items-center gap-2 text-xs font-medium">
						<Icono icon="lucide:languages" className="size-3.5" />
						{t("Idioma")}
					</span>
					<SelectorIdioma valor={lang} onCambio={setLang} ariaLabel={t("Idioma")} />
				</div>
			),
		},
		{
			id: "actualizaciones",
			titulo: t("Actualizaciones"),
			icono: "lucide:refresh-cw",
			cuerpo: <Actualizaciones />,
		},
		{
			id: "nombres",
			titulo: t("Nombres en otros idiomas"),
			icono: "lucide:globe",
			cuerpo: <NombresIdiomas />,
		},
		{
			id: "aspecto",
			titulo: t("Apariencia"),
			icono: "lucide:palette",
			cuerpo: <Appearance />,
		},
		{
			id: "mesa",
			titulo: t("Mesa de juego"),
			icono: "lucide:swords",
			cuerpo: <AjustesMesa fija={movil} />,
		},
		{
			id: "marcos",
			titulo: t("Packs de marcos"),
			icono: "lucide:frame",
			cuerpo: <AjustesMarcos />,
		},
		{
			id: "almacenamiento",
			titulo: t("Almacenamiento"),
			icono: "lucide:hard-drive",
			cuerpo: <Almacenamiento />,
		},
	];
	const abierta = movil ? secciones.find((s) => s.id === seccion) : undefined;

	return (
		<Dialog open={open} onOpenChange={onOpenChange}>
			<DialogoAdaptable
				className="sm:max-w-3xl"
				cambiosSinGuardar={false}
				onEscapeKeyDown={volverConEscape(abierta !== undefined, () => setSeccion(null))}
			>
				<DialogHeader>
					<DialogTitle>{t("Ajustes")}</DialogTitle>
					<DialogDescription className="flex flex-wrap items-center justify-between gap-2.5">
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
					{!movil ? (
						<div className="flex flex-col gap-4">
							{secciones.map((s) => (
								<Fragment key={s.id}>{s.cuerpo}</Fragment>
							))}
						</div>
					) : abierta ? (
						<div className="flex flex-col gap-2.5">
							<BarraSeccion titulo={abierta.titulo} onVolver={() => setSeccion(null)} />
							{abierta.cuerpo}
						</div>
					) : (
						<ListaSecciones secciones={secciones} onElegir={setSeccion} />
					)}
				</DialogCuerpo>
			</DialogoAdaptable>
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
