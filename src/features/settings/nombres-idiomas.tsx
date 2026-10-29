import { useEffect, useState } from "react";
import { toast } from "sonner";
import type { NombresIdiomasInfo } from "@/bindings/NombresIdiomasInfo";
import { Confirmar } from "@/components/confirmar";
import { Icono } from "@/components/icono";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import { Progress } from "@/components/ui/progress";
import { call, on } from "@/lib/ipc";

/**
 * **Nombres en otros idiomas** (descarga opcional): el índice de MTGJSON con
 * el nombre impreso de cada carta en cada idioma, para encontrar sin red las
 * cartas escritas en español (u otro idioma) que Scryfall no tiene impresas
 * así. Sin él se pregunta en línea a magicthegathering.io, que es lento.
 */
export function NombresIdiomas() {
	const { t, mensaje } = useLang();
	const [info, setInfo] = useState<NombresIdiomasInfo | null>(null);
	const [progreso, setProgreso] = useState<number | null>(null);
	const [leyendo, setLeyendo] = useState(false);

	const cargar = () =>
		call("names_index_info", {})
			.then(setInfo)
			.catch(() => setInfo(null));
	useEffect(() => {
		void cargar();
	}, []);

	const descargar = async () => {
		setProgreso(0);
		setLeyendo(false);
		const dejar = await on("names://progress", (p) => {
			if (p.fase === "leyendo") setLeyendo(true);
			else if (p.total > 0) setProgreso(Math.round((p.hecho / p.total) * 100));
		});
		try {
			setInfo(await call("names_index_download", {}));
		} catch (e) {
			toast.error(mensaje(e));
		} finally {
			dejar();
			setProgreso(null);
			setLeyendo(false);
		}
	};

	const [confirmarQuitar, setConfirmarQuitar] = useState(false);
	const quitar = async () => {
		try {
			await call("names_index_remove", {});
		} catch (e) {
			toast.error(mensaje(e));
		}
		await cargar();
	};

	const mb = Math.round((info?.tamano ?? 0) / (1024 * 1024));
	return (
		<div className="flex flex-col gap-2 rounded-lg border p-2.5">
			<div className="flex items-center justify-between gap-2.5">
				<span className="flex items-center gap-2 text-xs font-medium">
					<Icono icon="lucide:globe" className="size-3.5" />
					{t("Nombres en otros idiomas")}
				</span>
				<div className="flex items-center gap-1.5">
					{info?.descargado && progreso === null && (
						<Button size="xs" variant="ghost" onClick={() => setConfirmarQuitar(true)}>
							<Icono icon="lucide:trash-2" />
							{t("Quitar")}
						</Button>
					)}
					<Button size="xs" variant="outline" onClick={descargar} disabled={progreso !== null}>
						{progreso !== null ? (
							<Icono icon="lucide:loader-2" className="animate-spin" />
						) : (
							<Icono icon="lucide:download" />
						)}
						{info?.descargado ? t("Actualizar") : t("Descargar (unos {mb} MB)", { mb })}
					</Button>
				</div>
			</div>
			<p className="text-xs text-muted-foreground">
				{t(
					"Para encontrar las cartas escritas por su nombre en español u otro idioma que Scryfall no tiene impresas así (las antiguas, sobre todo), en la lista y al vincular. Sin descargarlo se pregunta en línea, más despacio.",
				)}
			</p>
			{progreso !== null && (
				<div className="flex items-center gap-2 text-[11px] text-muted-foreground">
					<Progress value={leyendo ? 100 : progreso} className="h-1.5 flex-1" />
					<span>{leyendo ? t("Preparando el índice…") : `${progreso} %`}</span>
				</div>
			)}
			{info?.descargado && progreso === null && (
				<p className="text-[11px] text-muted-foreground">
					{t("{n} nombres descargados.", { n: info.nombres })}
				</p>
			)}
			<Confirmar
				abierto={confirmarQuitar}
				onCerrar={() => setConfirmarQuitar(false)}
				titulo={t("¿Quitar los nombres en otros idiomas?")}
				descripcion={t("Para volver a tenerlos hay que descargarlos otra vez ({mb} MB).", { mb })}
				accion={t("Quitar")}
				destructiva
				onConfirmar={() => void quitar()}
			/>
		</div>
	);
}
