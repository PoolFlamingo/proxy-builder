import { useEffect, useState } from "react";
import { toast } from "sonner";
import type { InfoPaquete } from "@/bindings/InfoPaquete";
import { DialogoAdaptable } from "@/components/dialogo-adaptable";
import { Icono } from "@/components/icono";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import {
	Dialog,
	DialogCuerpo,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { nombreSeguro } from "@/features/export/acciones";
import { useBiblioteca } from "@/features/library/store";
import { guardarParaExportar } from "@/lib/dialogos";
import { call } from "@/lib/ipc";
import { useEsMovil } from "@/lib/plataforma";
import { pesar, textoDeError } from "./cuentas";
import { usePasarUi } from "./store";

/**
 * **Compartir un deck**: el `.pbdeck` con o sin proxies. En el móvil se
 * ofrece a otra app («Compartir…»: Archivos, un mensaje…); en el ordenador se
 * guarda («Guardar como…»). Con varios decks elegidos solo se puede enviar a
 * otro equipo.
 */
export function DialogoCompartir() {
	const d = usePasarUi((s) => (s.dialogo?.tipo === "compartir" ? s.dialogo : null));
	const cerrar = usePasarUi((s) => s.cerrar);
	return (
		<Dialog open={d !== null} onOpenChange={(o) => !o && cerrar()}>
			{d && <Contenido key={d.ids.join(",")} ids={d.ids} onCerrar={cerrar} />}
		</Dialog>
	);
}

function Contenido({ ids, onCerrar }: { ids: string[]; onCerrar: () => void }) {
	const { t, mensaje, lang } = useLang();
	const movil = useEsMovil();
	const decks = useBiblioteca((s) => s.decks) ?? [];
	const uno = ids.length === 1 ? (decks.find((d) => d.id === ids[0]) ?? null) : null;
	const [info, setInfo] = useState<InfoPaquete | null>(null);
	const [sinProxies, setSinProxies] = useState(false);
	const [ocupado, setOcupado] = useState(false);

	useEffect(() => {
		if (!uno) return;
		let vivo = true;
		call("deck_paquete_info", { id: uno.id })
			.then((i) => vivo && setInfo(i))
			.catch(() => {});
		return () => {
			vivo = false;
		};
	}, [uno]);

	const guardar = async () => {
		if (!uno) return;
		setOcupado(true);
		try {
			const destino = await guardarParaExportar(
				{
					title: t("Guardar el deck"),
					filters: [{ name: "Proxy Builder", extensions: ["pbdeck"] }],
				},
				nombreSeguro(uno.name, "pbdeck"),
			);
			if (!destino) return;
			const bytes = await call("deck_paquete_guardar", {
				id: uno.id,
				ruta: destino,
				sinProxies,
			});
			toast.success(t("Deck guardado ({mb})", { mb: pesar(bytes, lang) }), {
				action: {
					label: t("Mostrar en carpeta"),
					onClick: () => void call("export_reveal", { path: destino }),
				},
			});
			onCerrar();
		} catch (e) {
			toast.error(textoDeError(e, t, mensaje));
		} finally {
			setOcupado(false);
		}
	};

	const compartir = async () => {
		if (!uno) return;
		setOcupado(true);
		try {
			await call("deck_paquete_compartir", { id: uno.id, sinProxies });
			onCerrar();
		} catch (e) {
			toast.error(textoDeError(e, t, mensaje));
		} finally {
			setOcupado(false);
		}
	};

	const enviar = () => {
		onCerrar();
		usePasarUi.getState().abrirPasar({ pestana: "enviar", ids });
	};

	return (
		<DialogoAdaptable className="text-xs sm:max-w-md" cambiosSinGuardar={false}>
			<DialogHeader>
				<DialogTitle>{t("Compartir")}</DialogTitle>
				<DialogDescription className="text-xs">
					{uno ? uno.name : t("{n} decks elegidos", { n: ids.length })}
				</DialogDescription>
			</DialogHeader>
			<DialogCuerpo>
				<div className="flex flex-col gap-3 p-1">
					{uno && info?.proxies && (
						<div className="flex flex-col gap-1.5">
							<ToggleGroup
								type="single"
								variant="outline"
								size="sm"
								value={sinProxies ? "sin" : "con"}
								onValueChange={(v) => v && setSinProxies(v === "sin")}
								className="w-full flex-wrap"
							>
								<ToggleGroupItem value="con" className="h-auto min-h-7 flex-1 px-2 py-1 text-xs">
									{t("Con proxies ({peso})", { peso: pesar(info.tamano, lang) })}
								</ToggleGroupItem>
								<ToggleGroupItem value="sin" className="h-auto min-h-7 flex-1 px-2 py-1 text-xs">
									{t("Sin proxies ({peso})", { peso: pesar(info.tamanoOriginales, lang) })}
								</ToggleGroupItem>
							</ToggleGroup>
							<p className="text-[11px] leading-snug text-muted-foreground">
								{sinProxies
									? t(
											"Solo van las cartas originales: quien lo abra baja las imágenes de las vinculadas.",
										)
									: t("Van tus imágenes propias y tus diseños.")}
							</p>
						</div>
					)}
					{uno && (
						<p className="text-muted-foreground">
							{movil
								? t("Se guarda en un fichero .pbdeck y eliges con qué app compartirlo.")
								: t("Se guarda en un fichero .pbdeck que puedes mandar o abrir en otro equipo.")}
						</p>
					)}
				</div>
			</DialogCuerpo>
			<DialogFooter>
				<Button size="sm" variant="outline" onClick={onCerrar}>
					{t("Cerrar")}
				</Button>
				<Button size="sm" variant="outline" onClick={enviar}>
					<Icono icon="lucide:arrow-left-right" />
					{t("Enviar a otro equipo…")}
				</Button>
				{uno &&
					(movil ? (
						<Button size="sm" onClick={() => void compartir()} disabled={ocupado}>
							<Icono icon="lucide:share-2" />
							{t("Compartir…")}
						</Button>
					) : (
						<Button size="sm" onClick={() => void guardar()} disabled={ocupado}>
							<Icono icon="lucide:save" />
							{t("Guardar como…")}
						</Button>
					))}
			</DialogFooter>
		</DialogoAdaptable>
	);
}
