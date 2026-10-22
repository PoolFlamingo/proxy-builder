import { ArrowUpCircle, Download, Loader2, RefreshCw } from "lucide-react";
import { useEffect, useId } from "react";
import { toast } from "sonner";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import { Progress } from "@/components/ui/progress";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Switch } from "@/components/ui/switch";
import { useActualizacion } from "@/stores/actualizacion-store";
import { useUi } from "@/stores/ui-store";

/**
 * **Actualizaciones**: buscar a mano, bajar, ver las notas e instalar, y el
 * interruptor de hacerlo solo (`useActualizacionAutomatica`).
 */
export function Actualizaciones() {
	const { t, mensaje } = useLang();
	const id = useId();
	const sola = useUi((s) => s.actualizarSola);
	const { soportada, actual, paso, info, progreso, error } = useActualizacion();

	useEffect(() => {
		void useActualizacion.getState().iniciar();
	}, []);

	const buscar = async () => {
		const a = useActualizacion.getState();
		const nueva = await a.buscar();
		// Con «solo» encendido se baja ya, como al arrancar.
		if (nueva && useUi.getState().actualizarSola) await a.bajar();
	};
	const instalar = () => {
		toast.dismiss("actualizacion");
		useActualizacion
			.getState()
			.instalar()
			.catch((e) => toast.error(mensaje(e)));
	};

	const ocupada = paso === "buscando" || paso === "bajando" || paso === "instalando";
	return (
		<div className="flex flex-col gap-2 rounded-lg border p-2.5">
			<div className="flex items-center justify-between gap-2.5">
				<span className="flex items-center gap-2 text-xs font-medium">
					<RefreshCw className="size-3.5" />
					{t("Actualizaciones")}
				</span>
				{soportada && (
					<Button size="xs" variant="outline" onClick={buscar} disabled={ocupada}>
						{paso === "buscando" ? <Loader2 className="animate-spin" /> : <RefreshCw />}
						{t("Buscar ahora")}
					</Button>
				)}
			</div>
			{soportada === false ? (
				<p className="text-xs text-muted-foreground">
					{t(
						"Solo en la app instalada: esta copia no viene de un instalador (por ejemplo, en desarrollo).",
					)}
				</p>
			) : (
				<>
					<div className="flex items-center justify-between gap-2.5">
						<label htmlFor={id} className="text-xs text-muted-foreground">
							{t(
								"Buscar versiones nuevas al abrir la app y bajarlas sin molestar. Se instalan cuando reinicies.",
							)}
						</label>
						<Switch
							id={id}
							checked={sola}
							onCheckedChange={(v) => useUi.getState().setPreferencia("actualizarSola", v)}
						/>
					</div>
					<Estado />
					{paso === "disponible" && info && (
						<Button
							size="xs"
							variant="outline"
							className="self-start"
							onClick={() => void useActualizacion.getState().bajar()}
						>
							<Download />
							{t("Descargar la {version}", { version: info.version })}
						</Button>
					)}
					{(paso === "lista" || paso === "instalando") && info && (
						<Button
							size="xs"
							className="self-start"
							onClick={instalar}
							disabled={paso === "instalando"}
						>
							{paso === "instalando" ? <Loader2 className="animate-spin" /> : <ArrowUpCircle />}
							{t("Reiniciar y actualizar")}
						</Button>
					)}
					{progreso !== null && paso === "bajando" && (
						<div className="flex items-center gap-2 text-[11px] text-muted-foreground">
							<Progress value={progreso} className="h-1.5 flex-1" />
							<span>{`${progreso} %`}</span>
						</div>
					)}
					{error !== null && <p className="text-[11px] text-destructive">{mensaje(error)}</p>}
					{info?.notas && paso !== "al-dia" && (
						<ScrollArea className="rounded-md border" viewportClassName="max-h-40">
							<p className="p-2.5 font-mono text-[11px] break-words whitespace-pre-wrap text-muted-foreground">
								{info.notas}
							</p>
						</ScrollArea>
					)}
				</>
			)}
			{actual && paso === "al-dia" && (
				<p className="text-[11px] text-muted-foreground">
					{t("Tienes la última versión ({version}).", { version: actual })}
				</p>
			)}
		</div>
	);
}

/** Una línea con lo que está pasando. */
function Estado() {
	const { t } = useLang();
	const paso = useActualizacion((s) => s.paso);
	const info = useActualizacion((s) => s.info);
	if (!info) return null;
	const texto =
		paso === "disponible"
			? t("Hay una versión nueva: {version}.", { version: info.version })
			: paso === "bajando"
				? t("Descargando la {version}…", { version: info.version })
				: paso === "lista"
					? info.pidePermiso
						? t("La {version} está lista. Al reiniciar, el sistema pedirá tu contraseña.", {
								version: info.version,
							})
						: t("La {version} está lista. Se instala al reiniciar.", { version: info.version })
					: paso === "instalando"
						? t("Instalando la {version}…", { version: info.version })
						: null;
	return texto && <p className="text-xs font-medium">{texto}</p>;
}

/**
 * En la cabecera de Ajustes, a mano, mientras haya una versión bajada:
 * reiniciar la app con ella.
 */
export function ReiniciarParaActualizar() {
	const { t, mensaje } = useLang();
	const paso = useActualizacion((s) => s.paso);
	const info = useActualizacion((s) => s.info);
	if ((paso !== "lista" && paso !== "instalando") || !info) return null;
	return (
		<Button
			size="xs"
			onClick={() => {
				toast.dismiss("actualizacion");
				useActualizacion
					.getState()
					.instalar()
					.catch((e) => toast.error(mensaje(e)));
			}}
			disabled={paso === "instalando"}
		>
			{paso === "instalando" ? <Loader2 className="animate-spin" /> : <ArrowUpCircle />}
			{t("Reiniciar y actualizar a la {version}", { version: info.version })}
		</Button>
	);
}
