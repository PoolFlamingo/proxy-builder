import { useState } from "react";
import { toast } from "sonner";
import { Confirmar } from "@/components/confirmar";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import { abrirParaImportar, guardarParaExportar } from "@/lib/dialogos";
import { esAppError, mensajeDeError } from "@/lib/errores";
import { call } from "@/lib/ipc";

/**
 * **Ajustes > Mesa > Identidad en red**: exportar la identidad de la
 * instalación a un fichero (para jugar con ella desde otro equipo o para
 * tener copia) e importar la de otro. Todo lo hace Rust (`red_identidad_*`).
 * Importar pregunta antes y no deja con una partida o una sala abiertas.
 */
export function IdentidadRed() {
	const { t } = useLang();
	const [ruta, setRuta] = useState<string | null>(null);
	const filtros = () => [{ name: t("Identidad en red"), extensions: ["json"] }];

	const exportar = async () => {
		const destino = await guardarParaExportar(
			{ filters: filtros() },
			"identidad-proxy-builder.json",
		);
		if (!destino) return;
		try {
			await call("red_identidad_exportar", { ruta: destino });
			toast.success(t("Identidad exportada."));
		} catch (e) {
			toast.error(mensajeDeError(e, t));
		}
	};

	const elegir = async () => {
		const r = await abrirParaImportar({ multiple: false, filters: filtros() });
		if (r?.[0]) setRuta(r[0]);
	};

	const importar = async () => {
		if (!ruta) return;
		try {
			await call("red_identidad_importar", { ruta });
			toast.success(t("Identidad cambiada."));
			setRuta(null);
		} catch (e) {
			setRuta(null);
			toast.error(
				esAppError(e) && e.code === "Busy"
					? t("Sal de la partida o de la sala antes de cambiar la identidad.")
					: mensajeDeError(e, t),
			);
		}
	};

	return (
		<>
			<h4 className="text-[11px] font-semibold text-muted-foreground uppercase">
				{t("Identidad en red")}
			</h4>
			<p className="text-xs text-muted-foreground">
				{t(
					"Tu identidad es lo que te reconoce en las partidas y los playgroups. Guárdala para jugar con ella desde otro equipo o para recuperarla. Quien tenga el fichero puede jugar como tú: no lo compartas.",
				)}
			</p>
			<div className="flex flex-wrap gap-2.5">
				<Button size="sm" variant="outline" onClick={() => void exportar()}>
					{t("Exportar identidad…")}
				</Button>
				<Button size="sm" variant="outline" onClick={() => void elegir()}>
					{t("Importar identidad…")}
				</Button>
			</div>
			<Confirmar
				abierto={ruta !== null}
				onCerrar={() => setRuta(null)}
				titulo={t("¿Cambiar la identidad?")}
				descripcion={t(
					"Este equipo pasará a ser el de la identidad del fichero. La de ahora se queda como copia en la carpeta de datos de la app.",
				)}
				accion={t("Cambiar")}
				destructiva
				onConfirmar={() => void importar()}
			/>
		</>
	);
}
