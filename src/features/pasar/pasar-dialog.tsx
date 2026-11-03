import { DialogoAdaptable } from "@/components/dialogo-adaptable";
import { useLang } from "@/components/language-provider";
import { Dialog, DialogDescription, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { PanelEnviar } from "./enviar";
import { PanelRecibir } from "./recibir";
import { usePasarUi } from "./store";

/**
 * **Enviar / Recibir**: pasar decks a otro equipo (el móvil, el ordenador de un
 * amigo) sin cuenta y sin servidor. La pantalla solo pinta lo que Rust cuenta
 * (`pasar/`); cerrarla cancela lo que estuviera en marcha.
 */
export function DialogoPasar() {
	const { t } = useLang();
	const d = usePasarUi((s) => (s.dialogo?.tipo === "pasar" ? s.dialogo : null));
	const cerrar = usePasarUi((s) => s.cerrar);
	return (
		<Dialog open={d !== null} onOpenChange={(o) => !o && cerrar()}>
			{d && (
				<DialogoAdaptable className="text-xs sm:max-w-lg" cambiosSinGuardar={false}>
					<DialogHeader>
						<DialogTitle>{t("Pasar decks")}</DialogTitle>
						<DialogDescription className="text-xs">
							{t(
								"Entre tus equipos o con un amigo, sin cuenta: los datos van cifrados de punta a punta.",
							)}
						</DialogDescription>
					</DialogHeader>
					<Tabs defaultValue={d.pestana} className="flex min-h-0 flex-1 flex-col gap-3">
						<TabsList className="w-full">
							<TabsTrigger value="enviar" className="text-xs">
								{t("Enviar")}
							</TabsTrigger>
							<TabsTrigger value="recibir" className="text-xs">
								{t("Recibir")}
							</TabsTrigger>
						</TabsList>
						<TabsContent value="enviar" className="flex min-h-0 flex-1 flex-col gap-3">
							<PanelEnviar idsIniciales={d.ids} onCerrar={cerrar} />
						</TabsContent>
						<TabsContent value="recibir" className="flex min-h-0 flex-1 flex-col gap-3">
							<PanelRecibir onCerrar={cerrar} />
						</TabsContent>
					</Tabs>
				</DialogoAdaptable>
			)}
		</Dialog>
	);
}
