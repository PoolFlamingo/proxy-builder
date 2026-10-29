import { useEffect, useState } from "react";
import { toast } from "sonner";
import type { CacheStats } from "@/bindings/CacheStats";
import { Confirmar } from "@/components/confirmar";
import { Icono } from "@/components/icono";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import { megas } from "@/features/export/acciones";
import { call } from "@/lib/ipc";

/**
 * **Almacenamiento**, en Ajustes: lo que ocupa la caché de miniaturas (se
 * vuelve a generar sola) y liberar las imágenes que ya no usa ningún deck.
 * Son las mismas acciones del panel de la lista y de la Biblioteca, juntas
 * donde se buscan.
 */
export function Almacenamiento() {
	const { t, mensaje } = useLang();
	const [cache, setCache] = useState<CacheStats | null>(null);
	const [confirmar, setConfirmar] = useState(false);
	const [ocupado, setOcupado] = useState(false);
	const leer = () =>
		call("cache_stats", {})
			.then(setCache)
			.catch(() => setCache(null));
	useEffect(() => {
		void leer();
	}, []);

	const liberar = async () => {
		setOcupado(true);
		try {
			const r = await call("blob_gc", {});
			toast.success(
				r.removed > 0
					? t("{n} imágenes sin usar borradas ({mb})", { n: r.removed, mb: megas(r.freedBytes) })
					: t("No había imágenes sin usar (se guardan 24 h por si acaso)."),
			);
		} catch (e) {
			toast.error(mensaje(e));
		} finally {
			setOcupado(false);
		}
	};

	return (
		<div className="flex flex-col gap-2.5 rounded-lg border p-2.5">
			<span className="flex items-center gap-2 text-xs font-medium">
				<Icono icon="lucide:hard-drive" className="size-3.5" />
				{t("Almacenamiento")}
			</span>
			<div className="flex flex-wrap items-center justify-between gap-2.5 text-xs">
				<span className="text-muted-foreground">
					{cache
						? t("Caché de miniaturas: {mb}", { mb: megas(cache.bytes) })
						: t("Caché de miniaturas")}
				</span>
				<Button size="xs" variant="outline" onClick={() => setConfirmar(true)}>
					<Icono icon="lucide:trash-2" />
					{t("Limpiar caché de miniaturas")}
				</Button>
			</div>
			<div className="flex flex-wrap items-center justify-between gap-2.5 text-xs">
				<span className="text-muted-foreground">{t("Imágenes que ya no usa ningún deck.")}</span>
				<Button size="xs" variant="outline" disabled={ocupado} onClick={() => void liberar()}>
					<Icono
						icon={ocupado ? "lucide:loader-2" : "lucide:hard-drive-download"}
						className={ocupado ? "animate-spin" : undefined}
					/>
					{t("Liberar espacio")}
				</Button>
			</div>
			<Confirmar
				abierto={confirmar}
				onCerrar={() => setConfirmar(false)}
				titulo={t("¿Limpiar la caché de miniaturas?")}
				descripcion={t(
					"Ocupa {mb} en {n} ficheros. Se vuelven a generar al verlas. Las imágenes de tus decks no se tocan.",
					{ mb: megas(cache?.bytes ?? 0), n: cache?.files ?? 0 },
				)}
				accion={t("Limpiar")}
				onConfirmar={() => {
					call("cache_clear", {})
						.then((r) => {
							toast.success(t("Caché limpiada ({mb})", { mb: megas(r.bytes) }));
							void leer();
						})
						.catch((e) => toast.error(mensaje(e)));
				}}
			/>
		</div>
	);
}
